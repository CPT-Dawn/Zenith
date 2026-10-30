//! One data source per module per process, shared by every bar.
//! A source is started only if its module is in the layout. Add a field and a line per module.

use std::rc::Rc;

use crate::config::ResolvedConfig;
use crate::modules::ModuleId;
use crate::modules::clock::source::ClockSource;
use crate::modules::logo::source::LogoSource;
use crate::modules::system::source::SystemSource;

#[derive(Default)]
pub struct Sources {
    pub clock: Option<Rc<ClockSource>>,
    pub logo: Option<Rc<LogoSource>>,
    pub system: Option<Rc<SystemSource>>,
}

impl Sources {
    pub fn start(config: &ResolvedConfig) -> Self {
        let on = |id| config.layout.contains(id);
        Self {
            clock: on(ModuleId::Clock).then(|| ClockSource::start(config.clock.clone())),
            logo: on(ModuleId::Logo).then(|| LogoSource::start(&config.logo)),
            system: on(ModuleId::System).then(|| SystemSource::start(config.system.clone())),
        }
    }
}
