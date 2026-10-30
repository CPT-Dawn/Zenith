//! The one clock source per process: a [`ZonedSource`] running the clock model, so it
//! wakes on the next minute (or second) boundary and re-syncs after resume and zone changes.

use std::rc::Rc;

use super::{ClockConfig, model};
use crate::modules::zoned_source::ZonedSource;

pub type ClockSource = ZonedSource<ClockConfig>;

impl ZonedSource<ClockConfig> {
    pub fn start(config: ClockConfig) -> Rc<Self> {
        Self::start_with("clock", config, model::view_state)
    }
}
