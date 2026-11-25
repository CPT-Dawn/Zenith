use pretty_assertions::assert_eq;
use zenith_bar::config::resolve;
use zenith_bar::modules::ModuleId;

#[test]
fn runners_are_off_until_they_are_in_the_layout() {
    let config = resolve("").expect("defaults are valid");

    assert!(!config.layout.contains(ModuleId::Runners));
    assert_eq!(config.cockpit.runners.repos, Vec::<String>::new());
    assert_eq!(config.cockpit.runners.interval, 30);
    assert_eq!(config.cockpit.runners.token_command, "gh auth token");
    assert_eq!(config.cockpit.runners.api_url, "https://api.github.com");
}

#[test]
fn every_runners_key_can_be_set() {
    let text = r#"
[layout]
left = ["clipboard", "runners"]

[runners]
repos = ["CPT-Dawn/PeekaBus", "acme/web.site"]
expected = 7
interval = 60
token_command = "pass github/token"
api_url = "https://ghe.example.com/api/v3"
on_right_click = ""
"#;
    let config = resolve(text).expect("valid config");

    assert_eq!(
        config.layout.left,
        vec![ModuleId::Clipboard, ModuleId::Runners]
    );
    assert_eq!(
        config.cockpit.runners.repos,
        ["CPT-Dawn/PeekaBus", "acme/web.site"]
    );
    assert_eq!(config.cockpit.runners.expected, 7);
    assert_eq!(config.cockpit.runners.interval, 60);
    assert_eq!(config.cockpit.runners.token_command, "pass github/token");
    assert_eq!(
        config.cockpit.runners.api_url,
        "https://ghe.example.com/api/v3"
    );
    assert_eq!(
        config.cockpit.runners.actions.on_right_click.as_deref(),
        Some("")
    );
}

fn messages(text: &str) -> Vec<String> {
    resolve(text)
        .expect_err("invalid config")
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_repo_must_be_owner_slash_name() {
    assert_eq!(
        messages("[runners]\nrepos = [\"PeekaBus\"]\n"),
        ["`PeekaBus` is not a repository; write it as \"owner/name\""]
    );
}

#[test]
fn polling_faster_than_every_ten_seconds_is_refused() {
    assert_eq!(
        messages("[runners]\ninterval = 5\n"),
        ["interval must be at least 10 seconds"]
    );
}

#[test]
fn the_token_command_and_api_url_must_be_usable() {
    assert_eq!(
        messages("[runners]\ntoken_command = \" \"\napi_url = \"api.github.com\"\n"),
        [
            "token_command must not be empty",
            "api_url must start with https:// or http://",
        ]
    );
}
