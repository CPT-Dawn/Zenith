//! Click actions: the shell commands any module table may bind to pointer gestures.
//!
//! A module config embeds [`ClickActions`] with `#[serde(flatten)]`, which puts the four
//! keys into that module's table, the schema and validation. A configured command
//! replaces the module's built-in action for that gesture; an unset key leaves it alone.

use schemars::JsonSchema;
use serde::Deserialize;

/// The click-action keys shared by every module table.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ClickActions {
    /// Shell command run on left click, instead of the module's built-in action.
    /// An empty string disables the built-in action without running anything.
    pub on_click: Option<String>,
    /// Shell command run on right click, instead of the module's built-in action.
    pub on_right_click: Option<String>,
    /// Shell command run on scroll up, instead of the module's built-in action.
    pub on_scroll_up: Option<String>,
    /// Shell command run on scroll down, instead of the module's built-in action.
    pub on_scroll_down: Option<String>,
}

/// A pointer gesture on a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gesture {
    Click,
    RightClick,
    ScrollUp,
    ScrollDown,
}

impl Gesture {
    pub const ALL: [Gesture; 4] = [
        Gesture::Click,
        Gesture::RightClick,
        Gesture::ScrollUp,
        Gesture::ScrollDown,
    ];
}

/// What a gesture on a module does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action<'a> {
    /// Nothing configured: the module's own behaviour, if it has any, applies.
    BuiltIn,
    /// Run this shell command (empty means do nothing) and suppress the built-in action.
    Run(&'a str),
}

impl ClickActions {
    /// No commands configured; every gesture keeps its built-in action.
    pub const NONE: ClickActions = ClickActions {
        on_click: None,
        on_right_click: None,
        on_scroll_up: None,
        on_scroll_down: None,
    };

    /// The config keys, as written in a module table.
    pub const KEYS: [&'static str; 4] = [
        "on_click",
        "on_right_click",
        "on_scroll_up",
        "on_scroll_down",
    ];

    pub fn action(&self, gesture: Gesture) -> Action<'_> {
        let command = match gesture {
            Gesture::Click => &self.on_click,
            Gesture::RightClick => &self.on_right_click,
            Gesture::ScrollUp => &self.on_scroll_up,
            Gesture::ScrollDown => &self.on_scroll_down,
        };
        command.as_deref().map_or(Action::BuiltIn, Action::Run)
    }

    /// True when no gesture is overridden.
    pub fn is_empty(&self) -> bool {
        *self == Self::NONE
    }
}
