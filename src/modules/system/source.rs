//! The one system source per process: a single `timeout_add_seconds_local` timer,
//! however many metrics and bars there are. Each tick reads only the files the
//! enabled metrics need; the temperature sensor is found once and then read directly.

use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk4::glib;

use super::model::{Readings, SystemModel};
use super::{Metric, SystemConfig, sensor};
use crate::modules::{Shared, ViewState};

pub struct SystemSource {
    state: Rc<Shared<ViewState>>,
    inner: RefCell<Inner>,
}

struct Inner {
    config: SystemConfig,
    model: SystemModel,
    sensor: Option<PathBuf>,
    timer: Option<glib::SourceId>,
    stat: Input,
    meminfo: Input,
    temperature: Input,
}

impl SystemSource {
    pub fn start(config: SystemConfig) -> Rc<Self> {
        let source = Rc::new(Self {
            state: Shared::new(ViewState::default()),
            inner: RefCell::new(Inner {
                sensor: find_sensor(&config),
                config,
                model: SystemModel::default(),
                timer: None,
                stat: Input::default(),
                meminfo: Input::default(),
                temperature: Input::default(),
            }),
        });
        source.tick();
        source.arm();
        source
    }

    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    pub fn reconfigure(self: &Rc<Self>, config: SystemConfig) {
        {
            let mut inner = self.inner.borrow_mut();
            let rediscover = config.temperature.sensor != inner.config.temperature.sensor
                || wants_temperature(&config) != wants_temperature(&inner.config);
            if rediscover {
                inner.sensor = find_sensor(&config);
                inner.temperature = Input::default();
            }
            inner.config = config;
        }
        self.tick();
        self.arm();
    }

    /// (Re)start the one periodic timer at the configured interval.
    fn arm(self: &Rc<Self>) {
        let mut inner = self.inner.borrow_mut();
        if let Some(timer) = inner.timer.take() {
            timer.remove();
        }
        let weak = Rc::downgrade(self);
        let interval = inner.config.interval.max(1);
        inner.timer = Some(glib::timeout_add_seconds_local(
            interval,
            move || match weak.upgrade() {
                Some(source) => {
                    source.tick();
                    glib::ControlFlow::Continue
                }
                None => glib::ControlFlow::Break,
            },
        ));
    }

    fn tick(&self) {
        let state = {
            let inner = &mut *self.inner.borrow_mut();
            let wants = |m| inner.config.metrics.contains(&m);
            let stat = wants(Metric::Cpu).then(|| inner.stat.read(Path::new("/proc/stat")));
            let meminfo =
                wants(Metric::Memory).then(|| inner.meminfo.read(Path::new("/proc/meminfo")));
            let temperature = match &inner.sensor {
                Some(path) if wants(Metric::Temperature) => Some(inner.temperature.read(path)),
                _ => None,
            };
            let readings = Readings {
                stat: stat.flatten(),
                meminfo: meminfo.flatten(),
                temperature: temperature.flatten(),
            };
            inner.model.update(&readings, &inner.config)
        };
        self.state.set(state);
    }
}

impl Drop for SystemSource {
    fn drop(&mut self) {
        if let Some(timer) = self.inner.get_mut().timer.take() {
            timer.remove();
        }
    }
}

fn wants_temperature(config: &SystemConfig) -> bool {
    config.metrics.contains(&Metric::Temperature)
}

fn find_sensor(config: &SystemConfig) -> Option<PathBuf> {
    if !wants_temperature(config) {
        return None;
    }
    let pinned = config
        .temperature
        .sensor
        .as_deref()
        .filter(|s| !s.is_empty());
    let found = sensor::discover(Path::new("/sys"), pinned);
    match (&found, pinned) {
        (Some(path), _) => log::debug!("system: temperature from {}", path.display()),
        (None, Some(pinned)) => {
            log::warn!("system: pinned temperature sensor `{pinned}` not found, hiding temperature")
        }
        (None, None) => log::warn!(
            "system: no CPU temperature sensor found, hiding temperature \
             (pin one with [system.temperature] sensor)"
        ),
    }
    found
}

/// One file read every tick into a reused buffer, warning once per outage.
#[derive(Default)]
struct Input {
    buffer: String,
    failing: bool,
}

impl Input {
    fn read(&mut self, path: &Path) -> Option<&str> {
        self.buffer.clear();
        let result = File::open(path).and_then(|mut f| f.read_to_string(&mut self.buffer));
        match result {
            Ok(_) => {
                self.failing = false;
                Some(&self.buffer)
            }
            Err(e) => {
                if !self.failing {
                    log::warn!("system: cannot read {}: {e}", path.display());
                }
                self.failing = true;
                None
            }
        }
    }
}
