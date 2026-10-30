use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::ClickActions;

/// The `[todo]` table. Todos themselves live in `$XDG_DATA_HOME/zenith-bar/todos.json`.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TodoConfig {
    #[serde(flatten)]
    pub actions: ClickActions,
}
