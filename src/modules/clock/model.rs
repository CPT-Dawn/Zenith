//! Pure clock model: a time and a format in, a label and the next wake out.

use std::time::Duration;

use jiff::{SignedDuration, Zoned};

use super::ClockConfig;
use crate::modules::ViewState;

pub fn view_state(now: &Zoned, config: &ClockConfig) -> ViewState {
    ViewState {
        label: format(now, &config.format),
        next_wake: Some(next_wake(now, &config.format)),
        ..ViewState::default()
    }
}

fn format(now: &Zoned, format: &str) -> String {
    // The config resolver rejects invalid formats; this is only a last resort.
    jiff::fmt::strtime::format(format, now).unwrap_or_default()
}

/// Time until the next boundary the label can change on: the next second if the
/// format shows seconds, otherwise the next minute. Rounded up to a whole
/// millisecond so a millisecond timer cannot fire before the boundary.
fn next_wake(now: &Zoned, format_str: &str) -> Duration {
    let into_second = Duration::from_nanos(now.subsec_nanosecond() as u64);
    let into_minute = Duration::from_secs(now.second() as u64) + into_second;
    let until = if shows_seconds(now, format_str) {
        Duration::from_secs(1) - into_second
    } else {
        Duration::from_secs(60) - into_minute
    };
    Duration::from_millis(until.as_nanos().div_ceil(1_000_000) as u64)
}

/// Whether the format renders differently within one minute. Asking the
/// formatter itself covers every directive (`%S`, `%T`, `%r`, `%s`, `%c`, …).
fn shows_seconds(now: &Zoned, format_str: &str) -> bool {
    let start = now.saturating_sub(SignedDuration::from_secs(now.second() as i64));
    let start = start.saturating_sub(SignedDuration::from_nanos(now.subsec_nanosecond() as i64));
    let later = start.saturating_add(SignedDuration::from_secs(1));
    format(&start, format_str) != format(&later, format_str)
}
