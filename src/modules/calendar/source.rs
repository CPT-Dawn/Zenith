//! The one calendar source per process: a [`ZonedSource`] running the calendar model, so it
//! wakes once a day at local midnight and re-syncs after resume and zone changes.

use std::rc::Rc;

use super::{CalendarConfig, model};
use crate::modules::zoned_source::ZonedSource;

pub type CalendarSource = ZonedSource<CalendarConfig>;

impl ZonedSource<CalendarConfig> {
    pub fn start(config: CalendarConfig) -> Rc<Self> {
        Self::start_with("calendar", config, model::view_state)
    }
}
