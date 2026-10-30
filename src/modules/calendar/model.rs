//! Pure calendar model: a time and a format in, a date label and the next wake out.

use std::time::Duration;

use jiff::Zoned;

use super::CalendarConfig;
use crate::modules::ViewState;

pub fn view_state(now: &Zoned, config: &CalendarConfig) -> ViewState {
    ViewState {
        // The config resolver rejects invalid formats; this is only a last resort.
        label: jiff::fmt::strtime::format(&config.format, now).unwrap_or_default(),
        next_wake: next_midnight(now),
        ..ViewState::default()
    }
}

/// Real time until the next local day starts, rounded up to a whole millisecond
/// so a millisecond timer cannot fire before the date has changed.
fn next_midnight(now: &Zoned) -> Option<Duration> {
    let tomorrow = now.tomorrow().ok()?.start_of_day().ok()?;
    let until = now.duration_until(&tomorrow).unsigned_abs();
    Some(Duration::from_millis(
        until.as_nanos().div_ceil(1_000_000) as u64
    ))
}
