use pretty_assertions::assert_eq;
use zenith_bar::modules::cockpit::model::{
    self, Chip, Feed, FeedState, GlanceItem, MAX_CHIPS, Row, Tone, age, countdown, glance, parse,
};

const FIXTURE: &str = include_str!("fixtures/cockpit-feed.json");

fn time(s: &str) -> jiff::Timestamp {
    s.parse().unwrap()
}

#[test]
fn the_real_feed_parses_in_order() {
    let feed = parse(FIXTURE).expect("fixture parses");

    let glance: Vec<(&str, &str, Tone)> = feed
        .glance
        .iter()
        .map(|g| (g.label.as_str(), g.text.as_str(), g.tone))
        .collect();
    assert_eq!(
        glance,
        [
            ("Claude", "5%", Tone::Ok),
            ("Codex", "87%", Tone::Critical),
            ("agy", "64%", Tone::Warn),
        ]
    );
    assert_eq!(
        feed.glance[1].tooltip.as_deref(),
        Some("Codex: 5h 0%, week 87%")
    );
    let ids: Vec<&str> = feed.sections.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids[..4], ["claude", "codex", "agy", "factory"]);

    let codex = &feed.sections[1];
    assert_eq!(codex.note.as_deref(), Some("plus · 1 reset left"));
    let Row::Meter(week) = &codex.rows[1] else {
        panic!("a meter")
    };
    assert_eq!(week.label, "week");
    assert_eq!(week.value, 0.87);
    assert_eq!(week.tone, Tone::Critical);
    assert_eq!(week.until, Some(time("2026-11-28T02:00:56Z")));

    let Row::Text(lab) = &feed.sections[3].rows[0] else {
        panic!("a text row")
    };
    assert_eq!(
        (lab.label.as_str(), lab.text.as_str(), lab.tone),
        ("lab", "0 leased · 3 free", Some(Tone::Muted))
    );
}

#[test]
fn another_version_is_refused_with_a_clear_error() {
    let error = parse(r#"{"version": 2, "generated": "2026-11-25T15:29:09Z"}"#).unwrap_err();
    assert_eq!(
        error,
        "feed version 2 is not supported (this zenith-bar reads version 1)"
    );
    assert_eq!(
        parse(r#"{"generated": "2026-11-25T15:29:09Z"}"#).unwrap_err(),
        "no `version` (expected 1)"
    );
    assert!(
        parse("not json")
            .unwrap_err()
            .starts_with("not a JSON document")
    );
}

#[test]
fn optional_fields_can_be_left_out() {
    let feed = parse(
        r#"{"version": 1, "generated": "2026-11-25T15:29:09+01:00",
            "glance": [{"id": "a", "label": "A", "text": "1", "tone": "ok"}],
            "sections": [
              {"id": "s", "title": "S", "rows": [
                {"kind": "meter", "label": "m", "value": 0.5, "text": "50%", "tone": "busy"},
                {"kind": "text", "label": "t", "text": "x"}
              ]},
              {"id": "e", "title": "E", "error": "down", "stale_since": "2026-11-25T15:00:00Z"}
            ]}"#,
    )
    .expect("valid");

    assert_eq!(feed.glance[0].tooltip, None);
    let section = &feed.sections[0];
    assert_eq!(
        (
            section.note.clone(),
            section.error.clone(),
            section.stale_since
        ),
        (None, None, None)
    );
    let Row::Meter(meter) = &section.rows[0] else {
        panic!()
    };
    assert_eq!(meter.until, None);
    let Row::Text(text) = &section.rows[1] else {
        panic!()
    };
    assert_eq!((text.tone, text.url.clone()), (None, None));
    assert_eq!(feed.sections[1].rows, Vec::new());
    assert_eq!(feed.sections[1].error.as_deref(), Some("down"));
    assert_eq!(
        feed.sections[1].stale_since,
        Some(time("2026-11-25T15:00:00Z"))
    );
    // Leaving out glance and sections entirely is an empty feed.
    assert_eq!(
        parse(r#"{"version": 1, "generated": "2026-11-25T15:29:09Z"}"#),
        Ok(Feed::default())
    );
}

#[test]
fn unknown_fields_and_row_kinds_are_ignored() {
    let feed = parse(
        r#"{"version": 1, "generated": "2026-11-25T15:29:09Z", "theme": "new",
            "sections": [{"id": "s", "title": "S", "icon": "x", "rows": [
              {"kind": "sparkline", "points": [1, 2]},
              {"label": "no kind"},
              {"kind": "text", "label": "t", "text": "x", "extra": true}
            ]}]}"#,
    )
    .expect("valid");
    assert_eq!(feed.sections[0].rows.len(), 1);
}

#[test]
fn an_unknown_tone_is_muted_but_a_tone_must_be_a_string() {
    let row = |tone: &str| {
        format!(
            r#"{{"version": 1, "generated": "2026-11-25T15:29:09Z",
                "glance": [{{"id": "a", "label": "A", "text": "1", "tone": {tone}}}]}}"#
        )
    };
    assert_eq!(
        parse(&row("\"purple\"")).unwrap().glance[0].tone,
        Tone::Muted
    );
    assert!(parse(&row("5")).is_err());
}

#[test]
fn bad_values_are_refused_or_clamped() {
    let meter = |value: &str, until: &str| {
        format!(
            r#"{{"version": 1, "generated": "2026-11-25T15:29:09Z", "sections": [{{"id": "s",
                "title": "S", "rows": [{{"kind": "meter", "label": "m", "value": {value},
                "text": "", "tone": "ok", "until": {until}}}]}}]}}"#
        )
    };
    let Row::Meter(m) = &parse(&meter("1.7", "null")).unwrap().sections[0].rows[0] else {
        panic!()
    };
    assert_eq!(m.value, 1.0);
    let error = parse(&meter("0.5", "\"soon\"")).unwrap_err();
    assert!(error.starts_with("section `s`: row 0:"), "{error}");
    assert!(error.contains("`soon` is not an RFC 3339 time"), "{error}");
    assert!(
        parse(r#"{"version": 1, "generated": "yesterday"}"#)
            .unwrap_err()
            .contains("`generated`")
    );
    // A required field missing rejects the document.
    assert!(
        parse(r#"{"version": 1, "generated": "2026-11-25T15:29:09Z", "glance": [{"id": "a"}]}"#)
            .is_err()
    );
}

fn item(label: &str) -> GlanceItem {
    GlanceItem {
        id: label.to_lowercase(),
        label: label.into(),
        text: "1".into(),
        tone: Tone::Ok,
        tooltip: None,
    }
}

fn feed(command: &str, labels: &[&str], failure: Option<&str>) -> FeedState {
    FeedState {
        command: command.into(),
        last: Some(Feed {
            glance: labels.iter().map(|l| item(l)).collect(),
            sections: Vec::new(),
        }),
        failure: failure.map(String::from),
    }
}

fn labels(chips: &[Chip]) -> Vec<String> {
    chips
        .iter()
        .map(|c| {
            if c.failed {
                "!".into()
            } else {
                c.label.clone()
            }
        })
        .collect()
}

#[test]
fn chips_follow_feed_order_then_item_order() {
    let chips = glance(&[feed("a", &["A1", "A2"], None), feed("b", &["B1"], None)]);
    assert_eq!(labels(&chips), ["A1", "A2", "B1"]);
    assert_eq!(chips[0].tone, Tone::Ok);
}

#[test]
fn the_glance_is_capped() {
    let chips = glance(&[
        feed("a", &["1", "2", "3", "4"], None),
        feed("b", &["5", "6", "7"], None),
    ]);
    assert_eq!(chips.len(), MAX_CHIPS);
    assert_eq!(labels(&chips), ["1", "2", "3", "4", "5", "6"]);
}

#[test]
fn a_failed_feed_keeps_its_chips_and_adds_a_muted_bang_that_the_cap_never_cuts() {
    let chips = glance(&[
        feed("a", &["1", "2", "3", "4", "5", "6"], None),
        feed("b", &["7"], Some("timed out after 30 s")),
    ]);
    assert_eq!(labels(&chips), ["1", "2", "3", "4", "5", "!"]);
    let bang = chips.last().unwrap();
    assert_eq!((bang.text.as_str(), bang.tone), ("!", Tone::Muted));
    assert_eq!(
        bang.tooltip.as_deref(),
        Some("Feed `b` failed: timed out after 30 s")
    );

    let never_worked = FeedState {
        command: "c".into(),
        last: None,
        failure: Some("exited with status 1".into()),
    };
    assert_eq!(labels(&glance(&[never_worked])), ["!"]);
    assert_eq!(glance(&[FeedState::default()]), Vec::new());
}

#[test]
fn countdowns_read_at_a_glance() {
    let now = time("2026-11-25T15:00:00Z");
    let at = |secs: i64| {
        countdown(
            now.checked_add(jiff::SignedDuration::from_secs(secs))
                .unwrap(),
            now,
        )
    };
    assert_eq!(at(45), "45s");
    assert_eq!(at(12 * 60 + 30), "12m");
    assert_eq!(at(3 * 3600 + 36 * 60 + 59), "3h36m");
    assert_eq!(at(3 * 3600 + 5 * 60), "3h05m");
    assert_eq!(at(2 * 86_400 + 10 * 3600 + 7), "2d10h");
    assert_eq!(at(0), "now");
    assert_eq!(at(-90), "now");
}

#[test]
fn ages_use_their_largest_unit() {
    let now = time("2026-11-25T15:00:00Z");
    let ago = |secs: i64| {
        age(
            now.checked_sub(jiff::SignedDuration::from_secs(secs))
                .unwrap(),
            now,
        )
    };
    assert_eq!(ago(9), "9s");
    assert_eq!(ago(10 * 60 + 59), "10m");
    assert_eq!(ago(3 * 3600), "3h");
    assert_eq!(ago(2 * 86_400 + 5), "2d");
    assert_eq!(model::Tone::Busy.css_class(), "zb-tone-busy");
}
