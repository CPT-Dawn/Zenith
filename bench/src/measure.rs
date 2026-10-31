//! Turning two process samples taken a measurement window apart into the
//! numbers the harness reports.

use std::collections::HashMap;
use std::time::Duration;

/// Cumulative counters for one process at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// User + system CPU time of the whole process, in clock ticks.
    pub cpu_ticks: u64,
    /// Resident set size, KiB.
    pub rss_kib: u64,
    /// Proportional set size, KiB.
    pub pss_kib: u64,
}

/// Thread wakeups over a window, from per-thread context-switch counters
/// sampled every so often.
///
/// Counting per thread matters: a thread that exits takes its counter with
/// it, so a plain "sum now minus sum then" goes down (or to zero) whenever an
/// idle pool thread retires. Only the switches an exiting thread made since
/// the last sample are lost, so sample often (the harness uses 1 s).
#[derive(Debug, Default)]
pub struct Wakeups {
    last: Option<HashMap<u32, u64>>,
    total: u64,
}

impl Wakeups {
    /// Feed `(thread id, voluntary + involuntary switches)` for every live
    /// thread. The first call is the baseline.
    pub fn sample(&mut self, threads: &[(u32, u64)]) {
        let now: HashMap<u32, u64> = threads.iter().copied().collect();
        if let Some(last) = &self.last {
            for (tid, count) in &now {
                // A thread born since the last sample did all its switching inside the window.
                self.total += count.saturating_sub(last.get(tid).copied().unwrap_or(0));
            }
        }
        self.last = Some(now);
    }

    pub fn total(&self) -> u64 {
        self.total
    }
}

/// What the harness reports for one run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measurements {
    /// Resident memory at the end of the window, MiB.
    pub rss_mib: f64,
    /// Proportional memory (shared pages split between their users) at the end of the window, MiB.
    pub pss_mib: f64,
    /// Average CPU use over the window, percent of one core.
    pub idle_cpu_percent: f64,
    /// Thread wakeups per second over the window.
    pub wakeups_per_sec: f64,
    /// Time from launch to the first committed frame, ms (None if not seen).
    pub first_frame_ms: Option<f64>,
}

/// Compare two samples taken `window` apart, with the [`Wakeups`] counted
/// between them. `ticks_per_sec` is `CLK_TCK`.
pub fn measure(before: &Sample, after: &Sample, wakeups: u64, window: Duration, ticks_per_sec: u64) -> Measurements {
    let secs = window.as_secs_f64();
    let cpu_secs = after.cpu_ticks.saturating_sub(before.cpu_ticks) as f64 / ticks_per_sec as f64;
    let switches = wakeups as f64;
    Measurements {
        rss_mib: after.rss_kib as f64 / 1024.0,
        pss_mib: after.pss_kib as f64 / 1024.0,
        idle_cpu_percent: cpu_secs / secs * 100.0,
        wakeups_per_sec: switches / secs,
        first_frame_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakeups_of_a_thread_that_exits_mid_window_are_not_subtracted() {
        // GLib's pool threads exit after a while idle. Summing the counters
        // of whichever threads are alive at each end would subtract their
        // whole history; tracking per thread keeps every switch we saw.
        let mut wakeups = Wakeups::default();
        wakeups.sample(&[(1, 100), (7, 5_000)]); // baseline
        wakeups.sample(&[(1, 103), (7, 5_002)]); // +3 +2
        wakeups.sample(&[(1, 104)]); // thread 7 exited: +1
        wakeups.sample(&[(1, 106), (9, 4)]); // thread 9 born mid-window: +2 +4
        assert_eq!(wakeups.total(), 12);
    }

    #[test]
    fn idle_window_yields_cpu_percent_wakeups_and_rss() {
        let before = Sample { cpu_ticks: 100, rss_kib: 50_000, pss_kib: 20_000 };
        let after = Sample { cpu_ticks: 103, rss_kib: 51_200, pss_kib: 20_480 };

        let m = measure(&before, &after, 300, Duration::from_secs(30), 100);

        // 3 ticks at 100 Hz = 30 ms of CPU over 30 s = 0.1 %.
        assert!((m.idle_cpu_percent - 0.1).abs() < 1e-9, "{}", m.idle_cpu_percent);
        // 300 switches over 30 s.
        assert!((m.wakeups_per_sec - 10.0).abs() < 1e-9, "{}", m.wakeups_per_sec);
        // RSS is the end-of-window value: 51 200 KiB = 50 MiB.
        assert!((m.rss_mib - 50.0).abs() < 1e-9, "{}", m.rss_mib);
        // PSS likewise: 20 480 KiB = 20 MiB.
        assert!((m.pss_mib - 20.0).abs() < 1e-9, "{}", m.pss_mib);
    }
}
