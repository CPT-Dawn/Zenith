use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[clipboard]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ClipboardConfig {
    /// Shell command run by the popover's "Search all…" row.
    pub search_command: String,
    #[serde(flatten)]
    pub actions: ClickActions,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            search_command: "hoarder picker".into(),
            actions: ClickActions::default(),
        }
    }
}
