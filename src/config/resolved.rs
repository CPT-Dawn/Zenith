//! The validated, typed config the UI consumes.

use super::actions::ClickActions;
use super::bar::{Monitors, Motion, Position};
use super::theme::ThemeConfig;
use crate::modules::calendar::CalendarConfig;
use crate::modules::media::MediaConfig;
use crate::modules::{ModuleId, clock::ClockConfig, logo::LogoConfig, system::SystemConfig};

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConfig {
    pub bar: Bar,
    pub layout: Layout,
    pub clock: ClockConfig,
    pub logo: LogoConfig,
    pub media: MediaConfig,
    pub theme: ThemeConfig,
    pub calendar: CalendarConfig,
    pub system: SystemConfig,
    /// Generated `:root { --zb-…: … }` block layered above the theme.
    pub css_overrides: String,
}

impl ResolvedConfig {
    /// The config-given CSS classes of a module's view: `zb-module zb-<id>`, plus
    /// `zb-focal` on the focal element. State classes come from the module's model.
    pub fn module_classes(&self, id: ModuleId) -> Vec<String> {
        let mut classes = vec!["zb-module".to_string(), format!("zb-{id}")];
        if self.bar.focal == Some(id) {
            classes.push("zb-focal".into());
        }
        classes
    }

    /// The click actions configured in a module's table. One arm per module.
    pub fn actions(&self, id: ModuleId) -> &ClickActions {
        match id {
            ModuleId::Clock => &self.clock.actions,
            ModuleId::Logo => &self.logo.actions,
            ModuleId::Media => &self.media.actions,
            ModuleId::Calendar => &self.calendar.actions,
            ModuleId::System => &self.system.actions,
            _ => &ClickActions::NONE,
        }
    }
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
