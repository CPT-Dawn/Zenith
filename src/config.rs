use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Embedded default template copied to disk on first launch.
const EMBEDDED_DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../Default_Config.toml");

/// Top-level configuration for Zenith bar.
#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZenithConfig {
    pub bar: BarConfig,
    pub modules: ModulesConfig,
}

/// Configuration for bar geometry, positioning, and appearance.
#[derive(Debug, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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

/// Resolve the default-config template text.
///
/// Order of precedence:
/// 1) `ZENITH_DEFAULT_CONFIG_TEMPLATE` path, if set and readable.
/// 2) `./Default_Config.toml` from current working directory, if readable.
/// 3) Embedded compile-time template fallback.
fn default_config_template() -> String {
    if let Some(path) = std::env::var_os("ZENITH_DEFAULT_CONFIG_TEMPLATE") {
        let path = PathBuf::from(path);
        match fs::read_to_string(&path) {
            Ok(content) => {
                log::info!(
                    "Using runtime default config template from {}",
                    path.display()
                );
                return content;
            }
            Err(err) => {
                log::warn!(
                    "Failed to read ZENITH_DEFAULT_CONFIG_TEMPLATE at {}: {err}; falling back",
                    path.display()
                );
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("Default_Config.toml");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    log::info!(
                        "Using runtime default config template from {}",
                        path.display()
                    );
                    return content;
                }
                Err(err) => {
                    log::warn!(
                        "Failed to read runtime default config template at {}: {err}; falling back",
                        path.display()
                    );
                }
            }
        }
    }

    EMBEDDED_DEFAULT_CONFIG_TEMPLATE.to_string()
}

/// Merge `overlay` into `base`, replacing scalar/array values and recursively
/// merging tables.
fn merge_toml_value(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                if let Some(base_value) = base_table.get_mut(&key) {
                    merge_toml_value(base_value, value);
                } else {
                    base_table.insert(key, value);
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
}

/// Load configuration from `~/.config/zenith/config.toml`.
///
/// If the file does not exist, it is created from `Default_Config.toml`.
/// Missing keys in the user config fall back to values from the default
/// template so template edits are applied consistently.
pub fn load() -> Result<ZenithConfig> {
    let default_template = default_config_template();

    let path = config_path()?;
    ensure_config_file(&path, &default_template)?;

    if let Ok(cwd) = std::env::current_dir() {
        let local_cfg = cwd.join("config.toml");
        if local_cfg.exists() && local_cfg != path {
            log::warn!(
                "Ignoring local config at {}; using {}",
                local_cfg.display(),
                path.display()
            );
        }
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    let mut merged: toml::Value = toml::from_str(&default_template)
        .context("Failed to parse embedded default config template")?;
    let user_value: toml::Value =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;

    merge_toml_value(&mut merged, user_value);

    let config: ZenithConfig = merged.try_into().with_context(|| {
        format!(
            "Failed to deserialize merged config from {}",
            path.display()
        )
    })?;

    log::info!("Loaded configuration from {}", path.display());
    log::info!(
        "Applied config: height={}, gap_h={}, gap_top={}, radius={}, border_w={}, cycle={}s, clock={}, system_stats={}, todo={}, playerctl={}",
        config.bar.height,
        config.bar.gap_horizontal,
        config.bar.gap_top,
        config.bar.border_radius,
        config.bar.border_width,
        config.bar.rgb_cycle_seconds,
        config.modules.clock,
        config.modules.system_stats,
        config.modules.todo,
        config.modules.playerctl
    );

    if let Some(override_path) = std::env::var_os("ZENITH_CONFIG") {
        log::info!(
            "ZENITH_CONFIG override active: {}",
            PathBuf::from(override_path).display()
        );
    }

    Ok(config)
}
