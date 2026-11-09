use std::fs;
use std::io::Read;
use std::path::PathBuf;

use gtk4::glib;
use pretty_assertions::assert_eq;
use zenith_bar::modules::todo::store;

/// A fresh, empty scratch directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("zenith-todo-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    glib::MainContext::new().block_on(future)
}

#[test]
fn saving_creates_the_data_directory_and_writes_the_file() {
    let path = scratch("creates").join("zenith-bar/nested/todos.json");

    block_on(store::save(&path, "{\"items\": []}\n".into())).expect("saved");

    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"items\": []}\n");
}

#[test]
fn reading_a_missing_file_is_none_not_an_error() {
    let path = scratch("missing").join("todos.json");

    assert_eq!(block_on(store::read(&path)).expect("read"), None);
}

#[test]
fn reading_returns_what_was_saved() {
    let path = scratch("roundtrip").join("todos.json");
    block_on(store::save(&path, "hello\n".into())).expect("saved");

    assert_eq!(
        block_on(store::read(&path)).expect("read"),
        Some("hello\n".into())
    );
}

#[test]
fn saves_replace_the_file_atomically_instead_of_rewriting_it_in_place() {
    let dir = scratch("atomic");
    let path = dir.join("todos.json");
    fs::write(&path, "old complete contents\n").unwrap();
    // A reader holding the old file open, like a crash mid-write would leave it.
    let mut old = fs::File::open(&path).unwrap();

    block_on(store::save(&path, "new contents\n".into())).expect("saved");

    // Rename swaps in a new file: the old one is untouched, never truncated or half-written.
    let mut seen = String::new();
    old.read_to_string(&mut seen).unwrap();
    assert_eq!(seen, "old complete contents\n");
    assert_eq!(fs::read_to_string(&path).unwrap(), "new contents\n");
    // No temp files are left behind.
    let names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, ["todos.json"]);
}

#[test]
fn a_failed_save_leaves_the_previous_file_intact() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("readonly");
    let path = dir.join("todos.json");
    fs::write(&path, "previous\n").unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    // Root ignores directory permissions; nothing to prove there.
    let writable = fs::File::create(dir.join("probe")).is_ok();

    let result = block_on(store::save(&path, "new\n".into()));

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    if writable {
        return;
    }
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "previous\n");
}

#[cfg(unix)]
#[test]
fn saving_through_a_symlink_keeps_the_link_and_updates_its_target() {
    use std::os::unix::fs::symlink;

    let dir = scratch("symlink");
    fs::create_dir_all(dir.join("real")).unwrap();
    let target = dir.join("real/todos.json");
    fs::write(&target, "old\n").unwrap();
    let link = dir.join("todos.json");
    symlink("real/todos.json", &link).unwrap();

    block_on(store::save(&link, "new\n".into())).expect("saved");

    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "new\n");
    assert!(!dir.join("real/.todos.json.tmp").exists());
}

#[test]
fn invalid_utf8_is_reported_as_corrupt_not_mangled() {
    let path = scratch("latin1").join("todos.json");
    fs::write(&path, b"caf\xe9").unwrap();

    let err = block_on(store::read(&path)).expect_err("not text");

    assert!(err.matches(gtk4::gio::IOErrorEnum::InvalidData), "{err}");
}

#[test]
fn a_utf8_bom_is_accepted_and_stripped() {
    let path = scratch("bom").join("todos.json");
    fs::write(&path, b"\xEF\xBB\xBF{\"items\": []}").unwrap();

    assert_eq!(
        block_on(store::read(&path)).expect("read"),
        Some("{\"items\": []}".into())
    );
}

#[test]
fn setting_aside_twice_keeps_both_backups() {
    let dir = scratch("aside");
    let path = dir.join("todos.json");
    fs::write(&path, "first").unwrap();
    block_on(store::set_aside(&path)).expect("first");
    fs::write(&path, "second").unwrap();
    block_on(store::set_aside(&path)).expect("second");

    assert_eq!(
        fs::read_to_string(dir.join("todos.json.corrupt")).unwrap(),
        "first"
    );
    assert_eq!(
        fs::read_to_string(dir.join("todos.json.corrupt.1")).unwrap(),
        "second"
    );
    assert!(!path.exists());
}
