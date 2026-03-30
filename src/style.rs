use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::BarConfig;

/// Embedded default stylesheet copied to disk on first launch.
const EMBEDDED_DEFAULT_STYLE_TEMPLATE: &str = include_str!("../Default_Style.css");

const TOKEN_RADIUS: &str = "__ZENITH_RADIUS__";
const TOKEN_BORDER_WIDTH: &str = "__ZENITH_BORDER_WIDTH__";
const TOKEN_INNER_RADIUS: &str = "__ZENITH_INNER_RADIUS__";
const TOKEN_CYCLE_SECONDS: &str = "__ZENITH_CYCLE_SECONDS__";
const TOKEN_BACKGROUND: &str = "__ZENITH_BACKGROUND__";

const TEMP_CLASS_BASE: &str = ".zenith-module-temp";
const TEMP_CLASS_COOL: &str = ".zenith-module-temp-cool";
const TEMP_CLASS_WARM: &str = ".zenith-module-temp-warm";
const TEMP_CLASS_HOT: &str = ".zenith-module-temp-hot";
const MODULE_SURFACE_CLASS: &str = ".zenith-module-surface";
const TODO_PLUS_CLASS: &str = ".zenith-todo-btn-plus";
const PLAYER_BTN_CLASS: &str = ".zenith-player-btn";
const PLAYER_TITLE_CLASS: &str = ".zenith-player-title";
const PLAYER_PROGRESS_CLASS: &str = ".zenith-player-progress";



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
fn ensure_style_file(path: &Path, default_template: &str) -> Result<()> {
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

            file.write_all(default_template.as_bytes())
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

/// Resolve the default-style template text.
///
/// Order of precedence:
/// 1) `ZENITH_DEFAULT_STYLE_TEMPLATE` path, if set and readable.
/// 2) `./Default_Style.css` from current working directory, if readable.
/// 3) Embedded compile-time template fallback.
fn default_style_template() -> String {
    if let Some(path) = std::env::var_os("ZENITH_DEFAULT_STYLE_TEMPLATE") {
        let path = PathBuf::from(path);
        match fs::read_to_string(&path) {
            Ok(content) => {
                log::info!(
                    "Using runtime default style template from {}",
                    path.display()
                );
                return content;
            }
            Err(err) => {
                log::warn!(
                    "Failed to read ZENITH_DEFAULT_STYLE_TEMPLATE at {}: {err}; falling back",
                    path.display()
                );
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let path = cwd.join("Default_Style.css");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    log::info!(
                        "Using runtime default style template from {}",
                        path.display()
                    );
                    return content;
                }
                Err(err) => {
                    log::warn!(
                        "Failed to read runtime default style template at {}: {err}; falling back",
                        path.display()
                    );
                }
            }
        }
    }

    EMBEDDED_DEFAULT_STYLE_TEMPLATE.to_string()
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
    let has_module_surface = css.contains(MODULE_SURFACE_CLASS);
    let has_todo_plus = css.contains(TODO_PLUS_CLASS);
    let has_player_btn = css.contains(PLAYER_BTN_CLASS);
    let has_player_title = css.contains(PLAYER_TITLE_CLASS);
    let has_player_progress = css.contains(PLAYER_PROGRESS_CLASS);

    if has_base
        && has_cool
        && has_warm
        && has_hot
        && has_module_surface
        && has_todo_plus
        && has_player_btn
        && has_player_title
        && has_player_progress
    {
        return css.to_string();
    }

    let mut out = String::with_capacity(css.len() + 980);
    out.push_str(css);
    out.push_str("\n\n/* Injected defaults for backward-compatible styling */\n");

    if !has_base {
        out.push_str(".zenith-module-temp { color: #e0af68; }\n");
    }
    if !has_cool {
        out.push_str(".zenith-module-temp-cool { color: #7dcfff; }\n");
    }
    if !has_warm {
        out.push_str(".zenith-module-temp-warm { color: #e0af68; }\n");
    }
    if !has_hot {
        out.push_str(".zenith-module-temp-hot { color: #f7768e; }\n");
    }
    if !has_module_surface {
        out.push_str(
            ".zenith-module-surface { background: transparent; border: none; border-radius: 4px; padding: 2px 8px; transition: background 140ms ease; }\n",
        );
        out.push_str(
            ".zenith-module-surface:hover { background: rgba(255, 255, 255, 0.06); }\n",
        );
        out.push_str(".zenith-module-surface:active { background: rgba(255, 255, 255, 0.10); }\n");
    }
    if !has_todo_plus {
        out.push_str(
            ".zenith-todo-btn-plus { font-size: 15px; font-weight: 800; min-width: 22px; padding: 1px 8px; line-height: 1; }\n",
        );
    }
    if !has_player_btn {
        out.push_str(
            ".zenith-player-btn { background: transparent; border: none; box-shadow: none; padding: 1px 8px; min-height: 0; min-width: 160px; }\n",
        );
    }
    if !has_player_title {
        out.push_str(
            ".zenith-player-title { font-family: \"JetBrainsMono Nerd Font\", \"Inter\", monospace; font-size: 11px; font-weight: 600; color: #73daca; }\n",
        );
    }
    if !has_player_progress {
        out.push_str(".zenith-player-progress { min-height: 3px; }\n");
        out.push_str(
            ".zenith-player-progress trough { min-height: 3px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); }\n",
        );
        out.push_str(
            ".zenith-player-progress progress { min-height: 3px; border-radius: 999px; background: linear-gradient(90deg, #73daca, #7dcfff); box-shadow: 0 0 6px rgba(115, 218, 202, 0.30); }\n",
        );
    }

    out
}

/// Load the user stylesheet from disk and apply config-driven template values.
pub fn load(bar: &BarConfig) -> Result<String> {
    let default_template = default_style_template();

    let path = style_path()?;
    ensure_style_file(&path, &default_template)?;

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
