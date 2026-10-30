//! Config resolution: TOML text in, a [`ResolvedConfig`] or every [`Diagnostic`] out.
//!
//! `run`, `check`, `init` and hot reload all go through [`resolve`].

mod actions;
mod bar;
mod diagnostic;
mod layout;
mod preset;
mod raw;
mod resolved;
pub mod schema;
mod theme;
mod validate;

pub use actions::{Action, ClickActions, Gesture};
pub use bar::{BarConfig, Monitors, Motion, Position};
pub use diagnostic::Diagnostic;
pub use layout::{LayoutConfig, SECTIONS};
pub use preset::Preset;
pub use raw::RawConfig;
pub use resolved::{Bar, Layout, ResolvedConfig};
pub use theme::{Css, REQUIRED_VARS, StyleLayer, ThemeConfig, ThemeName, stylesheets};

/// The config file `init` writes: the preset's text, or the commented default starter.
pub fn starter(preset: Option<Preset>) -> &'static str {
    if let Some(preset) = preset {
        match preset {}
    }
    include_str!("starter.toml")
}

/// Resolve config text, optionally on top of a preset. Empty text means all defaults.
/// Diagnostic positions refer to `text`.
pub fn resolve(text: &str, preset: Option<Preset>) -> Result<ResolvedConfig, Vec<Diagnostic>> {
    if let Some(preset) = preset {
        match preset {}
    }

    let document = toml_edit::Document::parse(text)
        .map_err(|e| vec![Diagnostic::at(text, e.span(), validate::tidy(e.message()))])?;

    let diagnostics = validate::document(text, &document);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let raw: RawConfig = toml::from_str(text)
        .map_err(|e| vec![Diagnostic::at(text, e.span(), validate::tidy(e.message()))])?;

    let diagnostics = validate::values(text, &document, &raw);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(build(&document, raw))
}

fn build(document: &toml_edit::Document<&str>, raw: RawConfig) -> ResolvedConfig {
    let ids = |names: &[String]| names.iter().filter_map(|n| n.parse().ok()).collect();
    let layout = Layout {
        left: ids(&raw.layout.left),
        center: ids(&raw.layout.center),
        right: ids(&raw.layout.right),
    };

    // An explicit focal is validated against the layout; the default quietly
    // steps aside when its module is not on the bar.
    let focal = raw.bar.focal.parse().ok();
    let focal_explicit = validate::lookup(document, &["bar", "focal"]).is_some();
    let focal = focal.filter(|id| focal_explicit || layout.contains(*id));

    let bar = Bar {
        position: raw.bar.position,
        monitors: raw.bar.monitors,
        height: raw.bar.height,
        margin: raw.bar.margin,
        radius: raw.bar.radius,
        motion: raw.bar.motion,
        focal,
    };
    let css_overrides = theme::override_block(&bar, &raw.theme);

    ResolvedConfig {
        bar,
        layout,
        clock: raw.clock,
        logo: raw.logo,
        theme: raw.theme,
        calendar: raw.calendar,
        system: raw.system,
        css_overrides,
    }
}
