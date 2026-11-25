//! The cockpit source with fake feed commands (plain `sh`), on a glib main loop as on
//! the bar.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use gtk4::glib;
use pretty_assertions::assert_eq;
use zenith_bar::modules::cockpit::model::{FeedState, Tone};
use zenith_bar::modules::cockpit::source::CockpitSource;
use zenith_bar::modules::cockpit::{CockpitConfig, FeedConfig};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cockpit-feed.json"
);

static MAIN_CONTEXT: Mutex<()> = Mutex::new(());

fn on_main_loop(test: impl FnOnce(&glib::MainContext)) {
    let _turn = MAIN_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
    let context = glib::MainContext::default();
    let _owner = context.acquire().expect("own the default main context");
    test(&context);
    while context.iteration(false) {}
}

fn wait_until(context: &glib::MainContext, what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

fn config(command: &str, timeout: u32) -> CockpitConfig {
    CockpitConfig {
        feeds: vec![FeedConfig {
            command: command.into(),
            interval: 300,
            timeout,
        }],
        ..CockpitConfig::default()
    }
}

fn first(source: &CockpitSource) -> FeedState {
    source.feeds().with(|f| f[0].clone())
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("zb-cockpit-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_feed_publishes_its_document_and_chips() {
    on_main_loop(|context| {
        let source = CockpitSource::start(config(&format!("cat '{FIXTURE}'"), 10));
        wait_until(context, "the feed lands", || first(&source).last.is_some());

        let state = first(&source);
        assert_eq!(state.failure, None);
        assert_eq!(state.last.unwrap().sections[1].title, "Codex");
        let chips = source.chips().with(Clone::clone);
        let shown: Vec<(String, String, Tone)> = chips
            .iter()
            .map(|c| (c.label.clone(), c.text.clone(), c.tone))
            .collect();
        assert_eq!(shown[1], ("Codex".into(), "87%".into(), Tone::Critical));
        // No repos: the runner fleet stays out of it, and so does the module state.
        assert_eq!(source.state().with(|s| s.tooltip.clone()), None);
    });
}

#[test]
fn a_feed_that_exits_non_zero_fails_with_its_last_stderr_line() {
    on_main_loop(|context| {
        let source = CockpitSource::start(config("echo boom >&2; exit 3", 10));
        wait_until(context, "the run fails", || {
            first(&source).failure.is_some()
        });

        assert_eq!(
            first(&source).failure.as_deref(),
            Some("exited with status 3: boom")
        );
        assert!(source.chips().with(|c| c[0].failed));
    });
}

#[test]
fn a_feed_that_overruns_is_killed_with_its_children() {
    let dir = scratch("timeout");
    let pid_file = dir.join("child.pid");
    let command = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
    on_main_loop(|context| {
        let started = Instant::now();
        let source = CockpitSource::start(config(&command, 1));
        wait_until(context, "the run times out", || {
            first(&source).failure.is_some()
        });

        assert_eq!(
            first(&source).failure.as_deref(),
            Some("timed out after 1 s")
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        let child = std::fs::read_to_string(&pid_file).unwrap();
        let stat = format!("/proc/{}/stat", child.trim());
        wait_until(context, "the child is gone", || {
            std::fs::read_to_string(&stat).map_or(true, |s| s.contains(") Z "))
        });
    });
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_bad_run_keeps_the_last_good_document() {
    let dir = scratch("keep");
    let feed = dir.join("feed.json");
    std::fs::copy(FIXTURE, &feed).unwrap();
    let command = format!("cat '{}'", feed.display());
    on_main_loop(|context| {
        let source = CockpitSource::start(config(&command, 10));
        wait_until(context, "the feed lands", || first(&source).last.is_some());
        let good = first(&source).last;

        std::fs::write(&feed, "{\"version\": 7}").unwrap();
        // Same command, new interval: it runs again now and keeps what it showed.
        let mut again = config(&command, 10);
        again.feeds[0].interval = 301;
        source.reconfigure(again);
        wait_until(context, "the second run fails", || {
            first(&source).failure.is_some()
        });

        let state = first(&source);
        assert_eq!(
            state.failure.as_deref(),
            Some("feed version 7 is not supported (this zenith-bar reads version 1)")
        );
        assert_eq!(state.last, good);
        let chips = source.chips().with(Clone::clone);
        assert_eq!(chips.len(), 4, "three glance chips and the `!`");
        assert!(chips[3].failed);
    });
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_changed_command_starts_fresh() {
    on_main_loop(|context| {
        let source = CockpitSource::start(config(&format!("cat '{FIXTURE}'"), 10));
        wait_until(context, "the feed lands", || first(&source).last.is_some());

        source.reconfigure(config("exit 1", 10));
        assert_eq!(first(&source).last, None);
        wait_until(context, "the new command fails", || {
            first(&source).failure.is_some()
        });
        assert_eq!(first(&source).command, "exit 1");

        source.reconfigure(CockpitConfig::default());
        assert_eq!(source.feeds().with(Vec::len), 0);
        assert_eq!(source.chips().with(Vec::len), 0);
    });
}
