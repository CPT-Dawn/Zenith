use schemars::JsonSchema;
use serde::Deserialize;

/// The `[system]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SystemConfig {
    /// Metrics to show, in order: any of `cpu`, `memory`, `temperature`.
    pub metrics: Vec<Metric>,
    /// CPU temperature thresholds and sensor.
    pub temperature: TemperatureConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            metrics: vec![Metric::Cpu, Metric::Memory, Metric::Temperature],
            temperature: TemperatureConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Cpu,
    Memory,
    Temperature,
}

/// The `[system.temperature]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TemperatureConfig {
    /// From this many °C the module gets the `zb-warm` state.
    pub warm: u32,
    /// From this many °C the module gets the `zb-hot` state.
    pub hot: u32,
}

impl Default for TemperatureConfig {
    fn default() -> Self {
        Self { warm: 70, hot: 85 }
    }
}
