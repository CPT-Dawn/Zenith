use schemars::JsonSchema;
use serde::Deserialize;

/// The `[system]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SystemConfig {
    /// Metrics to show, in order: any of `cpu`, `memory`, `temperature`.
    pub metrics: Vec<Metric>,
    /// Seconds between refreshes (at least 1). One timer serves every metric and every bar.
    pub interval: u32,
    /// CPU temperature thresholds and sensor.
    pub temperature: TemperatureConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            metrics: vec![Metric::Cpu, Metric::Memory, Metric::Temperature],
            interval: 2,
            temperature: TemperatureConfig::default(),
        }
    }
}

impl SystemConfig {
    /// Problems serde cannot catch, as (dotted key, message).
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if self.interval < 1 {
            out.push(("interval", "interval must be at least 1 second".into()));
        }
        for (i, metric) in self.metrics.iter().enumerate() {
            if self.metrics[..i].contains(metric) {
                out.push((
                    "metrics",
                    format!("metric `{}` is listed twice", metric.as_str()),
                ));
                break;
            }
        }
        let TemperatureConfig { warm, hot, .. } = self.temperature;
        if warm >= hot {
            out.push((
                "temperature.warm",
                format!("temperature.warm ({warm}) must be below temperature.hot ({hot})"),
            ));
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Cpu,
    Memory,
    Temperature,
}

impl Metric {
    pub fn as_str(self) -> &'static str {
        match self {
            Metric::Cpu => "cpu",
            Metric::Memory => "memory",
            Metric::Temperature => "temperature",
        }
    }
}

/// The `[system.temperature]` table.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TemperatureConfig {
    /// From this many °C the module gets the `zb-warm` state.
    pub warm: u32,
    /// From this many °C the module gets the `zb-hot` state.
    pub hot: u32,
    /// Pin the sensor instead of finding it: a hwmon chip name (`k10temp`), a thermal
    /// zone type (`x86_pkg_temp`) or an absolute path to a temperature file.
    pub sensor: Option<String>,
}

impl Default for TemperatureConfig {
    fn default() -> Self {
        Self {
            warm: 70,
            hot: 85,
            sensor: None,
        }
    }
}
