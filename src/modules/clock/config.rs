use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[clock]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ClockConfig {
    /// strftime-style format. The clock wakes every second only if this shows seconds.
    pub format: String,
    #[serde(flatten)]
    pub actions: ClickActions,
}

impl ClockConfig {
    /// Problems serde cannot catch, as (key, message).
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        crate::modules::zoned_source::format_problems("clock", &self.format)
    }
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format: "%H:%M".into(),
            actions: ClickActions::default(),
        }
    }
}
