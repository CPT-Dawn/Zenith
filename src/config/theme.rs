//! The `[theme]` table and the theme layer it generates above the built-in theme CSS.

use std::fmt::Write;

use schemars::JsonSchema;
use serde::Deserialize;

use super::bar::Motion;
use super::resolved::{Bar, ResolvedConfig};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ThemeConfig {
    /// Built-in theme to start from.
    pub name: ThemeName,
    /// Signature colour, overriding the theme's `--zb-accent` (e.g. `"#ff79c6"`).
    pub accent: Option<String>,
    /// Opacity of the bar surface from 0 (clear) to 1 (opaque), overriding `--zb-surface-opacity`.
    pub surface_opacity: Option<f64>,
    /// Corner radius of module pills in pixels, overriding `--zb-module-radius`.
    pub radius: Option<u32>,
}

impl ThemeConfig {
    /// Value problems, as `(key, message)`. Overrides are pasted into CSS, so they must be
    /// plain values that cannot close the block.
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(accent) = &self.accent
            && !is_colour(accent)
        {
            out.push((
                "accent",
                format!(
                    "`{accent}` is not a colour (expected e.g. `#8de3dc`, `rgb(141 227 220)` or a colour name)"
                ),
            ));
        }
        if let Some(opacity) = self.surface_opacity
            && !(0.0..=1.0).contains(&opacity)
        {
            out.push((
                "surface_opacity",
                "surface_opacity must be between 0 and 1".into(),
            ));
        }
        out
    }
}

/// A hex colour, a colour function such as `rgb(…)` or `hsl(…)`, or a colour name.
fn is_colour(value: &str) -> bool {
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    if let Some((function, rest)) = value.split_once('(') {
        const FUNCTIONS: [&str; 8] = ["rgb", "rgba", "hsl", "hsla", "hwb", "lab", "oklab", "oklch"];
        let Some(args) = rest.strip_suffix(')') else {
            return false;
        };
        return FUNCTIONS.contains(&function)
            && args
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || " .,%/+-".contains(c));
    }
    !value.is_empty() && value.chars().all(|c| c.is_ascii_alphabetic())
}

/// A built-in theme. Only the selected one is ever parsed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    #[default]
    MidnightBloom,
}

impl ThemeName {
    /// The theme's stylesheet, relative to the resource prefix.
    pub fn resource(self) -> &'static str {
        match self {
            ThemeName::MidnightBloom => "themes/midnight-bloom.css",
        }
    }
}

/// One stylesheet slot, lowest priority first. Each slot is one CSS provider, so a reload
/// only reloads the slots whose [`Css`] changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleLayer {
    /// The selected built-in theme: `:root { --zb-* }` tokens only.
    Theme,
    /// Component CSS. It only reads variables, so it sits below the overrides.
    Base,
    /// The generated `:root` block from `[bar]` geometry and `[theme]` overrides.
    Overrides,
    /// `motion-off.css`, `ambient.css` or nothing.
    Motion,
    /// The user's `style.css`, above everything.
    User,
}

/// What a [`StyleLayer`] is loaded from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Css {
    /// A bundled stylesheet, relative to the resource prefix.
    Resource(&'static str),
    /// Generated CSS text.
    Text(String),
    /// The user's style file, wherever paths resolved it to.
    UserFile,
    /// Nothing.
    Empty,
}

/// Every stylesheet the bar loads, in priority order (lowest first).
///
/// Motion is enforced structurally (ADR-0003): ambient keyframes exist only in the
/// ambient sheet, and `off` or a reduced-motion preference swaps in a sheet that
/// disables every transition. Reduced motion wins over `ambient`.
pub fn stylesheets(config: &ResolvedConfig, reduced_motion: bool) -> [(StyleLayer, Css); 5] {
    let motion = match (config.bar.motion, reduced_motion) {
        (Motion::Off, _) | (_, true) => Css::Resource("css/motion-off.css"),
        (Motion::Ambient, false) => Css::Resource("css/ambient.css"),
        (Motion::Subtle, false) => Css::Empty,
    };
    [
        (
            StyleLayer::Theme,
            Css::Resource(config.theme.name.resource()),
        ),
        (StyleLayer::Base, Css::Resource("css/base.css")),
        (
            StyleLayer::Overrides,
            Css::Text(config.css_overrides.clone()),
        ),
        (StyleLayer::Motion, motion),
        (StyleLayer::User, Css::UserFile),
    ]
}

/// `:root { --zb-… }` declarations derived from config: bar geometry, then `[theme]` overrides.
pub fn override_block(bar: &Bar, theme: &ThemeConfig) -> String {
    let mut css = String::from(":root {\n");
    let mut var = |name: &str, value: &dyn std::fmt::Display| {
        let _ = writeln!(css, "  --zb-{name}: {value};");
    };
    var("bar-height", &format_args!("{}px", bar.height));
    var("bar-radius", &format_args!("{}px", bar.radius));
    if let Some(accent) = &theme.accent {
        var("accent", accent);
    }
    if let Some(opacity) = theme.surface_opacity {
        var("surface-opacity", &opacity);
    }
    if let Some(radius) = theme.radius {
        var("module-radius", &format_args!("{radius}px"));
    }
    css.push_str("}\n");
    css
}
