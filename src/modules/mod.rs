//! Modules and the contract every module shares.
//!
//! Each module is a directory with the same five files:
//! - `config.rs`: its `[table]` in config.toml (serde + schemars, every field defaulted)
//! - `model.rs`: pure, no GTK or I/O; typed inputs in, [`ViewState`] out
//! - `source.rs`: the one data source per process; publishes into a [`Shared`]
//! - `view.rs`: a thin GTK binder subscribed to the `Shared`
//! - `mod.rs`: re-exports
//!
//! Extend this file only additively (new variants, new fields with defaults).

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;

pub mod calendar;
pub mod clock;
pub mod logo;
pub mod system;
pub mod todo;
pub mod zoned_source;

/// Every module Zenith knows about. Names are the strings used in `[layout]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleId {
    Clock,
    Calendar,
    Media,
    System,
    Todo,
    Logo,
}

impl ModuleId {
    pub const ALL: [ModuleId; 6] = [
        ModuleId::Clock,
        ModuleId::Calendar,
        ModuleId::Media,
        ModuleId::System,
        ModuleId::Todo,
        ModuleId::Logo,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ModuleId::Clock => "clock",
            ModuleId::Calendar => "calendar",
            ModuleId::Media => "media",
            ModuleId::System => "system",
            ModuleId::Todo => "todo",
            ModuleId::Logo => "logo",
        }
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModuleId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ModuleId::ALL
            .into_iter()
            .find(|id| id.as_str() == s)
            .ok_or(())
    }
}

/// What a module's view shows. Produced by models, applied by views.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewState {
    pub label: String,
    /// Icon name, e.g. a bundled `zb-logo-symbolic`.
    pub icon: Option<String>,
    /// State classes only (`zb-warm`, `zb-playing`, …). `zb-module zb-<id>` is added by the bar.
    pub classes: Vec<&'static str>,
    /// 0.0..=1.0
    pub progress: Option<f64>,
    pub visible: bool,
    pub tooltip: Option<String>,
    /// When the model next wants to be asked again, relative to the `now` it was given.
    /// `None` means it only changes on events.
    pub next_wake: Option<Duration>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            label: String::new(),
            icon: None,
            classes: Vec::new(),
            progress: None,
            visible: true,
            tooltip: None,
            next_wake: None,
        }
    }
}

/// A value published by one source and observed by any number of views (one per bar).
pub struct Shared<T> {
    value: RefCell<T>,
    subscribers: RefCell<Vec<(Subscription, Subscriber<T>)>>,
    next_id: Cell<u64>,
}

type Subscriber<T> = Box<dyn Fn(&T)>;

/// Handle returned by [`Shared::subscribe`]; pass it to [`Shared::unsubscribe`] when a view goes away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subscription(u64);

impl<T: PartialEq> Shared<T> {
    pub fn new(value: T) -> Rc<Self> {
        Rc::new(Self {
            value: RefCell::new(value),
            subscribers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
        })
    }

    /// Replace the value and notify subscribers, unless nothing changed.
    pub fn set(&self, value: T) {
        if *self.value.borrow() == value {
            return;
        }
        *self.value.borrow_mut() = value;
        let value = self.value.borrow();
        for (_, subscriber) in self.subscribers.borrow().iter() {
            subscriber(&value);
        }
    }

    /// Call `f` now with the current value and again after every change.
    pub fn subscribe(&self, f: impl Fn(&T) + 'static) -> Subscription {
        f(&self.value.borrow());
        let id = Subscription(self.next_id.get());
        self.next_id.set(id.0 + 1);
        self.subscribers.borrow_mut().push((id, Box::new(f)));
        id
    }

    pub fn unsubscribe(&self, subscription: Subscription) {
        self.subscribers
            .borrow_mut()
            .retain(|(id, _)| *id != subscription);
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value.borrow())
    }
}
