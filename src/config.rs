use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table};

/// Embedded default template copied to disk on first launch.
const EMBEDDED_DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../Default_Config.toml");

/// Top-level configuration for Zenith bar.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ZenithConfig {
    pub bar: BarConfig,
    pub modules: ModulesConfig,
}

/// Configuration for bar geometry, positioning, and appearance.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct BarConfig {
    /// Monitor connector name to anchor to (e.g. "DP-1", "eDP-1").
    /// If `None`, anchors to the default/primary monitor.
    pub monitor: Option<String>,
    /// Bar height in pixels.
    pub height: i32,
    /// Horizontal gap (margin) from screen edges in pixels.
    pub gap_horizontal: i32,
    /// Vertical gap (margin) from the top edge in pixels.
    pub gap_top: i32,
    /// Corner radius for the inner bar surface (CSS `border-radius`).
    pub border_radius: i32,
    /// Width of the animated RGB border in pixels.
    pub border_width: i32,
    /// Duration of one full RGB animation cycle in seconds.
    pub rgb_cycle_seconds: f64,
    /// Inner bar background color as an `rgba(r,g,b,a)` CSS string.
    pub background: String,
}

/// Toggle individual bar modules on or off.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ModulesConfig {
    pub clock: bool,
    pub clock_format: String,
    pub system_stats: bool,
    pub todo: bool,
    pub playerctl: bool,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

impl Default for BarConfig {
    fn default() -> Self {
        // Keep in sync with `Default_Config.toml` (and the embedded template).
        Self {
            monitor: None,
            height: 32,
            gap_horizontal: 12,
            gap_top: 8,
            border_radius: 12,
            border_width: 1,
            rgb_cycle_seconds: 16.0,
            background: "rgba(26, 27, 38, 0.72)".into(),
        }
    }
}

impl Default for ModulesConfig {
    fn default() -> Self {
        // Keep in sync with `Default_Config.toml` (and the embedded template).
        Self {
            clock: true,
            clock_format: "%a %H:%M".into(),
            system_stats: true,
            todo: true,
            playerctl: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Return the canonical config directory: `~/.config/zenith`.
pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("Could not determine XDG config directory")?;
    Ok(base.join("zenith"))
}

/// Return the canonical config path: `~/.config/zenith/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ZENITH_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    Ok(config_dir()?.join("config.toml"))
}

/// Ensure the config directory and file exist.
///
/// If `config.toml` is missing, this writes `Default_Config.toml` as the
/// initial user-facing configuration.
fn ensure_config_file(path: &Path, default_template: &str) -> Result<()> {
    let parent = path
        .parent()
        .context("Config path has no parent directory")?;

    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create config directory {}", parent.display()))?;

    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;

            file.write_all(default_template.as_bytes())
                .with_context(|| format!("Failed to write default config to {}", path.display()))?;

            log::info!("Created default config at {}", path.display());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => {
            Err(err).with_context(|| format!("Failed to create config file {}", path.display()))
        }
    }
}

/// Merge missing keys from the embedded default config into the user config.
///
/// Existing user values are never overwritten. Only missing keys are injected.
fn sync_missing_default_keys(path: &Path, default_template: &str) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    let mut user_doc: DocumentMut = raw
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    let default_doc: DocumentMut = default_template
        .parse()
        .context("Failed to parse embedded default config template")?;

    let changed = merge_missing_items(default_doc.as_table(), user_doc.as_table_mut());
    if !changed {
        return Ok(());
    }

    let rendered = user_doc.to_string();

    // Atomic write: write to temp file, then rename.
    let tmp_path = path.with_extension("toml.tmp");
    fs::write(&tmp_path, rendered)
        .with_context(|| format!("Failed to write merged config at {}", tmp_path.display()))?;

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err).with_context(|| {
            format!(
                "Failed to replace config {} with merged defaults",
                path.display()
            )
        });
    }

    log::info!(
        "Config schema changed; injected missing keys into {}",
        path.display()
    );

    Ok(())
}

/// Recursively insert missing keys from `defaults` into `user` tables.
///
/// Returns `true` when at least one key was inserted.
fn merge_missing_items(defaults: &Table, user: &mut Table) -> bool {
    let mut changed = false;

    for (key, default_item) in defaults.iter() {
        match user.get_mut(key) {
            Some(user_item) => {
                changed |= merge_missing_item(default_item, user_item);
            }
            None => {
                user.insert(key, default_item.clone());
                changed = true;
            }
        }
    }

    changed
}

fn merge_missing_item(default_item: &Item, user_item: &mut Item) -> bool {
    if user_item.is_none() {
        *user_item = default_item.clone();
        return true;
    }

    match (default_item.as_table(), user_item.as_table_mut()) {
        (Some(default_table), Some(user_table)) => merge_missing_items(default_table, user_table),
        _ => false,
    }
}

/// Load configuration from `~/.config/zenith/config.toml`.
///
/// If the file does not exist, it is created from the embedded default template.
/// Zenith then reads only the user config file.
pub fn load() -> Result<ZenithConfig> {
    let path = config_path()?;
    ensure_config_file(&path, EMBEDDED_DEFAULT_CONFIG_TEMPLATE)?;
    sync_missing_default_keys(&path, EMBEDDED_DEFAULT_CONFIG_TEMPLATE)?;

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let config: ZenithConfig =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

    log::info!("Loaded configuration from {}", path.display());
    log::debug!("Loaded config values: {config:#?}");

    if let Some(override_path) = std::env::var_os("ZENITH_CONFIG") {
        log::info!(
            "ZENITH_CONFIG override active: {}",
            PathBuf::from(override_path).display()
        );
    }

    Ok(config)
}
