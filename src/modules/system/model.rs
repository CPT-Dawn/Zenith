//! Pure system model: `/proc` and sysfs *text* in, a fixed-width label out.
//! CPU usage needs two samples, so the model remembers the previous one.

use super::config::TemperatureConfig;
use super::{Metric, SystemConfig};
use crate::modules::ViewState;

/// U+2007 FIGURE SPACE: as wide as a digit, so padded numbers never change width.
const FIGURE_SPACE: char = '\u{2007}';

/// Readings outside this range (°C) are sensor glitches, not temperatures.
const PLAUSIBLE: std::ops::RangeInclusive<f64> = 0.0..=150.0;

/// The raw text read this tick. `None` = not read (metric off, no sensor) or unreadable.
#[derive(Clone, Copy, Debug, Default)]
pub struct Readings<'a> {
    /// Contents of `/proc/stat`.
    pub stat: Option<&'a str>,
    /// Contents of `/proc/meminfo`.
    pub meminfo: Option<&'a str>,
    /// Contents of the sensor file: millidegrees Celsius.
    pub temperature: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct SystemModel {
    previous_cpu: Option<CpuTimes>,
}

impl SystemModel {
    pub fn update(&mut self, readings: &Readings, config: &SystemConfig) -> ViewState {
        // Always advance the CPU baseline, even if cpu is not shown.
        let cpu = readings.stat.and_then(CpuTimes::parse).map(|now| {
            let usage = now.usage_since(self.previous_cpu.unwrap_or_default());
            self.previous_cpu = Some(now);
            usage
        });

        let mut state = ViewState::default();
        let mut parts = Vec::new();
        let mut details = Vec::new();
        for metric in &config.metrics {
            match metric {
                Metric::Cpu => {
                    if let Some(cpu) = cpu {
                        parts.push(format!("CPU {}%", pad(cpu)));
                        details.push(format!("CPU {:.0}%", cpu));
                    }
                }
                Metric::Memory => {
                    if let Some(memory) = readings.meminfo.and_then(memory_usage) {
                        parts.push(format!("MEM {}%", pad(memory)));
                        details.push(format!("Memory {:.0}%", memory));
                    }
                }
                Metric::Temperature => {
                    if let Some(celsius) = readings.temperature.and_then(celsius) {
                        parts.push(format!("{}°C", pad(celsius)));
                        let heat = Heat::of(celsius, &config.temperature);
                        let word = match heat {
                            Heat::Normal => "",
                            Heat::Warm => " (warm)",
                            Heat::Hot => " (hot)",
                        };
                        details.push(format!("Temperature {celsius:.0} °C{word}"));
                        state.classes.extend(heat.class());
                        state.icon = heat.icon().map(str::to_string);
                    }
                }
            }
        }

        state.visible = !parts.is_empty();
        state.label = parts.join("  ");
        state.tooltip = Some(details.join("\n")).filter(|t| !t.is_empty());
        state
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Heat {
    Normal,
    Warm,
    Hot,
}

impl Heat {
    fn of(celsius: f64, thresholds: &TemperatureConfig) -> Self {
        if celsius >= thresholds.hot as f64 {
            Heat::Hot
        } else if celsius >= thresholds.warm as f64 {
            Heat::Warm
        } else {
            Heat::Normal
        }
    }

    fn class(self) -> Option<&'static str> {
        match self {
            Heat::Normal => None,
            Heat::Warm => Some("zb-warm"),
            Heat::Hot => Some("zb-hot"),
        }
    }

    /// A distinct shape per state, so the state reads without colour.
    fn icon(self) -> Option<&'static str> {
        match self {
            Heat::Normal => None,
            Heat::Warm => Some("zb-system-warm-symbolic"),
            Heat::Hot => Some("zb-system-hot-symbolic"),
        }
    }
}

/// Cumulative jiffies from the aggregate `cpu` line of `/proc/stat`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CpuTimes {
    total: u64,
    idle: u64,
}

impl CpuTimes {
    fn parse(stat: &str) -> Option<Self> {
        let line = stat.lines().find(|l| l.starts_with("cpu "))?;
        let fields: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        // user nice system idle iowait irq softirq steal; guest time is already in user/nice.
        let total = fields.iter().take(8).sum();
        let idle = fields.get(3)? + fields.get(4).unwrap_or(&0);
        Some(Self { total, idle })
    }

    /// Busy percentage between `earlier` and `self`.
    fn usage_since(self, earlier: CpuTimes) -> f64 {
        let total = self.total.saturating_sub(earlier.total);
        let idle = self.idle.saturating_sub(earlier.idle);
        if total == 0 {
            return 0.0;
        }
        100.0 * total.saturating_sub(idle) as f64 / total as f64
    }
}

/// Used percentage from `/proc/meminfo`: everything not `MemAvailable`.
fn memory_usage(meminfo: &str) -> Option<f64> {
    let field = |name: &str| {
        meminfo.lines().find_map(|line| {
            let rest = line.strip_prefix(name)?.strip_prefix(':')?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
    };
    let total = field("MemTotal").filter(|&t| t > 0)?;
    let available = field("MemAvailable")?.min(total);
    Some(100.0 * (total - available) as f64 / total as f64)
}

/// Degrees Celsius from a sysfs millidegree reading, if plausible.
fn celsius(millidegrees: &str) -> Option<f64> {
    let value = millidegrees.trim().parse::<i64>().ok()? as f64 / 1000.0;
    PLAUSIBLE.contains(&value).then_some(value)
}

/// A whole number right-aligned to three digit widths.
fn pad(value: f64) -> String {
    let digits = format!("{}", value.round() as i64);
    let fill = 3usize.saturating_sub(digits.chars().count());
    std::iter::repeat_n(FIGURE_SPACE, fill)
        .chain(digits.chars())
        .collect()
}
