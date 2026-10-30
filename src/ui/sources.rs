//! One data source per module per process, shared by every bar.
//! A source is started only if its module is in the layout. Add a field and a line per module.

use std::rc::Rc;

use crate::config::ResolvedConfig;
use crate::modules::ModuleId;
use crate::modules::calendar::source::CalendarSource;
use crate::modules::clock::source::ClockSource;
use crate::modules::logo::source::LogoSource;
use crate::modules::media::source::MediaSource;
use crate::modules::system::source::SystemSource;
use crate::modules::todo::{self, source::TodoSource};

#[derive(Clone, Default)]
pub struct Sources {
    pub clock: Option<Rc<ClockSource>>,
    pub logo: Option<Rc<LogoSource>>,
    pub todo: Option<Rc<TodoSource>>,
    pub media: Option<Rc<MediaSource>>,
    pub calendar: Option<Rc<CalendarSource>>,
    pub system: Option<Rc<SystemSource>>,
}

impl Sources {
    pub fn start(config: &ResolvedConfig) -> Self {
        let on = |id| config.layout.contains(id);
        Self {
            clock: on(ModuleId::Clock).then(|| ClockSource::start(config.clock.clone())),
            logo: on(ModuleId::Logo).then(|| LogoSource::start(&config.logo)),
            todo: on(ModuleId::Todo).then(|| TodoSource::start(todo::data_file())),
            media: on(ModuleId::Media).then(|| MediaSource::start(config.media.clone())),
            calendar: on(ModuleId::Calendar)
                .then(|| CalendarSource::start(config.calendar.clone())),
            system: on(ModuleId::System).then(|| SystemSource::start(config.system.clone())),
        }
    }

    /// Hot reload: follow a new config. Sources of modules that joined the layout start,
    /// those that left are dropped, and running ones are reconfigured in place (so views
    /// stay subscribed) when their table changed. One `follow` per module.
    pub fn reconfigure(&mut self, old: &ResolvedConfig, new: &ResolvedConfig) {
        let on = |id| new.layout.contains(id);
        follow(
            &mut self.clock,
            on(ModuleId::Clock),
            old.clock != new.clock,
            || ClockSource::start(new.clock.clone()),
            |s| s.reconfigure(new.clock.clone()),
        );
        follow(
            &mut self.logo,
            on(ModuleId::Logo),
            old.logo != new.logo,
            || LogoSource::start(&new.logo),
            |s| s.reconfigure(&new.logo),
        );
        follow(
            &mut self.calendar,
            on(ModuleId::Calendar),
            old.calendar != new.calendar,
            || CalendarSource::start(new.calendar.clone()),
            |s| s.reconfigure(new.calendar.clone()),
        );
        follow(
            &mut self.system,
            on(ModuleId::System),
            old.system != new.system,
            || SystemSource::start(new.system.clone()),
            |s| s.reconfigure(new.system.clone()),
        );
    }
}

/// Bring one source slot in line with the config: start it if its module is `wanted` and
/// it is not running, drop it if not `wanted`, `reconfigure` it in place if its config
/// `changed`. A source without settings passes `|_| {}`.
pub fn follow<S>(
    slot: &mut Option<Rc<S>>,
    wanted: bool,
    changed: bool,
    start: impl FnOnce() -> Rc<S>,
    reconfigure: impl FnOnce(&Rc<S>),
) {
    match (slot.as_ref(), wanted) {
        (None, true) => *slot = Some(start()),
        (Some(_), false) => *slot = None,
        (Some(source), true) if changed => reconfigure(source),
        _ => {}
    }
}
