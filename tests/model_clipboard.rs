use jiff::tz::TimeZone;
use pretty_assertions::assert_eq;
use std::time::Duration;
use zenith_bar::modules::ViewState;

use hoarder_ipc::{ClipSummary, Event, Op, Recording, State};
use zenith_bar::modules::clipboard::{Backoff, Clipboard, Kind, Recent, Row};

#[test]
fn with_no_daemon_it_is_quietly_offline() {
    let clipboard = Clipboard::new();

    assert_eq!(
        clipboard.view_state(&tz()),
        ViewState {
            icon: Some("zb-clipboard-offline-symbolic".into()),
            classes: vec!["zb-clipboard-offline"],
            tooltip: Some("Clipboard history is offline".into()),
            ..ViewState::default()
        }
    );
}

fn tz() -> TimeZone {
    TimeZone::get("Europe/Berlin").expect("tzdb has Europe/Berlin")
}

fn recording(state: State) -> Recording {
    Recording { state, until: None }
}

#[test]
fn once_connected_it_shows_recording_is_active() {
    let mut clipboard = Clipboard::new();

    clipboard.connected(recording(State::Active), Vec::new());

    let state = clipboard.view_state(&tz());
    assert_eq!(state.icon.as_deref(), Some("zb-clipboard-symbolic"));
    assert_eq!(state.classes, vec!["zb-clipboard-active"]);
    assert_eq!(state.tooltip.as_deref(), Some("Clipboard history"));
}

#[test]
fn a_pause_shows_on_the_bar_until_recording_resumes() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(recording(State::Active), Vec::new());

    clipboard.apply(Event::State(recording(State::Paused)));
    let paused = clipboard.view_state(&tz());
    clipboard.apply(Event::State(recording(State::Active)));
    let resumed = clipboard.view_state(&tz());

    assert_eq!(paused.icon.as_deref(), Some("zb-clipboard-paused-symbolic"));
    assert_eq!(paused.classes, vec!["zb-clipboard-paused"]);
    assert_eq!(
        paused.tooltip.as_deref(),
        Some("Clipboard history is paused\nRight-click to resume")
    );
    assert_eq!(resumed.classes, vec!["zb-clipboard-active"]);
}

#[test]
fn a_timed_pause_says_when_it_ends() {
    let mut clipboard = Clipboard::new();

    clipboard.connected(
        Recording {
            state: State::Paused,
            until: Some("2026-11-08T13:05:00Z".into()),
        },
        Vec::new(),
    );

    assert_eq!(
        clipboard.view_state(&tz()).tooltip.as_deref(),
        Some("Clipboard history is paused until 14:05\nRight-click to resume")
    );
}

/// A Clip as `list` sends it.
fn clip(id: i64, kind: &str, preview: &str) -> ClipSummary {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "kind": kind,
        "preview": preview,
        "app": "org.example.editor",
        "at": "2026-11-08T12:00:00Z",
        "created": "2026-11-08T12:00:00Z",
        "pinned": false,
        "size": preview.len(),
        "types": ["text/plain"],
    }))
    .expect("a valid ClipSummary")
}

fn row(id: i64, kind: Kind, label: &str) -> Row {
    Row {
        id,
        kind,
        label: label.into(),
    }
}

#[test]
fn the_popover_lists_recent_clips_newest_first_with_one_line_labels() {
    let mut clipboard = Clipboard::new();

    clipboard.connected(
        recording(State::Active),
        vec![
            clip(7, "text", "  fn main() {\n    println!(\"hi\");\n}"),
            clip(6, "image", "1920×1080"),
            clip(5, "files", "notes.md, plan.pdf"),
            clip(4, "application/x-new-kind", "?"),
        ],
    );

    assert_eq!(
        clipboard.recent(),
        Recent {
            online: true,
            rows: vec![
                row(7, Kind::Text, "fn main() { println!(\"hi\"); }"),
                row(6, Kind::Image, "1920×1080"),
                row(5, Kind::Files, "notes.md, plan.pdf"),
                row(4, Kind::Other, "?"),
            ],
        }
    );
}

#[test]
fn long_previews_are_cut_with_an_ellipsis() {
    let mut clipboard = Clipboard::new();

    clipboard.connected(
        recording(State::Active),
        vec![clip(1, "text", &"word ".repeat(30))],
    );

    assert_eq!(
        clipboard.recent().rows[0].label,
        "word word word word word word word word word wo…"
    );
}

fn ids(clipboard: &Clipboard) -> Vec<i64> {
    clipboard.recent().rows.iter().map(|row| row.id).collect()
}

#[test]
fn a_new_clip_goes_on_top_and_the_list_keeps_ten() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(
        recording(State::Active),
        (1..=10).rev().map(|id| clip(id, "text", "old")).collect(),
    );

    clipboard.apply(Event::ClipAdded {
        clip: clip(11, "text", "new"),
    });

    assert_eq!(ids(&clipboard), vec![11, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
    assert_eq!(clipboard.recent().rows[0].label, "new");
}

#[test]
fn a_clip_copied_again_moves_up_instead_of_showing_twice() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(
        recording(State::Active),
        vec![
            clip(3, "text", "c"),
            clip(2, "text", "b"),
            clip(1, "text", "a"),
        ],
    );

    clipboard.apply(Event::ClipAdded {
        clip: clip(1, "text", "a"),
    });

    assert_eq!(ids(&clipboard), vec![1, 3, 2]);
}

#[test]
fn a_removed_clip_leaves_the_list() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(
        recording(State::Active),
        vec![clip(2, "text", "b"), clip(1, "text", "a")],
    );

    clipboard.apply(Event::ClipRemoved { clip: 2 });

    assert_eq!(ids(&clipboard), vec![1]);
}

#[test]
fn selection_changes_and_unknown_events_change_nothing() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(recording(State::Active), vec![clip(1, "text", "a")]);
    let before = (clipboard.view_state(&tz()), clipboard.recent());

    clipboard.apply(Event::Selection {
        selection: hoarder_ipc::Selection::Clipboard,
        types: None,
    });
    clipboard.apply(Event::Unknown);

    assert_eq!((clipboard.view_state(&tz()), clipboard.recent()), before);
}

#[test]
fn losing_the_daemon_goes_offline_and_forgets_its_clips() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(recording(State::Paused), vec![clip(1, "text", "a")]);

    clipboard.disconnected();

    assert_eq!(
        clipboard.view_state(&tz()),
        Clipboard::new().view_state(&tz())
    );
    assert_eq!(clipboard.recent(), Recent::default());
}

#[test]
fn events_without_a_daemon_are_ignored() {
    let mut clipboard = Clipboard::new();

    clipboard.apply(Event::State(recording(State::Paused)));
    clipboard.apply(Event::ClipAdded {
        clip: clip(1, "text", "a"),
    });

    assert_eq!(clipboard.recent(), Recent::default());
    assert_eq!(
        clipboard.view_state(&tz()).classes,
        vec!["zb-clipboard-offline"]
    );
}

#[test]
fn right_click_pauses_an_active_daemon_and_resumes_a_paused_one() {
    let mut clipboard = Clipboard::new();
    assert_eq!(clipboard.toggle_pause(), None);

    clipboard.connected(recording(State::Active), Vec::new());
    assert_eq!(clipboard.toggle_pause(), Some(Op::Pause { secs: None }));

    clipboard.apply(Event::State(recording(State::Paused)));
    assert_eq!(clipboard.toggle_pause(), Some(Op::Resume));
}

#[test]
fn reconnecting_backs_off_to_half_a_minute_and_starts_over_after_a_success() {
    let mut backoff = Backoff::default();

    let delays: Vec<u64> = (0..8).map(|_| backoff.after_failure().as_secs()).collect();
    backoff.reset();

    assert_eq!(delays, vec![1, 2, 4, 8, 16, 30, 30, 30]);
    assert_eq!(backoff.after_failure(), Duration::from_secs(1));
}

#[test]
fn a_later_list_replaces_the_clips_and_keeps_the_recording() {
    let mut clipboard = Clipboard::new();
    clipboard.connected(recording(State::Paused), Vec::new());

    clipboard.listed((1..=12).rev().map(|id| clip(id, "text", "x")).collect());

    assert_eq!(ids(&clipboard), vec![12, 11, 10, 9, 8, 7, 6, 5, 4, 3]);
    assert_eq!(
        clipboard.view_state(&tz()).classes,
        vec!["zb-clipboard-paused"]
    );
}

#[test]
fn a_list_without_a_daemon_is_ignored() {
    let mut clipboard = Clipboard::new();

    clipboard.listed(vec![clip(1, "text", "x")]);

    assert_eq!(clipboard.recent(), Recent::default());
}
