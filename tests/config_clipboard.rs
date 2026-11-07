use pretty_assertions::assert_eq;
use zenith_bar::config::resolve;
use zenith_bar::modules::ModuleId;

#[test]
fn the_clipboard_is_off_until_it_is_in_the_layout() {
    let config = resolve("").expect("defaults are valid");

    assert!(!config.layout.contains(ModuleId::Clipboard));
}

#[test]
fn the_clipboard_can_go_in_any_section() {
    let config = resolve("[layout]\nright = [\"clipboard\", \"system\"]\n").expect("valid config");

    assert_eq!(
        config.layout.right,
        vec![ModuleId::Clipboard, ModuleId::System]
    );
}

#[test]
fn search_all_runs_the_hoarder_picker_by_default() {
    let config = resolve("").expect("defaults are valid");

    assert_eq!(config.clipboard.search_command, "hoarder picker");
}

#[test]
fn every_clipboard_key_can_be_set() {
    let text = "[clipboard]\nsearch_command = \"~/bin/clips\"\non_right_click = \"\"\n";

    let config = resolve(text).expect("valid config");

    assert_eq!(config.clipboard.search_command, "~/bin/clips");
    assert_eq!(config.clipboard.actions.on_right_click.as_deref(), Some(""));
}
