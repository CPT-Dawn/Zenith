//! A source driven by local wall-clock time, shared by the clock and the calendar.
//!
//! A one-shot timer is re-armed from the model's `next_wake`; there is no fixed tick.
//! Re-syncs after resume and time zone changes are event-driven.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::modules::{Shared, ViewState};

/// The pure model a [`ZonedSource`] runs: local time and config in, view state out.
pub type ZonedModel<C> = fn(&Zoned, &C) -> ViewState;

pub struct ZonedSource<C> {
    name: &'static str,
    model: ZonedModel<C>,
    state: Rc<Shared<ViewState>>,
    config: RefCell<C>,
    zone: RefCell<TimeZone>,
    timer: RefCell<Option<glib::SourceId>>,
    resume: RefCell<Option<gio::SignalSubscription>>,
    zone_monitor: RefCell<Option<gio::FileMonitor>>,
}

impl<C: 'static> ZonedSource<C> {
    /// Start a source; `name` prefixes its log lines.
    pub fn start_with(name: &'static str, config: C, model: ZonedModel<C>) -> Rc<Self> {
        let source = Rc::new(Self {
            name,
            model,
            state: Shared::new(ViewState::default()),
            config: RefCell::new(config),
            zone: RefCell::new(local_zone(name)),
            timer: RefCell::new(None),
            resume: RefCell::new(None),
            zone_monitor: RefCell::new(None),
        });
        source.update();
        source.watch_resume();
        source.watch_zone();
        source
    }

    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    pub fn reconfigure(self: &Rc<Self>, config: C) {
        *self.config.borrow_mut() = config;
        self.resync();
    }

    /// Recompute now and re-arm, discarding the pending timer.
    fn resync(self: &Rc<Self>) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
        self.update();
    }

    /// Publish the current state and arm a one-shot timer for the next boundary.
    fn update(self: &Rc<Self>) {
        let now = Zoned::new(Timestamp::now(), self.zone.borrow().clone());
        let mut state = (self.model)(&now, &self.config.borrow());
        let wake = state.next_wake.take();
        self.state.set(state);

        if let Some(wake) = wake {
            let weak = Rc::downgrade(self);
            let timer = glib::timeout_add_local_once(wake, move || {
                if let Some(source) = weak.upgrade() {
                    // This timer has fired; forget it rather than remove it.
                    source.timer.take();
                    source.update();
                }
            });
            *self.timer.borrow_mut() = Some(timer);
        }
    }

    /// Monotonic timers stop during suspend, so re-sync when logind says we resumed.
    fn watch_resume(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        gio::bus_get(gio::BusType::System, gio::Cancellable::NONE, move |bus| {
            let Some(source) = weak.upgrade() else { return };
            let bus = match bus {
                Ok(bus) => bus,
                Err(e) => {
                    log::warn!(
                        "{}: no system bus, will not re-sync after resume: {e}",
                        source.name
                    );
                    return;
                }
            };
            let weak = Rc::downgrade(&source);
            let subscription = bus.subscribe_to_signal(
                Some("org.freedesktop.login1"),
                Some("org.freedesktop.login1.Manager"),
                Some("PrepareForSleep"),
                Some("/org/freedesktop/login1"),
                None,
                gio::DBusSignalFlags::NONE,
                move |signal| {
                    let going_to_sleep = signal.parameters.child_get::<bool>(0);
                    if !going_to_sleep {
                        with(&weak, |source| {
                            log::debug!("{}: resumed, re-syncing", source.name);
                            source.resync();
                        });
                    }
                },
            );
            *source.resume.borrow_mut() = Some(subscription);
        });
    }

    /// `/etc/localtime` is replaced when the time zone changes (timedatectl, tzupdate, …).
    fn watch_zone(self: &Rc<Self>) {
        let file = gio::File::for_path("/etc/localtime");
        let monitor =
            match file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE) {
                Ok(monitor) => monitor,
                Err(e) => {
                    log::warn!(
                        "{}: cannot watch /etc/localtime for time zone changes: {e}",
                        self.name
                    );
                    return;
                }
            };
        let weak = Rc::downgrade(self);
        monitor.connect_changed(move |_, _, _, event| {
            use gio::FileMonitorEvent::*;
            if matches!(event, Changed | Created | Deleted | Renamed | MovedIn) {
                with(&weak, |source| {
                    *source.zone.borrow_mut() = local_zone(source.name);
                    log::debug!("{}: time zone changed, re-syncing", source.name);
                    source.resync();
                });
            }
        });
        *self.zone_monitor.borrow_mut() = Some(monitor);
    }
}

impl<C> Drop for ZonedSource<C> {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

fn with<C: 'static>(weak: &Weak<ZonedSource<C>>, f: impl FnOnce(&Rc<ZonedSource<C>>)) {
    if let Some(source) = weak.upgrade() {
        f(&source);
    }
}

/// The current local zone, read fresh. `TimeZone::system()` caches for minutes,
/// which would leave the source in the old zone after a change.
fn local_zone(name: &str) -> TimeZone {
    if std::env::var_os("TZ").is_some() {
        return TimeZone::try_system().unwrap_or(TimeZone::UTC);
    }
    let named = std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|target| {
            let target = target.to_string_lossy().into_owned();
            let name = target.split_once("zoneinfo/")?.1.to_string();
            TimeZone::get(&name).ok()
        });
    named
        .or_else(|| {
            let data = std::fs::read("/etc/localtime").ok()?;
            TimeZone::tzif("Local", &data).ok()
        })
        .unwrap_or_else(|| {
            log::warn!("{name}: cannot read the local time zone, using UTC");
            TimeZone::UTC
        })
}
