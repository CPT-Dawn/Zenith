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

/// Runtime-managed compatibility CSS.
///
/// This block is appended on every load so future style updates can be made by
/// editing this constant only, without rewriting user files on disk.
const RUNTIME_COMPAT_CSS: &str = r#"/* Zenith runtime compatibility block (managed) */
.zenith-module-temp { color: #e0af68; }
.zenith-module-temp-cool { color: #7dcfff; }
.zenith-module-temp-warm { color: #e0af68; }
.zenith-module-temp-hot { color: #f7768e; }
.zenith-module-surface { background: transparent; border: none; border-radius: 4px; padding: 2px 8px; transition: background 140ms ease; }
.zenith-module-surface:hover { background: rgba(255, 255, 255, 0.06); }
.zenith-module-surface:active { background: rgba(255, 255, 255, 0.10); }
.zenith-todo-btn-plus { font-size: 15px; font-weight: 800; min-width: 24px; min-height: 24px; padding: 0 7px; line-height: 1.05; }
.zenith-todo-action-compact { background: rgba(255, 255, 255, 0.04); border: 1px solid rgba(255, 255, 255, 0.08); box-shadow: none; min-height: 26px; min-width: 26px; padding: 0; font-size: 11px; font-weight: 700; line-height: 1; border-radius: 8px; color: #8a98bc; transition: background 160ms ease, border-color 160ms ease, color 160ms ease, box-shadow 160ms ease, transform 160ms ease; }
.zenith-todo-action-compact:hover { background: rgba(122, 162, 247, 0.14); border-color: rgba(122, 162, 247, 0.26); color: #c0caf5; box-shadow: 0 0 8px rgba(122, 162, 247, 0.12); }
.zenith-todo-action-compact:active { transform: translateY(1px); }
.zenith-todo-row-action { min-width: 26px; min-height: 26px; border-radius: 8px; font-size: 11px; font-weight: 700; padding: 0; }
.zenith-todo-action-compact-muted:hover { background: rgba(122, 162, 247, 0.14); border-color: rgba(122, 162, 247, 0.24); color: #7dcfff; }
.zenith-todo-action-compact-danger:hover { background: rgba(247, 118, 142, 0.16); border-color: rgba(247, 118, 142, 0.26); color: #f7768e; box-shadow: 0 0 8px rgba(247, 118, 142, 0.12); }
.zenith-todo-action-compact-primary, .zenith-todo-add-btn { background: linear-gradient(135deg, #7aa2f7, #7dcfff); border: none; border-radius: 10px; color: #1a1b26; font-weight: 800; font-size: 14px; min-width: 32px; min-height: 32px; padding: 0; box-shadow: 0 0 8px rgba(122, 162, 247, 0.18); transition: box-shadow 180ms ease, transform 180ms ease, filter 180ms ease; }
.zenith-todo-action-compact-primary:hover, .zenith-todo-add-btn:hover { box-shadow: 0 0 12px rgba(122, 162, 247, 0.30); transform: translateY(-1px); }
.zenith-todo-action-compact-primary:active, .zenith-todo-add-btn:active { box-shadow: 0 0 6px rgba(122, 162, 247, 0.15); transform: translateY(0); }
.zenith-todo-entry-main { min-height: 32px; }
.zenith-todo-entry-inline { min-height: 26px; padding: 3px 8px; font-size: 11px; }
.zenith-player-btn { background: transparent; border: none; box-shadow: none; padding: 1px 8px; min-height: 0; min-width: 160px; }
.zenith-player-title { font-family: "JetBrainsMono Nerd Font", "Inter", monospace; font-size: 11px; font-weight: 600; color: #73daca; }
.zenith-player-progress { min-height: 3px; }
.zenith-player-progress trough { min-height: 3px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); }
.zenith-player-progress progress { min-height: 3px; border-radius: 999px; background: linear-gradient(90deg, #73daca, #7dcfff); box-shadow: 0 0 6px rgba(115, 218, 202, 0.30); }
"#;

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

/// Append the runtime-managed compatibility rules without mutating user files.
fn ensure_compat_style_rules(css: &str) -> String {
    let trimmed = css.trim_end();
    let mut out = String::with_capacity(trimmed.len() + RUNTIME_COMPAT_CSS.len() + 4);

    out.push_str(trimmed);
    out.push_str("\n\n");
    out.push_str(RUNTIME_COMPAT_CSS);
    out.push('\n');

    out
}

/// Load the user stylesheet from disk and apply config-driven template values.
pub fn load(bar: &BarConfig) -> Result<String> {
    let path = style_path()?;
    ensure_style_file(&path, EMBEDDED_DEFAULT_STYLE_TEMPLATE)?;

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
