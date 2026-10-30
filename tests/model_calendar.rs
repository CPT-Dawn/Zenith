use std::time::Duration;

use jiff::Zoned;
use pretty_assertions::assert_eq;
use zenith_bar::modules::calendar::{CalendarConfig, model};

fn at(time: &str) -> Zoned {
    time.parse().expect("valid zoned datetime")
}

fn calendar(format: &str) -> CalendarConfig {
    CalendarConfig {
        format: format.into(),
    }
}

#[test]
fn label_is_the_formatted_local_date() {
    let state = model::view_state(
        &at("2026-10-31T14:07:23[Europe/Berlin]"),
        &calendar("%a %d %b"),
    );

    assert_eq!(state.label, "Sat 31 Oct");
    assert!(state.visible);
}

#[test]
fn it_wakes_exactly_at_the_next_local_midnight() {
    let state = model::view_state(&at("2026-10-31T14:07:23.5[UTC]"), &calendar("%F"));

    // 14:07:23.5 + 9:52:36.5 = 24:00
    assert_eq!(
        state.next_wake,
        Some(Duration::from_millis(
            ((9 * 60 + 52) * 60 + 36) * 1000 + 500
        ))
    );
}

#[test]
fn at_midnight_itself_it_waits_a_whole_day_not_zero() {
    let state = model::view_state(&at("2026-11-01T00:00[UTC]"), &calendar("%a %d %b"));

    assert_eq!(state.label, "Sun 01 Nov");
    assert_eq!(state.next_wake, Some(Duration::from_secs(24 * 3600)));
}

#[test]
fn the_wake_rounds_up_so_a_millisecond_timer_never_fires_early() {
    let state = model::view_state(&at("2026-10-31T23:59:59.9995[UTC]"), &calendar("%F"));

    assert_eq!(state.next_wake, Some(Duration::from_millis(1)));
}

#[test]
fn a_spring_forward_day_is_23_hours_long() {
    let state = model::view_state(&at("2026-03-29T00:00[Europe/Berlin]"), &calendar("%F"));

    assert_eq!(state.label, "2026-03-29");
    assert_eq!(state.next_wake, Some(Duration::from_secs(23 * 3600)));
}

#[test]
fn a_fall_back_day_is_25_hours_long() {
    let state = model::view_state(&at("2026-10-25T00:30[Europe/Berlin]"), &calendar("%F"));

    assert_eq!(
        state.next_wake,
        Some(Duration::from_secs(24 * 3600 + 30 * 60))
    );
}

#[test]
fn when_dst_skips_midnight_the_day_starts_at_one() {
    // Chile springs forward from 24:00 straight to 01:00.
    let before = model::view_state(
        &at("2026-09-05T23:30[America/Santiago]"),
        &calendar("%a %d %b"),
    );
    let after = model::view_state(
        &at("2026-09-06T01:00[America/Santiago]"),
        &calendar("%a %d %b"),
    );

    assert_eq!(before.label, "Sat 05 Sep");
    assert_eq!(before.next_wake, Some(Duration::from_secs(30 * 60)));
    assert_eq!(after.label, "Sun 06 Sep");
    assert_eq!(after.next_wake, Some(Duration::from_secs(23 * 3600)));
}
