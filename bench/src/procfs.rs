//! Parsing of `/proc/<pid>` text into point-in-time process counters.

use crate::measure::Sample;
use std::io;

/// Read a live [`Sample`] for `pid`, summing context switches over all threads.
pub fn sample(pid: u32) -> io::Result<Sample> {
    let base = format!("/proc/{pid}");
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("{base}: cannot parse {what}"));

    let cpu_ticks = parse_cpu_ticks(&std::fs::read_to_string(format!("{base}/stat"))?).ok_or_else(|| bad("stat"))?;
    let rss_kib = parse_rss_kib(&std::fs::read_to_string(format!("{base}/status"))?).ok_or_else(|| bad("VmRSS"))?;

    let mut context_switches = 0;
    for task in std::fs::read_dir(format!("{base}/task"))? {
        // A thread may exit between readdir and read; that's fine, skip it.
        if let Ok(status) = std::fs::read_to_string(task?.path().join("status")) {
            context_switches += parse_context_switches(&status).unwrap_or(0);
        }
    }

    Ok(Sample { cpu_ticks, rss_kib, context_switches })
}

/// User + system CPU time, in clock ticks, from a `/proc/<pid>/stat` line.
///
/// For a thread-group leader these totals cover every thread in the process.
pub fn parse_cpu_ticks(stat: &str) -> Option<u64> {
    // comm (field 2) may contain spaces and parens, so split after the last ')'.
    let rest = &stat[stat.rfind(')')? + 1..];
    let mut fields = rest.split_whitespace();
    // `rest` starts at field 3 (state); utime and stime are fields 14 and 15.
    let utime: u64 = fields.nth(14 - 3)?.parse().ok()?;
    let stime: u64 = fields.next()?.parse().ok()?;
    Some(utime + stime)
}

/// Resident set size in KiB, from the `VmRSS:` line of `/proc/<pid>/status`.
pub fn parse_rss_kib(status: &str) -> Option<u64> {
    status_field(status, "VmRSS")
}

/// Voluntary + involuntary context switches from a `/proc/<pid>[/task/<tid>]/status`.
///
/// Each time a sleeping thread wakes up and goes back to sleep it records one
/// voluntary switch, so the sum over all threads is our proxy for wakeups.
pub fn parse_context_switches(status: &str) -> Option<u64> {
    Some(
        status_field(status, "voluntary_ctxt_switches")?
            + status_field(status, "nonvoluntary_ctxt_switches")?,
    )
}

fn status_field(status: &str, key: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if k != key {
            return None;
        }
        v.split_whitespace().next()?.parse().ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real-shaped /proc/<pid>/stat line; the comm contains a space and a
    // closing paren to prove we split on the *last* ')'. utime=137, stime=42.
    const STAT: &str = "4242 (zenith) bar) S 1 4242 4242 0 -1 4194560 9001 0 12 0 137 42 0 0 20 0 3 0 123456 812345678 15000 18446744073709551615 1 1 0 0 0 0 0 4096 17663 0 0 0 17 3 0 0 0 0 0\n";

    #[test]
    fn cpu_ticks_is_user_plus_system_time() {
        assert_eq!(parse_cpu_ticks(STAT), Some(179));
    }

    const STATUS: &str = "\
Name:\tzenith
Umask:\t0022
State:\tS (sleeping)
VmPeak:\t  912000 kB
VmSize:\t  812345 kB
VmHWM:\t   65000 kB
VmRSS:\t   61234 kB
RssAnon:\t   20000 kB
Threads:\t3
voluntary_ctxt_switches:\t1500
nonvoluntary_ctxt_switches:\t25
";

    #[test]
    fn rss_comes_from_vmrss_in_kib() {
        assert_eq!(parse_rss_kib(STATUS), Some(61234));
    }

    #[test]
    fn context_switches_count_voluntary_and_involuntary() {
        assert_eq!(parse_context_switches(STATUS), Some(1525));
    }

    #[test]
    fn live_sample_counts_wakeups_of_every_thread_not_just_the_main_one() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (done_tx, done_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            for _ in 0..50 {
                std::thread::sleep(Duration::from_millis(1));
            }
            done_tx.send(()).unwrap();
            let _ = exit_rx.recv(); // stay alive while we sample
        });
        done_rx.recv().unwrap();

        let pid = std::process::id();
        let sample = sample(pid).expect("sample own process");
        let leader_only =
            parse_context_switches(&std::fs::read_to_string("/proc/self/status").unwrap()).unwrap();

        exit_tx.send(()).unwrap();
        worker.join().unwrap();

        assert!(sample.rss_kib > 0);
        assert!(
            sample.context_switches >= leader_only + 50,
            "process total {} should include the worker's 50+ wakeups on top of the leader's {}",
            sample.context_switches,
            leader_only
        );
    }
}
