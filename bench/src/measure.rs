//! Turning two process samples taken a measurement window apart into the
//! numbers the harness reports.

use std::time::Duration;

/// Cumulative counters for one process at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// User + system CPU time of the whole process, in clock ticks.
    pub cpu_ticks: u64,
    /// Resident set size, KiB.
    pub rss_kib: u64,
    /// Context switches summed over every thread.
    pub context_switches: u64,
}

/// What the harness reports for one run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurements {
    /// Resident memory at the end of the window, MiB.
    pub rss_mib: f64,
    /// Average CPU use over the window, percent of one core.
    pub idle_cpu_percent: f64,
    /// Thread wakeups per second over the window.
    pub wakeups_per_sec: f64,
    /// Time from launch to the first committed frame, ms (None if not seen).
    pub first_frame_ms: Option<f64>,
}

/// Compare two samples taken `window` apart. `ticks_per_sec` is `CLK_TCK`.
pub fn measure(before: &Sample, after: &Sample, window: Duration, ticks_per_sec: u64) -> Measurements {
    let secs = window.as_secs_f64();
    let cpu_secs = after.cpu_ticks.saturating_sub(before.cpu_ticks) as f64 / ticks_per_sec as f64;
    let switches = after.context_switches.saturating_sub(before.context_switches) as f64;
    Measurements {
        rss_mib: after.rss_kib as f64 / 1024.0,
        idle_cpu_percent: cpu_secs / secs * 100.0,
        wakeups_per_sec: switches / secs,
        first_frame_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_window_yields_cpu_percent_wakeups_and_rss() {
        let before = Sample { cpu_ticks: 100, rss_kib: 50_000, context_switches: 1_000 };
        let after = Sample { cpu_ticks: 103, rss_kib: 51_200, context_switches: 1_300 };

        let m = measure(&before, &after, Duration::from_secs(30), 100);

        // 3 ticks at 100 Hz = 30 ms of CPU over 30 s = 0.1 %.
        assert!((m.idle_cpu_percent - 0.1).abs() < 1e-9, "{}", m.idle_cpu_percent);
        // 300 switches over 30 s.
        assert!((m.wakeups_per_sec - 10.0).abs() < 1e-9, "{}", m.wakeups_per_sec);
        // RSS is the end-of-window value: 51 200 KiB = 50 MiB.
        assert!((m.rss_mib - 50.0).abs() < 1e-9, "{}", m.rss_mib);
    }
}
