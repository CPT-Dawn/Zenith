use pretty_assertions::assert_eq;
use zenith_bar::config::resolve;
use zenith_bar::modules::ModuleId;
use zenith_bar::modules::cockpit::FeedConfig;

fn messages(text: &str) -> Vec<String> {
    resolve(text)
        .expect_err("invalid config")
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn the_cockpit_is_off_and_empty_by_default() {
    let config = resolve("").expect("defaults are valid");

    assert!(!config.layout.contains(ModuleId::Cockpit));
    assert_eq!(config.cockpit.feeds, Vec::new());
    assert_eq!(config.cockpit.runners.repos, Vec::<String>::new());
    assert_eq!(config.cockpit.runners.interval, 30);
    assert_eq!(config.warnings, Vec::new());
}

#[test]
fn feeds_and_runner_keys_share_the_table() {
    let text = r#"
[layout]
left = ["clipboard", "cockpit"]

[cockpit]
repos = ["CPT-Dawn/PeekaBus"]
expected = 7
on_right_click = ""

[[cockpit.feeds]]
command = "cockpit-feed"

[[cockpit.feeds]]
command = "other --json"
interval = 60
timeout = 5
"#;
    let config = resolve(text).expect("valid config");

    assert_eq!(config.layout.left, [ModuleId::Clipboard, ModuleId::Cockpit]);
    assert_eq!(config.cockpit.runners.repos, ["CPT-Dawn/PeekaBus"]);
    assert_eq!(config.cockpit.runners.expected, 7);
    assert_eq!(
        config.actions(ModuleId::Cockpit).on_right_click.as_deref(),
        Some("")
    );
    assert_eq!(
        config.cockpit.feeds,
        [
            FeedConfig {
                command: "cockpit-feed".into(),
                interval: 15,
                timeout: 30,
            },
            FeedConfig {
                command: "other --json".into(),
                interval: 60,
                timeout: 5,
            },
        ]
    );
    assert_eq!(config.warnings, Vec::new());
}

#[test]
fn bad_feeds_are_named_by_index_at_their_key() {
    let text = "[cockpit]\n\n[[cockpit.feeds]]\ncommand = \"ok\"\n\n[[cockpit.feeds]]\ncommand = \" \"\ninterval = 3\ntimeout = 0\n";
    let diagnostics = resolve(text).expect_err("invalid");
    let found: Vec<(usize, String)> = diagnostics
        .into_iter()
        .map(|d| (d.line, d.message))
        .collect();
    assert_eq!(
        found,
        [
            (7, "feeds[1]: command must not be empty".to_string()),
            (
                8,
                "feeds[1]: interval must be at least 5 seconds".to_string()
            ),
            (9, "feeds[1]: timeout must be at least 1 second".to_string()),
        ]
    );
}

#[test]
fn unknown_keys_in_a_feed_are_flagged() {
    assert_eq!(
        messages("[[cockpit.feeds]]\ncommand = \"x\"\nintervall = 20\n"),
        ["unknown key `intervall` in [cockpit.feeds[0]]"]
    );
    assert_eq!(
        messages("[cockpit]\nfeeds = [{ command = \"x\", tmeout = 3 }]\n"),
        ["unknown key `tmeout` in [cockpit.feeds[0]]"]
    );
}

#[test]
fn the_runner_keys_are_validated_in_the_cockpit_too() {
    assert_eq!(
        messages("[cockpit]\ninterval = 5\n"),
        ["interval must be at least 10 seconds"]
    );
}

#[test]
fn runners_still_works_for_a_release_with_warnings() {
    let text = r#"[layout]
left = ["clipboard", "runners"]

[runners]
repos = ["CPT-Dawn/PeekaBus"]
expected = 7
"#;
    let config = resolve(text).expect("the alias is valid");

    assert_eq!(config.layout.left, [ModuleId::Clipboard, ModuleId::Runners]);
    assert_eq!(config.cockpit.runners.repos, ["CPT-Dawn/PeekaBus"]);
    assert_eq!(config.cockpit.runners.expected, 7);
    assert_eq!(config.cockpit.feeds, Vec::new());
    let warnings: Vec<(usize, String)> = config
        .warnings
        .into_iter()
        .map(|w| (w.line, w.message))
        .collect();
    assert_eq!(
        warnings,
        [
            (
                2,
                "module `runners` is deprecated and goes in the next release; use `cockpit`"
                    .to_string()
            ),
            (
                4,
                "[runners] is deprecated and goes in the next release; rename it to [cockpit]"
                    .to_string()
            ),
        ]
    );
}

#[test]
fn the_old_and_new_names_cannot_be_mixed() {
    assert_eq!(
        messages("[runners]\nexpected = 1\n\n[cockpit]\nexpected = 2\n"),
        ["[runners] is the old name of [cockpit]; move its keys into [cockpit]"]
    );
    assert_eq!(
        messages("[layout]\nleft = [\"cockpit\"]\nright = [\"runners\"]\n"),
        ["`runners` is the old name of `cockpit`; keep only `cockpit`"]
    );
}
