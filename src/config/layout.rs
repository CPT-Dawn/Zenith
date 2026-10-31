//! The `[layout]` table: which modules sit in which section, in order.

use schemars::JsonSchema;
use serde::Deserialize;

/// Leaving `[layout]` out gives the default layout; once it is written, any
/// section left out is empty. A module may appear only once across all sections.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
pub struct LayoutConfig {
    /// Modules in the left section, in order.
    #[serde(default)]
    pub left: Vec<String>,
    /// Modules in the center section, in order.
    #[serde(default)]
    pub center: Vec<String>,
    /// Modules in the right section, in order.
    #[serde(default)]
    pub right: Vec<String>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        let names = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect();
        Self {
            left: names(&["logo", "todo"]),
            center: names(&["clock", "calendar"]),
            right: names(&["media", "system"]),
        }
    }
}

/// One of the three ordered slots of a bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Left,
    Center,
    Right,
}

impl Section {
    /// Every section, in bar order.
    pub const ALL: [Section; 3] = [Section::Left, Section::Center, Section::Right];

    /// Its key in `[layout]`.
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Left => "left",
            Section::Center => "center",
            Section::Right => "right",
        }
    }

    /// Its stable CSS class.
    pub fn css_class(self) -> &'static str {
        match self {
            Section::Left => "zb-left",
            Section::Center => "zb-center",
            Section::Right => "zb-right",
        }
    }
}
