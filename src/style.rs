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

const TEMP_CLASS_BASE: &str = ".zenith-module-temp";
const TEMP_CLASS_COOL: &str = ".zenith-module-temp-cool";
const TEMP_CLASS_WARM: &str = ".zenith-module-temp-warm";
const TEMP_CLASS_HOT: &str = ".zenith-module-temp-hot";
const TODO_PLUS_CLASS: &str = ".zenith-todo-btn-plus";

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

/// Append fallback rules when older user stylesheets don't define newer
/// classes yet.
fn ensure_compat_style_rules(css: &str) -> String {
    let has_base = css.contains(TEMP_CLASS_BASE);
    let has_cool = css.contains(TEMP_CLASS_COOL);
    let has_warm = css.contains(TEMP_CLASS_WARM);
    let has_hot = css.contains(TEMP_CLASS_HOT);
    let has_todo_plus = css.contains(TODO_PLUS_CLASS);

    if has_base && has_cool && has_warm && has_hot && has_todo_plus {
        return css.to_string();
    }

    let mut out = String::with_capacity(css.len() + 340);
    out.push_str(css);
    out.push_str("\n\n/* Injected defaults for backward-compatible styling */\n");

    if !has_base {
        out.push_str(".zenith-module-temp { color: #ffcc00; }\n");
    }
    if !has_cool {
        out.push_str(".zenith-module-temp-cool { color: #00ccff; }\n");
    }
    if !has_warm {
        out.push_str(".zenith-module-temp-warm { color: #ffcc00; }\n");
    }
    if !has_hot {
        out.push_str(".zenith-module-temp-hot { color: #ff5555; }\n");
    }
    if !has_todo_plus {
        out.push_str(
            ".zenith-todo-btn-plus { font-size: 18px; font-weight: 800; min-width: 28px; padding: 2px 12px; line-height: 1; }\n",
        );
    }

    out
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

    let css = ensure_compat_style_rules(&render_template(&raw, bar));

    log::info!("Loaded style from {}", path.display());

    if let Some(override_path) = std::env::var_os("ZENITH_STYLE") {
        log::info!(
            "ZENITH_STYLE override active: {}",
            PathBuf::from(override_path).display()
        );
    }

    Ok(css)
}
