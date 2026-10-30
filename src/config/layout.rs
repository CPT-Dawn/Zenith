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

/// Section names, in bar order.
pub const SECTIONS: [&str; 3] = ["left", "center", "right"];
