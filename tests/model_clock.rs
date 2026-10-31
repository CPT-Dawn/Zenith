use std::time::Duration;

use jiff::Zoned;
use pretty_assertions::assert_eq;
use zenith_bar::modules::clock::{ClockConfig, model};

fn at(time: &str) -> Zoned {
    time.parse().expect("valid zoned datetime")
}

fn clock(format: &str) -> ClockConfig {
    ClockConfig {
        format: format.into(),
        ..ClockConfig::default()
    }
}

#[test]
fn label_is_the_formatted_local_time() {
    let state = model::view_state(
        &at("2026-10-31T14:07:23.5[Europe/Berlin]"),
        &clock("%a %H:%M"),
    );

    assert_eq!(state.label, "Sat 14:07");
    assert!(state.visible);
}

#[test]
fn without_seconds_it_wakes_exactly_at_the_next_minute() {
    let state = model::view_state(&at("2026-10-31T14:07:23.5[UTC]"), &clock("%H:%M"));

    assert_eq!(state.next_wake, Some(Duration::from_millis(36_500)));
}

#[test]
fn on_the_boundary_itself_it_waits_a_whole_minute() {
    let state = model::view_state(&at("2026-10-31T14:08:00[UTC]"), &clock("%H:%M"));

    assert_eq!(state.label, "14:08");
    assert_eq!(state.next_wake, Some(Duration::from_secs(60)));
}

#[test]
fn with_seconds_it_wakes_at_the_next_second() {
    let state = model::view_state(&at("2026-10-31T14:07:23.25[UTC]"), &clock("%H:%M:%S"));

    assert_eq!(state.label, "14:07:23");
    assert_eq!(state.next_wake, Some(Duration::from_millis(750)));
}

#[test]
fn every_way_of_showing_seconds_switches_to_second_ticks() {
    let now = at("2026-10-31T14:07:23.25[UTC]");
    for format in ["%S", "%T", "%r", "%s", "%-S", "%c", "%H:%M:%_S"] {
        let state = model::view_state(&now, &clock(format));
        assert_eq!(
            state.next_wake,
            Some(Duration::from_millis(750)),
            "{format}"
        );
    }
}

#[test]
fn formats_without_seconds_stay_on_minute_ticks() {
    let now = at("2026-10-31T14:07:23.25[UTC]");
    for format in [
        "%H:%M",
        "%I:%M %p",
        "%a %d %b  %H:%M",
        "%R",
        "%Z %z",
        "plain text",
    ] {
        let state = model::view_state(&now, &clock(format));
        assert_eq!(
            state.next_wake,
            Some(Duration::from_millis(36_750)),
            "{format}"
        );
    }
}

#[test]
fn the_wake_rounds_up_so_a_millisecond_timer_never_fires_early() {
    let state = model::view_state(&at("2026-10-31T14:07:59.9995[UTC]"), &clock("%H:%M"));

    assert_eq!(state.next_wake, Some(Duration::from_millis(1)));
}

#[test]
fn half_hour_time_zones_still_tick_on_the_local_minute() {
    let state = model::view_state(&at("2026-10-31T19:37:10[Asia/Kolkata]"), &clock("%H:%M"));

    assert_eq!(state.label, "19:37");
    assert_eq!(state.next_wake, Some(Duration::from_secs(50)));
}

#[test]
fn the_label_follows_a_dst_jump() {
    let before = model::view_state(
        &at("2026-03-08T01:59:30[America/New_York]"),
        &clock("%H:%M %Z"),
    );
    let after = model::view_state(
        &at("2026-03-08T03:00:00[America/New_York]"),
        &clock("%H:%M %Z"),
    );

    assert_eq!(before.label, "01:59 EST");
    assert_eq!(before.next_wake, Some(Duration::from_secs(30)));
    assert_eq!(after.label, "03:00 EDT");
}
