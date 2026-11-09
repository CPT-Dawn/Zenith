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
