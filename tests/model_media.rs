use std::time::Duration;

use pretty_assertions::assert_eq;
use zenith_bar::modules::ViewState;
use zenith_bar::modules::media::{Media, MediaConfig, PropertyMap, Value};

fn at(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

fn s(text: &str) -> Value {
    Value::Str(text.into())
}

/// `Metadata` as players send it: `xesam:artist` is a string list, `mpris:length` microseconds.
fn metadata(title: &str, artists: &[&str], length_secs: i64) -> Value {
    let mut map = PropertyMap::new();
    map.insert("mpris:trackid".into(), s(&format!("/track/{title}")));
    if !title.is_empty() {
        map.insert("xesam:title".into(), s(title));
    }
    if !artists.is_empty() {
        map.insert(
            "xesam:artist".into(),
            Value::StrList(artists.iter().map(|a| a.to_string()).collect()),
        );
    }
    if length_secs > 0 {
        map.insert("mpris:length".into(), Value::I64(length_secs * 1_000_000));
    }
    Value::Map(map)
}

/// A `GetAll(org.mpris.MediaPlayer2.Player)` reply.
fn player(status: &str, metadata: Value, position_secs: i64) -> PropertyMap {
    PropertyMap::from([
        ("PlaybackStatus".into(), s(status)),
        ("Metadata".into(), metadata),
        ("Position".into(), Value::I64(position_secs * 1_000_000)),
        ("Rate".into(), Value::F64(1.0)),
    ])
}

fn changed(entries: &[(&str, Value)]) -> PropertyMap {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn config() -> MediaConfig {
    MediaConfig::default()
}

#[test]
fn with_no_players_it_says_no_media_and_never_wakes() {
    let media = Media::new();

    assert_eq!(
        media.view_state(&MediaConfig::default(), at(0)),
        ViewState {
            label: "No media".into(),
            icon: Some("zb-media-symbolic".into()),
            visible: true,
            ..ViewState::default()
        }
    );
}

#[test]
fn hide_when_idle_hides_the_module_when_nothing_plays() {
    let config = MediaConfig {
        hide_when_idle: true,
        ..MediaConfig::default()
    };

    let state = Media::new().view_state(&config, at(0));

    assert!(!state.visible);
    assert_eq!(state.next_wake, None);
}

#[test]
fn a_playing_player_shows_title_and_artist() {
    let mut media = Media::new();
    media.player_added(
        ":1.42",
        &player("Playing", metadata("Blue Monday", &["New Order"], 448), 0),
        at(0),
    );

    let state = media.view_state(&config(), at(0));

    assert_eq!(state.label, "Blue Monday - New Order");
    assert_eq!(state.icon.as_deref(), Some("zb-media-play-symbolic"));
    assert_eq!(state.classes, vec!["zb-playing"]);
    assert!(state.visible);
}

fn label_for(format: &str, metadata: Value) -> String {
    let mut media = Media::new();
    media.player_added(":1.1", &player("Playing", metadata, 0), at(0));
    let config = MediaConfig {
        format: format.into(),
        ..config()
    };
    media.view_state(&config, at(0)).label
}

fn with_album(metadata: Value, album: &str) -> Value {
    let Value::Map(mut map) = metadata else {
        unreachable!()
    };
    map.insert("xesam:album".into(), s(album));
    Value::Map(map)
}

#[test]
fn the_format_places_title_artist_and_album() {
    let track = with_album(metadata("Heroes", &["David Bowie"], 371), "\"Heroes\"");

    assert_eq!(
        label_for("{artist} · {title} ({album})", track),
        "David Bowie · Heroes (\"Heroes\")"
    );
}

#[test]
fn several_artists_are_joined_with_commas() {
    let track = metadata("Under Pressure", &["Queen", "David Bowie"], 248);

    assert_eq!(
        label_for("{title} - {artist}", track),
        "Under Pressure - Queen, David Bowie"
    );
}

#[test]
fn the_separator_next_to_a_missing_artist_is_dropped() {
    let track = metadata("Untitled", &[], 60);

    assert_eq!(label_for("{title} - {artist}", track.clone()), "Untitled");
    assert_eq!(label_for("{artist} - {title}", track), "Untitled");
}

#[test]
fn a_missing_field_in_the_middle_keeps_the_brackets_of_the_next_one() {
    let track = with_album(metadata("Song", &[], 60), "Album");

    assert_eq!(
        label_for("{title} - {artist} ({album})", track),
        "Song (Album)"
    );
}

#[test]
fn a_missing_last_field_takes_its_brackets_with_it() {
    let track = metadata("Song", &["Band"], 60);

    assert_eq!(
        label_for("{title} - {artist} ({album})", track),
        "Song - Band"
    );
}

#[test]
fn text_around_the_whole_label_stays() {
    assert_eq!(
        label_for("♪ {title} ♪", metadata("Song", &[], 60)),
        "♪ Song ♪"
    );
}

#[test]
fn a_track_without_a_title_is_unknown() {
    assert_eq!(
        label_for("{title} - {artist}", metadata("", &["Band"], 60)),
        "Unknown title - Band"
    );
}

fn state_with_max(max_length: u32, title: &str) -> ViewState {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata(title, &[], 60), 0),
        at(0),
    );
    let config = MediaConfig {
        format: "{title}".into(),
        max_length,
        ..config()
    };
    media.view_state(&config, at(0))
}

#[test]
fn a_long_label_is_cut_to_max_length_with_an_ellipsis_and_shown_whole_in_the_tooltip() {
    let state = state_with_max(11, "Bohemian Rhapsody");

    assert_eq!(state.label, "Bohemian R…");
    assert_eq!(state.tooltip.as_deref(), Some("Bohemian Rhapsody"));
}

#[test]
fn the_cut_does_not_leave_a_space_before_the_ellipsis() {
    assert_eq!(state_with_max(10, "Bohemian Rhapsody").label, "Bohemian…");
}

#[test]
fn truncation_counts_characters_not_bytes() {
    assert_eq!(state_with_max(4, "Sigur Rós – Hoppípolla").label, "Sig…");
    assert_eq!(state_with_max(6, "Hoppípolla").label, "Hoppí…");
}

#[test]
fn a_label_that_fits_is_left_alone() {
    let state = state_with_max(10, "Heroes");

    assert_eq!(state.label, "Heroes");
    assert_eq!(state.tooltip.as_deref(), Some("Heroes"));
}

#[test]
fn max_length_zero_never_truncates() {
    let title = "A Very Long Title That Goes On And On And On And On";

    assert_eq!(state_with_max(0, title).label, title);
}

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[test]
fn progress_is_estimated_from_the_last_known_position() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 120), 30),
        at(100),
    );

    let state = media.view_state(&config(), at(115));

    assert_eq!(state.progress, Some(0.375));
}

#[test]
fn progress_follows_the_playback_rate() {
    let mut properties = player("Playing", metadata("Podcast", &[], 100), 10);
    properties.insert("Rate".into(), Value::F64(2.0));
    let mut media = Media::new();
    media.player_added(":1.1", &properties, at(0));

    assert_eq!(media.view_state(&config(), at(20)).progress, Some(0.5));
}

#[test]
fn before_the_progress_bar_is_measured_it_wakes_once_per_two_hundredth_of_the_track() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 200), 0),
        at(0),
    );

    assert_eq!(media.view_state(&config(), at(0)).next_wake, Some(at(1)));
}

#[test]
fn the_tick_moves_the_progress_bar_by_one_pixel_of_its_width() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 200), 0),
        at(0),
    );

    media.set_progress_width(100);

    assert_eq!(media.view_state(&config(), at(0)).next_wake, Some(at(2)));
}

#[test]
fn a_wide_progress_bar_still_ticks_no_faster_than_four_times_a_second() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 200), 0),
        at(0),
    );

    media.set_progress_width(4000);

    assert_eq!(media.view_state(&config(), at(0)).next_wake, Some(ms(250)));
}

#[test]
fn a_hidden_progress_bar_keeps_the_last_width_it_had() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 200), 0),
        at(0),
    );

    media.set_progress_width(50);
    media.set_progress_width(0);

    assert_eq!(media.view_state(&config(), at(0)).next_wake, Some(at(4)));
}

#[test]
fn short_tracks_tick_no_faster_than_four_times_a_second() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Jingle", &[], 10), 0),
        at(0),
    );

    assert_eq!(media.view_state(&config(), at(0)).next_wake, Some(ms(250)));
}

#[test]
fn progress_stops_at_the_end_of_the_track_and_so_do_the_wakes() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 60), 50),
        at(0),
    );

    let state = media.view_state(&config(), at(30));

    assert_eq!(state.progress, Some(1.0));
    assert_eq!(state.next_wake, None);
}

#[test]
fn a_track_of_unknown_length_has_no_progress_and_never_wakes() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Radio", &[], 0), 0),
        at(0),
    );

    let state = media.view_state(&config(), at(5));

    assert_eq!(state.progress, None);
    assert_eq!(state.next_wake, None);
}

#[test]
fn pausing_freezes_progress_and_stops_every_wake() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 100), 0),
        at(0),
    );
    media.properties_changed(":1.1", &changed(&[("PlaybackStatus", s("Paused"))]), at(25));

    let paused = media.view_state(&config(), at(25));
    let much_later = media.view_state(&config(), at(1000));

    assert_eq!(paused.label, "Song");
    assert_eq!(paused.classes, vec!["zb-paused"]);
    assert_eq!(paused.icon.as_deref(), Some("zb-media-pause-symbolic"));
    assert_eq!(paused.progress, Some(0.25));
    assert_eq!(paused.next_wake, None);
    assert_eq!(much_later, paused);
}

#[test]
fn resuming_continues_from_where_it_paused() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Paused", metadata("Song", &[], 100), 40),
        at(0),
    );
    media.properties_changed(
        ":1.1",
        &changed(&[("PlaybackStatus", s("Playing"))]),
        at(500),
    );

    assert_eq!(media.view_state(&config(), at(510)).progress, Some(0.5));
}

#[test]
fn a_stopped_player_counts_as_nothing_playing() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 100), 0),
        at(0),
    );
    media.properties_changed(":1.1", &changed(&[("PlaybackStatus", s("Stopped"))]), at(5));

    let shown = media.view_state(&config(), at(5));
    let hidden = media.view_state(
        &MediaConfig {
            hide_when_idle: true,
            ..config()
        },
        at(5),
    );

    assert_eq!(shown.label, "No media");
    assert_eq!(shown.classes, Vec::<&str>::new());
    assert_eq!(shown.progress, None);
    assert_eq!(shown.next_wake, None);
    assert!(!hidden.visible);
}

#[test]
fn a_track_change_updates_the_label_and_restarts_progress() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("One", &["A"], 100), 0),
        at(0),
    );
    media.properties_changed(
        ":1.1",
        &changed(&[("Metadata", metadata("Two", &["B"], 200))]),
        at(90),
    );

    let state = media.view_state(&config(), at(140));

    assert_eq!(state.label, "Two - B");
    assert_eq!(state.progress, Some(0.25));
}

#[test]
fn new_metadata_for_the_same_song_does_not_restart_progress() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("One", &["A"], 100), 0),
        at(0),
    );
    media.properties_changed(
        ":1.1",
        &changed(&[("Metadata", metadata("One", &["A"], 100))]),
        at(50),
    );

    assert_eq!(media.view_state(&config(), at(50)).progress, Some(0.5));
}

#[test]
fn seeking_moves_the_estimate() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 100), 0),
        at(0),
    );
    media.seeked(":1.1", 80_000_000, at(10));

    assert_eq!(media.view_state(&config(), at(15)).progress, Some(0.85));
}

fn status(name: &str) -> PropertyMap {
    changed(&[("PlaybackStatus", s(name))])
}

#[test]
fn the_player_that_is_playing_wins_over_a_newer_paused_one() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Spotify song", &[], 100), 0),
        at(0),
    );
    media.player_added(
        ":1.2",
        &player("Paused", metadata("Browser video", &[], 100), 0),
        at(1),
    );

    assert_eq!(media.view_state(&config(), at(2)).label, "Spotify song");
    assert_eq!(media.active_player(), Some(":1.1"));
}

#[test]
fn the_player_that_started_playing_last_wins() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("First", &[], 100), 0),
        at(0),
    );
    media.player_added(
        ":1.2",
        &player("Paused", metadata("Second", &[], 100), 0),
        at(1),
    );
    media.properties_changed(":1.2", &status("Playing"), at(2));

    assert_eq!(media.view_state(&config(), at(3)).label, "Second");
}

#[test]
fn with_nothing_playing_the_most_recently_active_player_is_followed() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Paused", metadata("Old", &[], 100), 0),
        at(0),
    );
    media.player_added(
        ":1.2",
        &player("Playing", metadata("Recent", &[], 100), 0),
        at(1),
    );
    media.properties_changed(":1.2", &status("Paused"), at(2));
    media.properties_changed(":1.1", &changed(&[("Rate", Value::F64(1.0))]), at(3));

    assert_eq!(media.view_state(&config(), at(4)).label, "Recent");
    assert_eq!(media.active_player(), Some(":1.2"));
}

#[test]
fn stopped_players_are_never_followed() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Paused", metadata("Paused one", &[], 100), 0),
        at(0),
    );
    media.player_added(":1.2", &player("Stopped", metadata("", &[], 0), 0), at(1));

    assert_eq!(media.view_state(&config(), at(2)).label, "Paused one");
}

#[test]
fn when_the_active_player_disappears_the_next_one_takes_over() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Paused", metadata("Fallback", &[], 100), 0),
        at(0),
    );
    media.player_added(
        ":1.2",
        &player("Playing", metadata("Main", &[], 100), 0),
        at(1),
    );
    media.player_removed(":1.2");

    let state = media.view_state(&config(), at(2));

    assert_eq!(state.label, "Fallback");
    assert_eq!(state.next_wake, None);
}

#[test]
fn when_the_last_player_disappears_it_goes_back_to_no_media() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Song", &[], 100), 0),
        at(0),
    );
    media.player_removed(":1.1");

    assert_eq!(media.view_state(&config(), at(1)).label, "No media");
    assert_eq!(media.active_player(), None);
}

#[test]
fn a_restarted_player_is_picked_up_again() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Before", &[], 100), 0),
        at(0),
    );
    media.player_removed(":1.1");
    media.player_added(
        ":1.9",
        &player("Playing", metadata("After", &[], 100), 0),
        at(5),
    );

    assert_eq!(media.view_state(&config(), at(6)).label, "After");
    assert_eq!(media.active_player(), Some(":1.9"));
}

#[test]
fn signals_from_unknown_or_removed_players_are_ignored() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Paused", metadata("Song", &[], 100), 0),
        at(0),
    );
    media.player_removed(":1.2");
    media.properties_changed(":1.2", &status("Playing"), at(1));
    media.seeked(":1.2", 5_000_000, at(1));

    assert_eq!(
        media.view_state(&config(), at(2)).classes,
        vec!["zb-paused"]
    );
}

#[test]
fn adding_the_same_player_twice_replaces_it() {
    let mut media = Media::new();
    media.player_added(
        ":1.1",
        &player("Playing", metadata("Old", &[], 100), 0),
        at(0),
    );
    media.player_added(
        ":1.1",
        &player("Playing", metadata("New", &[], 100), 0),
        at(1),
    );
    media.player_removed(":1.1");

    assert_eq!(media.view_state(&config(), at(2)).label, "No media");
}
