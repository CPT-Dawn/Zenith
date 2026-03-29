use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Embedded default template copied to disk on first launch.
const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../Default_Config.toml");

/// Top-level configuration for Zenith bar.
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
#[derive(Default)]
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
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            monitor: None,
            height: 40,
            gap_horizontal: 8,
            gap_top: 6,
            border_radius: 12,
            border_width: 2,
            rgb_cycle_seconds: 4.0,
            background: "rgba(30, 30, 46, 0.6)".into(),
        }
    }
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            clock: true,
            clock_format: "%H:%M:%S".into(),
            system_stats: true,
            todo: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Return the canonical config path: `~/.config/zenith/config.toml`.
pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("Could not determine XDG config directory")?;
    Ok(config_dir.join("zenith").join("config.toml"))
}

/// Ensure the config directory and file exist.
///
/// If `config.toml` is missing, this writes `Default_Config.toml` as the
/// initial user-facing configuration.
fn ensure_config_file(path: &Path) -> Result<()> {
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

            file.write_all(DEFAULT_CONFIG_TEMPLATE.as_bytes())
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

/// Load configuration from `~/.config/zenith/config.toml`.
///
/// If the file does not exist, it is created from `Default_Config.toml`.
/// Individual missing keys still fall back to struct defaults via `serde`.
pub fn load() -> Result<ZenithConfig> {
    let path = config_path()?;
    ensure_config_file(&path)?;

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    let config: ZenithConfig =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

    log::info!("Loaded configuration from {}", path.display());
    Ok(config)
}
