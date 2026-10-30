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
        let sample = jiff::Timestamp::UNIX_EPOCH.to_zoned(jiff::tz::TimeZone::UTC);
        match jiff::fmt::strtime::format(&self.format, &sample) {
            Ok(_) => Vec::new(),
            Err(e) => vec![("format", format!("invalid clock format: {e}"))],
        }
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
