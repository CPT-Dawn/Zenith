use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[media]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct MediaConfig {
    /// Label format. Placeholders: `{title}`, `{artist}`, `{album}`. Separators next to an
    /// empty placeholder are dropped, so `{title} - {artist}` shows just the title when there is no artist.
    pub format: String,
    /// Longest label in characters before it is cut with `…`. 0 means no limit.
    pub max_length: u32,
    /// Hide the module when nothing is playing, instead of showing "No media".
    pub hide_when_idle: bool,
    #[serde(flatten)]
    pub actions: ClickActions,
}

impl MediaConfig {
    /// Problems serde cannot catch, as (key, message).
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        match super::model::parse_format(&self.format) {
            Ok(_) => Vec::new(),
            Err(e) => vec![("format", e)],
        }
    }
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            format: "{title} - {artist}".into(),
            max_length: 40,
            hide_when_idle: false,
            actions: ClickActions::default(),
        }
    }
}
