use pretty_assertions::assert_eq;
use zenith_bar::modules::system::model::{Readings, SystemModel};
use zenith_bar::modules::system::{Metric, SystemConfig};

/// U+2007 FIGURE SPACE: as wide as a digit, so padded numbers never change width.
const FS: char = '\u{2007}';

/// `/proc/stat` with the aggregate line: user nice system idle iowait irq softirq steal guest guest_nice.
fn stat(user: u64, system: u64, idle: u64, iowait: u64) -> String {
    format!(
        "cpu  {user} 0 {system} {idle} {iowait} 0 0 0 0 0\ncpu0 1 2 3 4 5 6 7 8 0 0\nintr 12345 0 0\nctxt 999\n"
    )
}

fn only(metrics: &[Metric]) -> SystemConfig {
    SystemConfig {
        metrics: metrics.to_vec(),
        ..SystemConfig::default()
    }
}

fn cpu(stat: &str) -> Readings<'_> {
    Readings {
        stat: Some(stat),
        ..Readings::default()
    }
}

#[test]
fn cpu_usage_is_the_busy_share_of_the_time_between_two_samples() {
    let config = only(&[Metric::Cpu]);
    let mut model = SystemModel::default();
    model.update(&cpu(&stat(100, 100, 700, 100)), &config);

    // 1000 jiffies later: 150 user + 100 system busy, 700 idle + 50 iowait idle → 25 %.
    let state = model.update(&cpu(&stat(250, 200, 1400, 150)), &config);

    assert_eq!(state.label, format!("CPU {FS}25%"));
}

const MEMINFO: &str = "MemTotal:       16000000 kB
MemFree:         2000000 kB
MemAvailable:    6000000 kB
Buffers:          500000 kB
Cached:          3000000 kB
SwapTotal:       8000000 kB
";

fn mem(meminfo: &str) -> Readings<'_> {
    Readings {
        meminfo: Some(meminfo),
        ..Readings::default()
    }
}

#[test]
fn memory_is_what_is_not_available_out_of_the_total() {
    let state = SystemModel::default().update(&mem(MEMINFO), &only(&[Metric::Memory]));

    // (16 000 000 − 6 000 000) / 16 000 000 = 62.5 % → 63 %.
    assert_eq!(state.label, format!("MEM {FS}63%"));
}

#[test]
fn meminfo_without_a_total_hides_the_memory_metric() {
    let state =
        SystemModel::default().update(&mem("MemAvailable: 100 kB\n"), &only(&[Metric::Memory]));

    assert_eq!(state.label, "");
}

fn temp(millidegrees: &str) -> Readings<'_> {
    Readings {
        temperature: Some(millidegrees),
        ..Readings::default()
    }
}

#[test]
fn temperature_is_read_in_millidegrees_and_shown_in_celsius() {
    let state = SystemModel::default().update(&temp("54250\n"), &only(&[Metric::Temperature]));

    assert_eq!(state.label, format!("{FS}54°C"));
    assert!(state.classes.is_empty());
    assert_eq!(state.icon, None);
}

#[test]
fn crossing_thresholds_switches_state_by_class_icon_and_tooltip() {
    // Defaults: warm at 70 °C, hot at 85 °C.
    let config = only(&[Metric::Temperature]);
    let mut model = SystemModel::default();

    let cool = model.update(&temp("69000"), &config);
    let warm = model.update(&temp("70000"), &config);
    let hot = model.update(&temp("85000"), &config);
    let cooled = model.update(&temp("60000"), &config);

    assert_eq!(cool.classes, Vec::<&str>::new());
    assert_eq!(warm.classes, vec!["zb-warm"]);
    assert_eq!(hot.classes, vec!["zb-hot"]);
    assert_eq!(cooled.classes, Vec::<&str>::new());

    // Not by colour alone: each state has its own icon shape and words.
    assert_eq!(cool.icon, None);
    assert_eq!(warm.icon.as_deref(), Some("zb-system-warm-symbolic"));
    assert_eq!(hot.icon.as_deref(), Some("zb-system-hot-symbolic"));
    assert_eq!(cooled.icon, None);
    assert!(warm.tooltip.unwrap().contains("warm"));
    assert!(hot.tooltip.unwrap().contains("hot"));
}

#[test]
fn thresholds_come_from_config() {
    let mut config = only(&[Metric::Temperature]);
    config.temperature.warm = 50;
    config.temperature.hot = 60;

    let state = SystemModel::default().update(&temp("55000"), &config);

    assert_eq!(state.classes, vec!["zb-warm"]);
}

#[test]
fn metrics_appear_in_the_configured_order() {
    let stat = stat(10, 10, 80, 0);
    let readings = Readings {
        stat: Some(&stat),
        meminfo: Some(MEMINFO),
        temperature: Some("42000"),
    };
    let config = only(&[Metric::Temperature, Metric::Cpu, Metric::Memory]);

    let state = SystemModel::default().update(&readings, &config);

    // The first sample counts from boot: 20 busy out of 100.
    assert_eq!(state.label, format!("{FS}42°C  CPU {FS}20%  MEM {FS}63%"));
}

#[test]
fn without_a_sensor_temperature_quietly_drops_out() {
    let stat = stat(10, 10, 80, 0);
    let readings = Readings {
        stat: Some(&stat),
        meminfo: Some(MEMINFO),
        temperature: None,
    };

    let state = SystemModel::default().update(&readings, &SystemConfig::default());

    assert_eq!(state.label, format!("CPU {FS}20%  MEM {FS}63%"));
    assert!(state.visible);
    assert!(state.classes.is_empty());
}

#[test]
fn garbage_or_implausible_temperatures_are_ignored() {
    let config = only(&[Metric::Cpu, Metric::Temperature]);
    let stat = stat(10, 10, 80, 0);
    for bad in ["", "n/a", "-5000", "200000"] {
        let readings = Readings {
            stat: Some(&stat),
            temperature: Some(bad),
            ..Readings::default()
        };
        let state = SystemModel::default().update(&readings, &config);
        assert_eq!(state.label, format!("CPU {FS}20%"), "{bad:?}");
    }
}

#[test]
fn with_nothing_readable_the_module_hides() {
    let state = SystemModel::default().update(&Readings::default(), &SystemConfig::default());

    assert!(!state.visible);
}

#[test]
fn a_stalled_counter_reads_as_idle_not_as_nan() {
    let config = only(&[Metric::Cpu]);
    let mut model = SystemModel::default();
    let same = stat(10, 10, 80, 0);
    model.update(&cpu(&same), &config);

    let state = model.update(&cpu(&same), &config);

    assert_eq!(state.label, format!("CPU {FS}{FS}0%"));
}
