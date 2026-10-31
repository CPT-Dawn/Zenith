//! Performance budgets: ceilings a run must stay at or under.
//!
//! A budget file is a flat list of `metric = number` lines (a TOML subset, so
//! TOML tooling reads it too). `#` starts a comment. Every metric is optional.

use crate::measure::Measurements;

/// Ceilings for each measurement; `None` means "not budgeted".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Budget {
    pub rss_mib: Option<f64>,
    pub pss_mib: Option<f64>,
    pub idle_cpu_percent: Option<f64>,
    pub wakeups_per_sec: Option<f64>,
    pub first_frame_ms: Option<f64>,
}

impl Budget {
    /// A flat budget: every line applies.
    pub fn parse(text: &str) -> Result<Budget, String> {
        Budget::parse_for(text, None)
    }

    /// Top-level lines apply to every run; a `[preset]` table adds or
    /// overrides ceilings for that preset only. Asking for a preset the file
    /// has no table for is an error once the file has any tables, so a typo
    /// can't quietly fall back to the shared ceilings.
    pub fn parse_for(text: &str, preset: Option<&str>) -> Result<Budget, String> {
        let mut budget = Budget::default();
        let mut sections = Vec::new();
        let mut section: Option<String> = None;
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let at = |msg: String| format!("budget line {}: {msg}", n + 1);
            if let Some(name) = line.strip_prefix('[') {
                let name = name.strip_suffix(']').ok_or_else(|| at(format!("expected `[preset]`, got `{line}`")))?;
                sections.push(name.trim().to_owned());
                section = Some(name.trim().to_owned());
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| at(format!("expected `metric = number`, got `{line}`")))?;
            let (key, value) = (key.trim(), value.trim());
            let value: f64 = value.parse().map_err(|_| at(format!("`{value}` is not a number")))?;
            let slot = match key {
                "rss_mib" => &mut budget.rss_mib,
                "pss_mib" => &mut budget.pss_mib,
                "idle_cpu_percent" => &mut budget.idle_cpu_percent,
                "wakeups_per_sec" => &mut budget.wakeups_per_sec,
                "first_frame_ms" => &mut budget.first_frame_ms,
                other => {
                    return Err(at(format!(
                        "unknown metric `{other}` (expected rss_mib, pss_mib, idle_cpu_percent, wakeups_per_sec or first_frame_ms)"
                    )));
                }
            };
            if section.is_none() || section.as_deref() == preset {
                *slot = Some(value);
            }
        }
        if let Some(p) = preset
            && !sections.is_empty()
            && !sections.iter().any(|s| s == p)
        {
            return Err(format!("budget has no table for preset `{p}` (it has: {})", sections.join(", ")));
        }
        Ok(budget)
    }

    /// Every budgeted metric the run exceeded (or failed to produce).
    pub fn check(&self, m: &Measurements) -> Vec<Violation> {
        [
            ("rss_mib", self.rss_mib, Some(m.rss_mib)),
            ("pss_mib", self.pss_mib, Some(m.pss_mib)),
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
            Budget { rss_mib: Some(60.0), wakeups_per_sec: Some(2.5), ..Budget::default() }
        );
    }

    #[test]
    fn preset_table_overrides_the_shared_ceilings_for_that_preset_only() {
        let text = "\
first_frame_ms = 500        # every run
rss_mib = 250

[minimal]
rss_mib = 200

[bloom]
wakeups_per_sec = 3
";
        assert_eq!(
            Budget::parse_for(text, Some("minimal")).unwrap(),
            Budget { rss_mib: Some(200.0), first_frame_ms: Some(500.0), ..Budget::default() }
        );
        assert_eq!(
            Budget::parse_for(text, Some("bloom")).unwrap(),
            Budget {
                rss_mib: Some(250.0),
                wakeups_per_sec: Some(3.0),
                first_frame_ms: Some(500.0),
                ..Budget::default()
            }
        );
        // No preset: only the shared ceilings.
        assert_eq!(
            Budget::parse_for(text, None).unwrap(),
            Budget { rss_mib: Some(250.0), first_frame_ms: Some(500.0), ..Budget::default() }
        );
    }

    #[test]
    fn preset_missing_from_a_sectioned_budget_is_an_error() {
        // A typo in CI must not silently fall back to looser ceilings.
        let err = Budget::parse_for("rss_mib = 1\n[bloom]\nrss_mib = 2\n", Some("blom")).unwrap_err();
        assert!(err.contains("`blom`") && err.contains("bloom"), "{err}");
    }

    #[test]
    fn proportional_memory_can_be_budgeted() {
        let budget = Budget::parse("pss_mib = 40\n").unwrap();
        assert_eq!(budget, Budget { pss_mib: Some(40.0), ..Budget::default() });
        let m = Measurements { pss_mib: 41.0, ..run(10.0, Some(1.0)) };
        assert_eq!(budget.check(&m), vec![Violation { metric: "pss_mib", measured: Some(41.0), limit: 40.0 }]);
    }

    #[test]
    fn misspelled_metric_is_rejected_with_its_line() {
        let err = Budget::parse("rss_mib = 60\nwakeups = 2\n").unwrap_err();
        assert!(err.contains("line 2") && err.contains("`wakeups`"), "{err}");
    }

    fn run(rss_mib: f64, first_frame_ms: Option<f64>) -> Measurements {
        Measurements { rss_mib, pss_mib: 1.0, idle_cpu_percent: 0.05, wakeups_per_sec: 1.0, first_frame_ms }
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
