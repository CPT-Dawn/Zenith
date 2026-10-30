//! `zenith-bar check` and `zenith-bar init`, driven through the real binary.

use std::path::{Path, PathBuf};
use std::process::Output;

use pretty_assertions::assert_eq;

/// A fresh scratch directory per test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("zenith-bar-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn zenith_bar(args: &[&str], config: &Path) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_zenith-bar"))
        .args(args)
        .arg("--config")
        .arg(config)
        .env_remove("ZENITH_BAR_CONFIG")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn check_reports_a_misspelled_key_with_its_position_and_the_nearest_key() {
    let scratch = Scratch::new("check-typo");
    let config = scratch.file("config.toml");
    std::fs::write(&config, "[bar]\nheigth = 30\n\n[clock]\nfromat = \"%H\"\n").unwrap();

    let output = zenith_bar(&["check"], &config);

    let path = config.display();
    assert_eq!(
        stderr(&output),
        format!(
            "{path}:2:1: unknown key `heigth` in [bar] (did you mean `height`?)\n\
             {path}:5:1: unknown key `fromat` in [clock] (did you mean `format`?)\n\
             2 problems in {path}\n"
        )
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn check_accepts_a_valid_config() {
    let scratch = Scratch::new("check-ok");
    let config = scratch.file("config.toml");
    std::fs::write(&config, "[bar]\nheight = 30\n").unwrap();

    let output = zenith_bar(&["check"], &config);

    assert_eq!(stdout(&output), format!("{} is valid\n", config.display()));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn check_fails_when_the_config_is_missing() {
    let scratch = Scratch::new("check-missing");
    let config = scratch.file("nope.toml");

    let output = zenith_bar(&["check"], &config);

    assert!(
        stderr(&output).contains("cannot read"),
        "stderr: {}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn init_writes_a_commented_config_that_check_accepts() {
    let scratch = Scratch::new("init-roundtrip");
    let config = scratch.file("nested/dir/config.toml");

    let output = zenith_bar(&["init"], &config);
    assert_eq!(stdout(&output), format!("wrote {}\n", config.display()));
    assert_eq!(output.status.code(), Some(0));

    let text = std::fs::read_to_string(&config).unwrap();
    assert!(
        text.lines().any(|l| l.starts_with("# ")),
        "not commented:\n{text}"
    );
    assert!(text.contains("[bar]"), "no [bar] table:\n{text}");

    let output = zenith_bar(&["check"], &config);
    assert_eq!(stderr(&output), "");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn init_never_overwrites_without_force() {
    let scratch = Scratch::new("init-no-clobber");
    let config = scratch.file("config.toml");
    std::fs::write(&config, "# mine\n").unwrap();

    let output = zenith_bar(&["init"], &config);

    assert_eq!(
        stderr(&output),
        format!(
            "zenith-bar: {} already exists; use --force to overwrite it\n",
            config.display()
        )
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(&config).unwrap(), "# mine\n");
}

#[test]
fn init_with_force_replaces_the_config() {
    let scratch = Scratch::new("init-force");
    let config = scratch.file("config.toml");
    std::fs::write(&config, "# mine\n").unwrap();

    let output = zenith_bar(&["init", "--force"], &config);

    assert_eq!(output.status.code(), Some(0));
    assert!(std::fs::read_to_string(&config).unwrap().contains("[bar]"));
}
