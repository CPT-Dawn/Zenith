use pretty_assertions::assert_eq;
use zenith_bar::config::{Diagnostic, resolve};

#[test]
fn calendar_format_defaults_to_a_short_date() {
    let config = resolve("").expect("empty config resolves");

    assert_eq!(config.calendar.format, "%a %d %b");
}

#[test]
fn calendar_format_is_configurable() {
    let config = resolve("[calendar]\nformat = \"%Y-%m-%d\"\n").expect("valid config");

    assert_eq!(config.calendar.format, "%Y-%m-%d");
}

#[test]
fn an_invalid_calendar_format_is_reported_at_its_value() {
    let diagnostics = resolve("[calendar]\nformat = \"%d %\"\n").expect_err("invalid format");
    let lines: Vec<String> = diagnostics.iter().map(Diagnostic::to_string).collect();

    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("2:10: invalid calendar format"),
        "{lines:?}"
    );
}
