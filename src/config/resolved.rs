//! The validated, typed config the UI consumes.

use super::bar::{Monitors, Motion, Position};
use crate::modules::{ModuleId, clock::ClockConfig, logo::LogoConfig, system::SystemConfig};

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConfig {
    pub bar: Bar,
    pub layout: Layout,
    pub clock: ClockConfig,
    pub logo: LogoConfig,
    pub system: SystemConfig,
    /// Generated `:root { --zb-…: … }` block layered above the theme.
    pub css_overrides: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub position: Position,
    pub monitors: Monitors,
    pub height: u32,
    pub margin: u32,
    pub radius: u32,
    pub motion: Motion,
    /// `None` when set to `"none"`, or when the default focal module is not in the layout.
    pub focal: Option<ModuleId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub left: Vec<ModuleId>,
    pub center: Vec<ModuleId>,
    pub right: Vec<ModuleId>,
}

impl Layout {
    /// Every module on the bar, left to right.
    pub fn modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.left
            .iter()
            .chain(&self.center)
            .chain(&self.right)
            .copied()
    }

    pub fn contains(&self, id: ModuleId) -> bool {
        self.modules().any(|m| m == id)
    }
}
