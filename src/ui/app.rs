//! The GTK application: single instance, config, sources, styles, bars and hot reload.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use super::bar_manager::BarManager;
use super::reload::{self, File, Outcome, Watched, Watcher};
use super::sources::Sources;
use super::style::Style;
use crate::cli::Options;
use crate::config::{self, ResolvedConfig};
use crate::{APP_ID, paths};

/// Everything a running bar owns. Dropping it tears the bars down.
struct Runtime {
    app: gtk4::Application,
    display: gdk::Display,
    config_path: PathBuf,
    /// The last good config: what is running now.
    config: ResolvedConfig,
    bars: Option<BarManager>,
    style: Style,
    sources: Sources,
    _watcher: Watcher,
    _hold: gio::ApplicationHoldGuard,
}

pub fn run(options: Options) -> glib::ExitCode {
    gio::resources_register_include!("zenith-bar.gresource").expect("bundled resources are valid");

    let config_path = paths::config_file(options.config.as_deref());
    let style_path = paths::style_file(options.style.as_deref());

    // Default flags make the application unique on the session bus: a second
    // launch forwards `activate` to the running instance and exits.
    let app = gtk4::Application::builder().application_id(APP_ID).build();
    let runtime: Rc<RefCell<Option<Runtime>>> = Rc::default();

    app.connect_activate(move |app| {
        if runtime.borrow().is_some() {
            log::info!("already running");
            return;
        }
        if !gtk4_layer_shell::is_supported() {
            log::error!("this compositor does not support the layer-shell protocol");
            app.quit();
            return;
        }
        let Some(display) = gdk::Display::default() else {
            log::error!("no display");
            app.quit();
            return;
        };

        let config = load_config(&config_path);
        let sources = Sources::start(&config);
        let style = Style::install(&display, &config, &style_path);
        let bars = BarManager::start(app, &display, &config, &sources);

        let weak = Rc::downgrade(&runtime);
        let watcher = Watcher::start(Watched::new(&config_path, &style_path), move |files| {
            if let Some(runtime) = weak.upgrade()
                && let Some(runtime) = runtime.borrow_mut().as_mut()
            {
                runtime.reload(files);
            }
        });

        *runtime.borrow_mut() = Some(Runtime {
            app: app.clone(),
            display,
            config_path: config_path.clone(),
            config,
            bars: Some(bars),
            style,
            sources,
            _watcher: watcher,
            _hold: app.hold(),
        });
    });

    // Our own flags were parsed already; GTK sees only the program name.
    app.run_with_args(&["zenith-bar"])
}

impl Runtime {
    fn reload(&mut self, files: &[File]) {
        if files.contains(&File::Config) {
            self.reload_config();
        }
        if files.contains(&File::Style) {
            log::info!("style.css changed, reloading it");
            self.style.reload_user();
        }
    }

    /// Apply a saved config, or keep the running one if the edit is invalid.
    fn reload_config(&mut self) {
        let name = self.config_path.display().to_string();
        let Some(text) = read_config(&self.config_path) else {
            log::error!("{name}: keeping the running config");
            return;
        };
        let new = match reload::evaluate(&text, &self.config) {
            Outcome::Unchanged => {
                log::debug!("{name} saved without changes");
                return;
            }
            Outcome::Invalid(diagnostics) => {
                for diagnostic in diagnostics {
                    log::error!("{}", diagnostic.render(&name));
                }
                log::error!("{name} has errors; keeping the last good config");
                return;
            }
            Outcome::Changed(new) => *new,
        };

        let plan = reload::plan(&self.config, &new);
        self.sources.reconfigure(&self.config, &new);
        self.style.apply(&new);
        if plan.rebuild_bars {
            // Old bars go first, so two never claim the same exclusive zone.
            self.bars = None;
            self.bars = Some(BarManager::start(
                &self.app,
                &self.display,
                &new,
                &self.sources,
            ));
        }
        self.config = new;
        let rebuilt = if plan.rebuild_bars {
            " and rebuilt the bars"
        } else {
            ""
        };
        log::info!("reloaded {name}{rebuilt}");
    }
}

/// A missing file means defaults. An invalid file is reported and replaced by defaults,
/// so a typo never leaves you without a bar.
fn load_config(path: &Path) -> ResolvedConfig {
    let text = read_config(path).unwrap_or_else(|| {
        log::error!("{}: using defaults", path.display());
        String::new()
    });
    config::resolve(&text, None).unwrap_or_else(|diagnostics| {
        let name = path.display().to_string();
        for diagnostic in diagnostics {
            log::error!("{}", diagnostic.render(&name));
        }
        log::error!("{name} has errors; using defaults");
        config::resolve("", None).expect("defaults are valid")
    })
}

/// The config text: empty for a missing file, `None` (logged) if it cannot be read.
fn read_config(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(String::new()),
        Err(e) => {
            log::error!("cannot read {}: {e}", path.display());
            None
        }
    }
}
