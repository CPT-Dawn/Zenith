//! The one cockpit source per process: the runner fleet (a [`RunnersSource`] it owns)
//! and one loop per feed command, all on the main loop (ADR 0002).
//!
//! A feed loop runs its command with `sh -c` as an async `gio::Subprocess` in a
//! process group of its own, waits for it (killing the group after `timeout`), parses
//! stdout, publishes, sleeps `interval` and runs it again. One loop per feed, so a
//! feed never runs twice at once. A failed run (timeout, non-zero exit, bad JSON)
//! keeps the last good document and marks the feed failed.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk4::gio;
use gtk4::glib;

use super::model::{self, Chip, FeedState};
use super::{CockpitConfig, FeedConfig};
use crate::modules::runners::source::RunnersSource;
use crate::modules::{Shared, Subscription, ViewState};

pub struct CockpitSource {
    state: Rc<Shared<ViewState>>,
    runners: Rc<RunnersSource>,
    runner_state: RefCell<Option<Subscription>>,
    feeds: Rc<Shared<Vec<FeedState>>>,
    chips: Rc<Shared<Vec<Chip>>>,
    inner: RefCell<Inner>,
}

#[derive(Default)]
struct Inner {
    config: CockpitConfig,
    /// Bumped whenever the feeds change, so loops of the old set end.
    generation: u64,
    /// Commands in flight: (feed index, process group id).
    running: Vec<(usize, i32)>,
}

impl CockpitSource {
    pub fn start(config: CockpitConfig) -> Rc<Self> {
        let source = Rc::new(Self {
            state: Shared::new(ViewState::default()),
            runners: RunnersSource::start(config.runners.clone()),
            runner_state: RefCell::new(None),
            feeds: Shared::new(Vec::new()),
            chips: Shared::new(Vec::new()),
            inner: RefCell::new(Inner {
                config,
                ..Inner::default()
            }),
        });
        let weak = Rc::downgrade(&source);
        let subscription = source.runners.state().subscribe(move |runners| {
            if let Some(source) = weak.upgrade() {
                source.refresh_state(runners);
            }
        });
        *source.runner_state.borrow_mut() = Some(subscription);
        source.restart_feeds(&[]);
        source
    }

    /// The module's own state: the runner fleet's tooltip and classes while it
    /// watches any repo.
    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    /// The runner fleet, for its glance and card.
    pub fn runners(&self) -> &Rc<RunnersSource> {
        &self.runners
    }

    /// Every feed, in config order.
    pub fn feeds(&self) -> Rc<Shared<Vec<FeedState>>> {
        self.feeds.clone()
    }

    /// The bar's chips; changes only when what the bar shows changes.
    pub fn chips(&self) -> Rc<Shared<Vec<Chip>>> {
        self.chips.clone()
    }

    pub fn reconfigure(self: &Rc<Self>, config: CockpitConfig) {
        let old = std::mem::replace(&mut self.inner.borrow_mut().config, config.clone());
        if old.runners != config.runners {
            self.runners.reconfigure(config.runners.clone());
        }
        self.runners.state().with(|s| self.refresh_state(s));
        if old.feeds != config.feeds {
            self.restart_feeds(&old.feeds);
        }
    }

    fn refresh_state(&self, runners: &ViewState) {
        let watching = !self.inner.borrow().config.runners.repos.is_empty();
        self.state.set(if watching {
            ViewState {
                tooltip: runners.tooltip.clone(),
                classes: runners.classes.clone(),
                ..ViewState::default()
            }
        } else {
            ViewState::default()
        });
    }

    /// Stop the current feed loops and start one per configured feed. A feed whose
    /// command is unchanged at the same place keeps what it showed.
    fn restart_feeds(self: &Rc<Self>, old: &[FeedConfig]) {
        let (generation, feeds) = {
            let mut inner = self.inner.borrow_mut();
            for (_, group) in inner.running.drain(..) {
                kill_group(group);
            }
            inner.generation += 1;
            (inner.generation, inner.config.feeds.clone())
        };
        let previous = self.feeds.with(Vec::clone);
        let states: Vec<FeedState> = feeds
            .iter()
            .enumerate()
            .map(|(i, feed)| {
                let same = old.get(i).is_some_and(|o| o.command == feed.command);
                match previous.get(i) {
                    Some(state) if same => state.clone(),
                    _ => FeedState {
                        command: feed.command.clone(),
                        ..FeedState::default()
                    },
                }
            })
            .collect();
        self.publish_all(states);
        for index in 0..feeds.len() {
            let weak = Rc::downgrade(self);
            glib::MainContext::default().spawn_local(feed_loop(weak, generation, index));
        }
    }

    fn publish_all(&self, feeds: Vec<FeedState>) {
        self.chips.set(model::glance(&feeds));
        self.feeds.set(feeds);
    }

    fn publish(&self, index: usize, outcome: Result<model::Feed, String>) {
        let mut feeds = self.feeds.with(Vec::clone);
        let Some(state) = feeds.get_mut(index) else {
            return;
        };
        match outcome {
            Ok(feed) => {
                state.last = Some(feed);
                state.failure = None;
            }
            Err(why) => {
                log::warn!("cockpit: feed `{}` failed: {why}", state.command);
                state.failure = Some(why);
            }
        }
        self.publish_all(feeds);
    }

    /// The feed at `index` if `generation` is still current.
    fn feed(&self, generation: u64, index: usize) -> Option<FeedConfig> {
        let inner = self.inner.borrow();
        (inner.generation == generation)
            .then(|| inner.config.feeds.get(index).cloned())
            .flatten()
    }
}

impl Drop for CockpitSource {
    fn drop(&mut self) {
        if let Some(subscription) = self.runner_state.get_mut().take() {
            self.runners.state().unsubscribe(subscription);
        }
        for (_, group) in self.inner.get_mut().running.drain(..) {
            kill_group(group);
        }
    }
}

/// Run one feed again and again until the source goes away or its feeds change.
async fn feed_loop(weak: Weak<CockpitSource>, generation: u64, index: usize) {
    loop {
        let Some(feed) = weak.upgrade().and_then(|s| s.feed(generation, index)) else {
            return;
        };
        let outcome = run(&feed, |group| {
            if let Some(source) = weak.upgrade() {
                source.inner.borrow_mut().running.push((index, group));
            }
        })
        .await;
        let Some(source) = weak.upgrade() else { return };
        {
            let mut inner = source.inner.borrow_mut();
            if inner.generation != generation {
                return;
            }
            inner.running.retain(|(i, _)| *i != index);
        }
        source.publish(index, outcome.and_then(|out| model::parse(&out)));
        drop(source);
        glib::timeout_future(Duration::from_secs(feed.interval.max(5).into())).await;
    }
}

/// Run `feed.command` once; its stdout, or why it failed. `started` gets the process
/// group id as soon as the command runs.
async fn run(feed: &FeedConfig, started: impl FnOnce(i32)) -> Result<String, String> {
    let launcher = gio::SubprocessLauncher::new(
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    );
    // A group of its own, so a timeout kills the command's children too.
    launcher.set_child_setup(|| {
        // SAFETY: setpgid is async-signal-safe and touches only the child itself.
        unsafe {
            libc::setpgid(0, 0);
        }
    });
    let process = launcher
        .spawn(&["sh".as_ref(), "-c".as_ref(), feed.command.as_ref()])
        .map_err(|e| e.message().to_string())?;
    let group = process
        .identifier()
        .and_then(|pid| pid.parse::<i32>().ok())
        .unwrap_or(0);
    started(group);

    let limit = Duration::from_secs(feed.timeout.max(1).into());
    let finished = glib::future_with_timeout(limit, process.communicate_utf8_future(None)).await;
    let Ok(finished) = finished else {
        kill_group(group);
        return Err(format!("timed out after {} s", feed.timeout.max(1)));
    };
    let (stdout, stderr) = finished.map_err(|e| e.message().to_string())?;
    if process.is_successful() {
        return Ok(stdout.map(|s| s.to_string()).unwrap_or_default());
    }
    let status = if process.has_exited() {
        format!("exited with status {}", process.exit_status())
    } else {
        format!("killed by signal {}", process.term_sig())
    };
    let why = stderr
        .as_deref()
        .and_then(|s| s.lines().rev().find(|l| !l.trim().is_empty()))
        .map(|l| format!("{status}: {}", l.trim()))
        .unwrap_or(status);
    Err(why)
}

fn kill_group(group: i32) {
    if group > 0 {
        // SAFETY: plain syscall; a group that has already gone is ESRCH, ignored.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
}
