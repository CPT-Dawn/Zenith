use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::BarConfig;

/// Embedded default stylesheet copied to disk on first launch.
const DEFAULT_STYLE_TEMPLATE: &str = include_str!("../Default_Style.css");

const TOKEN_RADIUS: &str = "__ZENITH_RADIUS__";
const TOKEN_BORDER_WIDTH: &str = "__ZENITH_BORDER_WIDTH__";
const TOKEN_INNER_RADIUS: &str = "__ZENITH_INNER_RADIUS__";
const TOKEN_CYCLE_SECONDS: &str = "__ZENITH_CYCLE_SECONDS__";
const TOKEN_BACKGROUND: &str = "__ZENITH_BACKGROUND__";

/// Return the canonical style path: `~/.config/zenith/style.css`.
pub fn style_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("ZENITH_STYLE") {
        return Ok(PathBuf::from(path));
    }

    if let Some(config_override) = std::env::var_os("ZENITH_CONFIG") {
        let config_override = PathBuf::from(config_override);
        let parent = config_override.parent().unwrap_or(Path::new("."));
        return Ok(parent.join("style.css"));
    }

    Ok(crate::config::config_dir()?.join("style.css"))
}

/// Ensure the style file exists.
///
/// If `style.css` is missing, this writes `Default_Style.css` as the initial
/// user-facing stylesheet.
fn ensure_style_file(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .context("Style path has no parent directory")?;

    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create style directory {}", parent.display()))?;

    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;

            file.write_all(DEFAULT_STYLE_TEMPLATE.as_bytes())
                .with_context(|| format!("Failed to write default style to {}", path.display()))?;

            log::info!("Created default style at {}", path.display());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => {
            Err(err).with_context(|| format!("Failed to create style file {}", path.display()))
        }
    }
}

/// Render style-template tokens from the current runtime config values.
fn render_template(template: &str, bar: &BarConfig) -> String {
    let inner_radius = bar.border_radius.saturating_sub(bar.border_width);

    template
        .replace(TOKEN_RADIUS, &bar.border_radius.to_string())
        .replace(TOKEN_BORDER_WIDTH, &bar.border_width.to_string())
        .replace(TOKEN_INNER_RADIUS, &inner_radius.to_string())
        .replace(TOKEN_CYCLE_SECONDS, &bar.rgb_cycle_seconds.to_string())
        .replace(TOKEN_BACKGROUND, &bar.background)
}

/// Load the user stylesheet from disk and apply config-driven template values.
pub fn load(bar: &BarConfig) -> Result<String> {
    let path = style_path()?;
    ensure_style_file(&path)?;

    if let Ok(cwd) = std::env::current_dir() {
        let local_style = cwd.join("style.css");
        if local_style.exists() && local_style != path {
            log::warn!(
                "Ignoring local style at {}; using {}",
                local_style.display(),
                path.display()
            );
        }
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read style at {}", path.display()))?;

    let css = render_template(&raw, bar);

    log::info!("Loaded style from {}", path.display());

    if let Some(override_path) = std::env::var_os("ZENITH_STYLE") {
        log::info!(
            "ZENITH_STYLE override active: {}",
            PathBuf::from(override_path).display()
        );
    }

    Ok(css)
}
