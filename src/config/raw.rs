//! The whole config file as written: one field per top-level table, every field defaulted.
//! Add a line per new table; validation and the schema follow automatically.

use schemars::JsonSchema;
use serde::Deserialize;

use super::bar::BarConfig;
use super::layout::LayoutConfig;
use crate::modules::calendar::CalendarConfig;
use crate::modules::{clock::ClockConfig, logo::LogoConfig, system::SystemConfig};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RawConfig {
    pub bar: BarConfig,
    pub layout: LayoutConfig,
    pub clock: ClockConfig,
    pub logo: LogoConfig,
    pub calendar: CalendarConfig,
    pub system: SystemConfig,
}
