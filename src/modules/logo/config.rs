use schemars::JsonSchema;
use serde::Deserialize;

/// The `[logo]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct LogoConfig {
    /// Icon name: a bundled `zb-*-symbolic` icon or any name from your icon theme. Empty hides the logo.
    pub icon: String,
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            icon: "zb-logo-symbolic".into(),
        }
    }
}
