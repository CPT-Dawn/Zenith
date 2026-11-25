use schemars::JsonSchema;
use serde::Deserialize;

use crate::modules::runners::RunnersConfig;

/// The `[cockpit]` table: the runner fleet's keys (as `[runners]` had them) and the feeds.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct CockpitConfig {
    #[serde(flatten)]
    pub runners: RunnersConfig,
    /// Commands that print a cockpit feed (JSON, contract v1). Each adds chips to the
    /// bar and cards to the popover.
    pub feeds: Vec<FeedConfig>,
}

/// One `[[cockpit.feeds]]` entry.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FeedConfig {
    /// Shell command, run with `sh -c`. Its stdout is one JSON feed document.
    pub command: String,
    /// Seconds from the end of one run to the start of the next (at least 5).
    pub interval: u32,
    /// Seconds a run may take before it is killed and counted as failed (at least 1).
    pub timeout: u32,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            interval: 15,
            timeout: 30,
        }
    }
}

impl CockpitConfig {
    /// Problems serde cannot catch, as (dotted key, message). A feed's key is
    /// `feeds.<index>.<key>`.
    pub fn problems(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .runners
            .problems()
            .into_iter()
            .map(|(key, message)| (key.to_string(), message))
            .collect();
        for (i, feed) in self.feeds.iter().enumerate() {
            let mut problem = |key: &str, message: &str| {
                out.push((format!("feeds.{i}.{key}"), format!("feeds[{i}]: {message}")));
            };
            if feed.command.trim().is_empty() {
                problem("command", "command must not be empty");
            }
            if feed.interval < 5 {
                problem("interval", "interval must be at least 5 seconds");
            }
            if feed.timeout < 1 {
                problem("timeout", "timeout must be at least 1 second");
            }
        }
        out
    }
}
