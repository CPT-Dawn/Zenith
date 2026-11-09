use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use gtk4::glib;
use zenith_bar::modules::todo::model::{Command, Priority};
use zenith_bar::modules::todo::source::TodoSource;

fn run_until_idle(ctx: &glib::MainContext) {
    for _ in 0..25 {
        while ctx.iteration(false) {}
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn an_unreadable_todo_file_is_never_overwritten() {
    let dir = std::env::temp_dir().join(format!("zenith-todo-{}-unreadable", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("todos.json");
    let original = "{\"version\": 1, \"items\": [{\"text\": \"precious\", \"done\": false}]}";
    fs::write(&path, original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads anything; nothing to prove there.
    if fs::read(&path).is_ok() {
        return;
    }

    let ctx = glib::MainContext::new();
    ctx.with_thread_default(|| {
        let source = TodoSource::start(path.clone());
        source.apply(Command::Add {
            text: "new".into(),
            priority: Priority::Normal,
        });
        run_until_idle(&ctx);
    })
    .unwrap();

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!dir.join("todos.json.corrupt").exists());
}

#[test]
fn a_second_corrupt_file_keeps_the_first_backup() {
    let dir = std::env::temp_dir().join(format!("zenith-todo-{}-two-corrupt", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("todos.json");
    let ctx = glib::MainContext::default();
    let _guard = ctx.acquire().unwrap();
    std::fs::write(&path, "first garbage").unwrap();
    let a = TodoSource::start(path.clone());
    run_until_idle(&ctx);
    drop(a);
    std::fs::write(&path, "second garbage").unwrap();
    let b = TodoSource::start(path.clone());
    run_until_idle(&ctx);
    drop(b);
    let kept: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path()).unwrap_or_default())
        .collect();
    assert!(
        kept.iter().any(|c| c == "first garbage"),
        "first backup lost: {kept:?}"
    );
}
