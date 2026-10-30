//! The one clock source per process: a one-shot timer re-armed from the model's
//! `next_wake`, plus re-syncs after resume and time zone changes (both event-driven).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use gtk4::gio::prelude::*;
use gtk4::{gio, glib};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use super::{ClockConfig, model};
use crate::modules::{Shared, ViewState};

pub struct ClockSource {
    state: Rc<Shared<ViewState>>,
    config: RefCell<ClockConfig>,
    zone: RefCell<TimeZone>,
    timer: RefCell<Option<glib::SourceId>>,
    resume: RefCell<Option<gio::SignalSubscription>>,
    zone_monitor: RefCell<Option<gio::FileMonitor>>,
}

impl ClockSource {
    pub fn start(config: ClockConfig) -> Rc<Self> {
        let source = Rc::new(Self {
            state: Shared::new(ViewState::default()),
            config: RefCell::new(config),
            zone: RefCell::new(local_zone()),
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

    pub fn reconfigure(self: &Rc<Self>, config: ClockConfig) {
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

    /// Publish the current time and arm a one-shot timer for the next boundary.
    fn update(self: &Rc<Self>) {
        let now = Zoned::new(Timestamp::now(), self.zone.borrow().clone());
        let mut state = model::view_state(&now, &self.config.borrow());
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
                    log::warn!("clock: no system bus, will not re-sync after resume: {e}");
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
                        log::debug!("clock: resumed, re-syncing");
                        with(&weak, |source| source.resync());
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
                    log::warn!("clock: cannot watch /etc/localtime for time zone changes: {e}");
                    return;
                }
            };
        let weak = Rc::downgrade(self);
        monitor.connect_changed(move |_, _, _, event| {
            use gio::FileMonitorEvent::*;
            if matches!(event, Changed | Created | Deleted | Renamed | MovedIn) {
                with(&weak, |source| {
                    *source.zone.borrow_mut() = local_zone();
                    log::debug!("clock: time zone changed, re-syncing");
                    source.resync();
                });
            }
        });
        *self.zone_monitor.borrow_mut() = Some(monitor);
    }
}

impl Drop for ClockSource {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.remove();
        }
    }
}

fn with(weak: &Weak<ClockSource>, f: impl FnOnce(&Rc<ClockSource>)) {
    if let Some(source) = weak.upgrade() {
        f(&source);
    }
}

/// The current local zone, read fresh. `TimeZone::system()` caches for minutes,
/// which would leave the clock in the old zone after a change.
fn local_zone() -> TimeZone {
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
            log::warn!("clock: cannot read the local time zone, using UTC");
            TimeZone::UTC
        })
}
