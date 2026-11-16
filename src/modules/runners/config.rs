use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[runners]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RunnersConfig {
    /// Repositories whose self-hosted runners are shown, as "owner/name".
    pub repos: Vec<String>,
    /// How many runner slots you run, across all repos. Fewer registered slots show as
    /// missing. 0 turns the check off.
    pub expected: u32,
    /// Seconds between polls (at least 10). Unchanged replies don't count against
    /// GitHub's rate limit.
    pub interval: u32,
    /// Shell command that prints a GitHub token. It needs read access to the repos'
    /// runners (admin on the repo, or a classic token with `repo`).
    pub token_command: String,
    /// GitHub API base URL; change it for GitHub Enterprise Server.
    pub api_url: String,
    #[serde(flatten)]
    pub actions: ClickActions,
}

impl RunnersConfig {
    /// Problems serde cannot catch, as (key, message).
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(bad) = self.repos.iter().find(|r| !is_repo(r)) {
            out.push((
                "repos",
                format!("`{bad}` is not a repository; write it as \"owner/name\""),
            ));
        }
        if self.interval < 10 {
            out.push(("interval", "interval must be at least 10 seconds".into()));
        }
        if self.token_command.trim().is_empty() {
            out.push(("token_command", "token_command must not be empty".into()));
        }
        if !(self.api_url.starts_with("https://") || self.api_url.starts_with("http://")) {
            out.push((
                "api_url",
                "api_url must start with https:// or http://".into(),
            ));
        }
        out
    }
}

fn is_repo(repo: &str) -> bool {
    let ok = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    matches!(repo.split_once('/'), Some((owner, name)) if ok(owner) && ok(name))
}

impl Default for RunnersConfig {
    fn default() -> Self {
        Self {
            repos: Vec::new(),
            expected: 0,
            interval: 30,
            token_command: "gh auth token".into(),
            api_url: "https://api.github.com".into(),
            actions: ClickActions::default(),
        }
    }
}
