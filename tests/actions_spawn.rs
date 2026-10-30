//! Click commands run detached: spawning returns at once and never waits on the child.

use std::time::{Duration, Instant};

use zenith_bar::ui::actions::spawn;

#[test]
fn a_hanging_command_does_not_block_the_caller() {
    let dir = std::env::temp_dir().join(format!("zb-actions-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let marker = dir.join("ran");
    let _ = std::fs::remove_file(&marker);

    let started = Instant::now();
    spawn("sleep 5");
    spawn(&format!("touch '{}'", marker.display()));
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "spawning blocked for {:?}",
        started.elapsed()
    );

    // The main loop keeps turning while the sleeper hangs, and the other command runs.
    let context = gtk4::glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() && Instant::now() < deadline {
        context.iteration(false);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(marker.exists(), "the command never ran");
    std::fs::remove_dir_all(&dir).unwrap();
}
