//! Where files live: flag, then environment variable, then the XDG default.

use std::env;
use std::path::{Path, PathBuf};

const DIR: &str = "zenith-bar";

pub fn config_file(flag: Option<&Path>) -> PathBuf {
    pick(flag, "ZENITH_BAR_CONFIG", || {
        config_dir().join("config.toml")
    })
}

pub fn style_file(flag: Option<&Path>) -> PathBuf {
    pick(flag, "ZENITH_BAR_STYLE", || config_dir().join("style.css"))
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join(DIR)
}

pub fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join(DIR)
}

fn pick(flag: Option<&Path>, var: &str, default: impl FnOnce() -> PathBuf) -> PathBuf {
    if let Some(path) = flag {
        return path.to_path_buf();
    }
    match env::var_os(var) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => default(),
    }
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match env::var_os(var) {
        Some(dir) if Path::new(&dir).is_absolute() => PathBuf::from(dir),
        _ => env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(fallback),
    }
}
