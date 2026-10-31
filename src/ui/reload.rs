//! Hot reload: what a saved `config.toml` or `style.css` means for the running bar.
//!
//! The decisions are pure ([`Watched`], [`evaluate`], [`plan`]); [`Watcher`] only turns
//! inotify-backed directory monitors into one debounced callback, so an idle bar spends
//! zero wakeups on watching.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::{gio, glib};

use crate::config::{self, Diagnostic, ResolvedConfig};
use crate::modules::ModuleId;

/// Called with the files that changed in one burst.
type OnChange = Rc<dyn Fn(&[File])>;

/// Quiet time after the last event before reloading. Editors save in several steps
/// (truncate, write, rename, chmod); one reload follows the burst.
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Monitors the watched files' directories and reports which files changed, once per
/// burst of events. Dropping it stops watching.
pub struct Watcher {
    monitors: Vec<gio::FileMonitor>,
    pending: Rc<Pending>,
}

#[derive(Default)]
struct Pending {
    files: RefCell<BTreeSet<File>>,
    timer: RefCell<Option<glib::SourceId>>,
}

impl Watcher {
    pub fn start(watched: Watched, on_change: impl Fn(&[File]) + 'static) -> Self {
        let watched = Rc::new(watched);
        let on_change: OnChange = Rc::new(on_change);
        let pending = Rc::new(Pending::default());
        let mut monitors = Vec::new();
        for dir in watched.dirs() {
            // GIO polls for missing paths every few seconds; an idle bar must not.
            if !dir.is_dir() {
                log::warn!(
                    "{} does not exist; changes there apply after a restart",
                    dir.display()
                );
                continue;
            }
            let monitor = match gio::File::for_path(&dir)
                .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            {
                Ok(monitor) => monitor,
                Err(e) => {
                    log::warn!("cannot watch {} for changes: {e}", dir.display());
                    continue;
                }
            };
            let (watched, on_change) = (watched.clone(), on_change.clone());
            let weak = Rc::downgrade(&pending);
            monitor.connect_changed(move |_, file, other, event| {
                use gio::FileMonitorEvent::*;
                if !matches!(
                    event,
                    Changed | ChangesDoneHint | Created | Deleted | Renamed | MovedIn | MovedOut
                ) {
                    return;
                }
                // A rename names the old path in `file` and the new one in `other`.
                let hits: Vec<File> = [Some(file), other]
                    .into_iter()
                    .flatten()
                    .filter_map(|f| watched.kind_of(&f.path()?))
                    .collect();
                if let (false, Some(pending)) = (hits.is_empty(), weak.upgrade()) {
                    pending.add(hits, &on_change);
                }
            });
            monitors.push(monitor);
        }
        Self { monitors, pending }
    }
}

impl Pending {
    /// Record changed files and restart the quiet-time timer.
    fn add(self: &Rc<Self>, files: Vec<File>, on_change: &OnChange) {
        self.files.borrow_mut().extend(files);
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
        let weak = Rc::downgrade(self);
        let on_change = on_change.clone();
        let timer = glib::timeout_add_local_once(DEBOUNCE, move || {
            let Some(pending) = weak.upgrade() else {
                return;
            };
            // This timer has fired; forget it rather than remove it.
            pending.timer.take();
            let files: Vec<File> = std::mem::take(&mut *pending.files.borrow_mut())
                .into_iter()
                .collect();
            on_change(&files);
        });
        *self.timer.borrow_mut() = Some(timer);
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        for monitor in &self.monitors {
            monitor.cancel();
        }
        if let Some(timer) = self.pending.timer.take() {
            timer.remove();
        }
    }
}

/// A file hot reload follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum File {
    Config,
    Style,
}

/// The watched files and the directories to monitor for them.
///
/// Directories, not files: editors save by writing a temporary file and renaming it over
/// the original, which would leave a file monitor watching a deleted inode. A symlinked
/// file (dotfile managers) is also matched where its target lives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watched {
    files: Vec<(PathBuf, File)>,
}

impl Watched {
    pub fn new(config: &Path, style: &Path) -> Self {
        let mut files = Vec::new();
        for (path, file) in [(config, File::Config), (style, File::Style)] {
            let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
            if let Ok(target) = path.canonicalize()
                && target != path
            {
                files.push((target, file));
            }
            files.push((path, file));
        }
        files.sort();
        Self { files }
    }

    /// Which watched file a monitor event's path refers to, if any.
    pub fn kind_of(&self, path: &Path) -> Option<File> {
        self.files
            .iter()
            .find(|(watched, _)| watched == path)
            .map(|&(_, file)| file)
    }

    /// Every directory to monitor, each once.
    pub fn dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .files
            .iter()
            .filter_map(|(path, _)| path.parent().map(Path::to_path_buf))
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }
}

/// What a saved `config.toml` amounts to.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Resolves to the config already running (a comment or whitespace edit).
    Unchanged,
    /// A valid new config to apply.
    Changed(Box<ResolvedConfig>),
    /// The edit is invalid; the running config stays.
    Invalid(Vec<Diagnostic>),
}

/// Resolve saved config text against the config that is running.
pub fn evaluate(text: &str, running: &ResolvedConfig) -> Outcome {
    match config::resolve(text) {
        Err(diagnostics) => Outcome::Invalid(diagnostics),
        Ok(config) if config == *running => Outcome::Unchanged,
        Ok(config) => Outcome::Changed(Box::new(config)),
    }
}

/// What applying a new config takes, beyond restyling and reconfiguring sources
/// (both of which only touch what changed, so they always run).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Bars must be torn down and built again.
    pub rebuild_bars: bool,
}

/// Bars are rebuilt only when something baked into their windows changes: placement,
/// layout, the focal class, or click actions. Height, radius, motion and theme live in
/// CSS; module settings live in their sources, which views stay subscribed to.
pub fn plan(old: &ResolvedConfig, new: &ResolvedConfig) -> Plan {
    let (a, b) = (&old.bar, &new.bar);
    let window = a.position != b.position || a.margin != b.margin || a.monitors != b.monitors;
    let actions = ModuleId::ALL
        .into_iter()
        .any(|id| old.actions(id) != new.actions(id));
    Plan {
        rebuild_bars: window || a.focal != b.focal || old.layout != new.layout || actions,
    }
}
