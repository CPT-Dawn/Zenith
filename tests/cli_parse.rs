use pretty_assertions::assert_eq;
use std::path::PathBuf;
use zenith_bar::cli::{Command, Options, parse};
use zenith_bar::config::Preset;

fn args(list: &[&str]) -> Result<Command, String> {
    parse(list.iter().map(|s| s.to_string()))
}

#[test]
fn check_takes_the_config_flag() {
    assert_eq!(
        args(&["check", "-c", "my.toml"]),
        Ok(Command::Check(Options {
            config: Some(PathBuf::from("my.toml")),
            style: None,
        }))
    );
}

#[test]
fn init_does_not_overwrite_unless_forced() {
    assert_eq!(
        args(&["init"]),
        Ok(Command::Init {
            options: Options::default(),
            preset: None,
            force: false,
        })
    );
    assert_eq!(
        args(&["init", "--force", "--config=x.toml"]),
        Ok(Command::Init {
            options: Options {
                config: Some(PathBuf::from("x.toml")),
                style: None,
            },
            preset: None,
            force: true,
        })
    );
}

#[test]
fn init_takes_a_preset_by_name() {
    for (list, preset) in [
        (&["init", "--preset", "minimal"][..], Preset::Minimal),
        (&["init", "--preset=bloom"][..], Preset::Bloom),
        (
            &["init", "--force", "--preset", "classic"][..],
            Preset::Classic,
        ),
    ] {
        assert!(
            matches!(args(list), Ok(Command::Init { preset: Some(p), .. }) if p == preset),
            "{list:?}"
        );
    }
}

#[test]
fn an_unknown_preset_lists_the_choices() {
    assert_eq!(
        args(&["init", "--preset", "glassy"]),
        Err("unknown preset `glassy` (choose bloom, minimal or classic)".into())
    );
    assert_eq!(
        args(&["init", "--preset"]),
        Err("--preset needs a name (bloom, minimal or classic)".into())
    );
}

#[test]
fn preset_is_only_for_init() {
    assert_eq!(
        args(&["check", "--preset", "bloom"]),
        Err("--preset only applies to `init`".into())
    );
}

#[test]
fn force_is_only_for_init() {
    assert_eq!(
        args(&["check", "--force"]),
        Err("--force only applies to `init`".into())
    );
    assert_eq!(
        args(&["--force"]),
        Err("--force only applies to `init`".into())
    );
}

#[test]
fn no_subcommand_runs_the_bar() {
    assert_eq!(args(&[]), Ok(Command::Run(Options::default())));
}
