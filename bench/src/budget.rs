//! Performance budgets: ceilings a run must stay at or under.
//!
//! A budget file is a flat list of `metric = number` lines (a TOML subset, so
//! TOML tooling reads it too). `#` starts a comment. Every metric is optional.

use crate::measure::Measurements;

/// Ceilings for each measurement; `None` means "not budgeted".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Budget {
    pub rss_mib: Option<f64>,
    pub idle_cpu_percent: Option<f64>,
    pub wakeups_per_sec: Option<f64>,
    pub first_frame_ms: Option<f64>,
}

impl Budget {
    pub fn parse(text: &str) -> Result<Budget, String> {
        let mut budget = Budget::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let at = |msg: String| format!("budget line {}: {msg}", n + 1);
            let (key, value) = line.split_once('=').ok_or_else(|| at(format!("expected `metric = number`, got `{line}`")))?;
            let (key, value) = (key.trim(), value.trim());
            let value: f64 = value.parse().map_err(|_| at(format!("`{value}` is not a number")))?;
            let slot = match key {
                "rss_mib" => &mut budget.rss_mib,
                "idle_cpu_percent" => &mut budget.idle_cpu_percent,
                "wakeups_per_sec" => &mut budget.wakeups_per_sec,
                "first_frame_ms" => &mut budget.first_frame_ms,
                other => {
                    return Err(at(format!(
                        "unknown metric `{other}` (expected rss_mib, idle_cpu_percent, wakeups_per_sec or first_frame_ms)"
                    )));
                }
            };
            *slot = Some(value);
        }
        Ok(budget)
    }

    /// Every budgeted metric the run exceeded (or failed to produce).
    pub fn check(&self, m: &Measurements) -> Vec<Violation> {
        [
            ("rss_mib", self.rss_mib, Some(m.rss_mib)),
            ("idle_cpu_percent", self.idle_cpu_percent, Some(m.idle_cpu_percent)),
            ("wakeups_per_sec", self.wakeups_per_sec, Some(m.wakeups_per_sec)),
            ("first_frame_ms", self.first_frame_ms, m.first_frame_ms),
        ]
        .into_iter()
        .filter_map(|(metric, limit, measured)| {
            let limit = limit?;
            match measured {
                Some(v) if v <= limit => None,
                _ => Some(Violation { metric, measured, limit }),
            }
        })
        .collect()
    }
}

/// One budget breach. `measured` is `None` when the metric could not be measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    pub metric: &'static str,
    pub measured: Option<f64>,
    pub limit: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_file_sets_only_the_ceilings_it_names() {
        let text = "\
# minimal preset, idle
rss_mib = 60
wakeups_per_sec = 2.5   # per second, all threads
";
        let budget = Budget::parse(text).unwrap();
        assert_eq!(
            budget,
            Budget { rss_mib: Some(60.0), idle_cpu_percent: None, wakeups_per_sec: Some(2.5), first_frame_ms: None }
        );
    }

    #[test]
    fn misspelled_metric_is_rejected_with_its_line() {
        let err = Budget::parse("rss_mib = 60\nwakeups = 2\n").unwrap_err();
        assert!(err.contains("line 2") && err.contains("`wakeups`"), "{err}");
    }

    fn run(rss_mib: f64, first_frame_ms: Option<f64>) -> Measurements {
        Measurements { rss_mib, idle_cpu_percent: 0.05, wakeups_per_sec: 1.0, first_frame_ms }
    }

    #[test]
    fn measurement_at_the_ceiling_passes_and_over_it_fails() {
        let budget = Budget { rss_mib: Some(60.0), ..Budget::default() };

        assert_eq!(budget.check(&run(60.0, Some(100.0))), vec![]);
        assert_eq!(
            budget.check(&run(61.5, Some(100.0))),
            vec![Violation { metric: "rss_mib", measured: Some(61.5), limit: 60.0 }]
        );
    }

    #[test]
    fn budgeted_first_frame_that_never_arrived_is_a_violation() {
        let budget = Budget { first_frame_ms: Some(500.0), ..Budget::default() };
        assert_eq!(
            budget.check(&run(40.0, None)),
            vec![Violation { metric: "first_frame_ms", measured: None, limit: 500.0 }]
        );
    }
}
