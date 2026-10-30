use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[calendar]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct CalendarConfig {
    /// strftime-style date format. The label is redrawn once a day, at local midnight.
    pub format: String,
    #[serde(flatten)]
    pub actions: ClickActions,
}

impl CalendarConfig {
    /// Problems serde cannot catch, as (key, message).
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let sample = jiff::Timestamp::UNIX_EPOCH.to_zoned(jiff::tz::TimeZone::UTC);
        match jiff::fmt::strtime::format(&self.format, &sample) {
            Ok(_) => Vec::new(),
            Err(e) => vec![("format", format!("invalid calendar format: {e}"))],
        }
    }
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            format: "%a %d %b".into(),
            actions: ClickActions::default(),
        }
    }
}
