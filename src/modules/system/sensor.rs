//! Finding the CPU temperature file, once, at startup. After that only that one
//! file is read per tick.
//!
//! Takes the sysfs root as a parameter so it can be tested against fixture trees.

use std::fs;
use std::path::{Path, PathBuf};

/// hwmon drivers that measure the CPU, most specific first.
const CPU_DRIVERS: [&str; 3] = ["k10temp", "coretemp", "zenpower"];

/// Channel labels for the whole-CPU reading, preferred over per-core or per-CCD ones.
const PACKAGE_LABELS: [&str; 3] = ["Tdie", "Tctl", "Package id"];

/// The temperature file to read each tick, if any.
///
/// `pinned` is either an absolute path to a temperature file, or a hwmon chip
/// name / thermal zone type. A pinned sensor that cannot be found yields `None`
/// rather than a silent guess.
pub fn discover(sys: &Path, pinned: Option<&str>) -> Option<PathBuf> {
    match pinned {
        Some(path) if path.starts_with('/') => {
            let path = PathBuf::from(path);
            path.is_file().then_some(path)
        }
        Some(name) => hwmon_named(sys, name).or_else(|| zone_typed(sys, |t| t == name)),
        None => CPU_DRIVERS
            .iter()
            .find_map(|driver| hwmon_named(sys, driver))
            .or_else(|| zone_typed(sys, is_cpu_zone)),
    }
}

fn hwmon_named(sys: &Path, name: &str) -> Option<PathBuf> {
    entries(&sys.join("class/hwmon"), "hwmon")
        .into_iter()
        .filter(|dir| read_trimmed(&dir.join("name")).as_deref() == Some(name))
        .find_map(|dir| package_input(&dir))
}

/// The chip's whole-CPU input, else its lowest-numbered input.
fn package_input(chip: &Path) -> Option<PathBuf> {
    let mut inputs: Vec<(u32, PathBuf)> = fs::read_dir(chip)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let n = name
                .to_str()?
                .strip_prefix("temp")?
                .strip_suffix("_input")?;
            Some((n.parse().ok()?, entry.path()))
        })
        .collect();
    inputs.sort();

    let label = |n: u32| read_trimmed(&chip.join(format!("temp{n}_label")));
    PACKAGE_LABELS
        .iter()
        .find_map(|wanted| {
            inputs
                .iter()
                .find(|(n, _)| label(*n).is_some_and(|l| l.starts_with(wanted)))
        })
        .or(inputs.first())
        .map(|(_, path)| path.clone())
}

fn zone_typed(sys: &Path, matches: impl Fn(&str) -> bool) -> Option<PathBuf> {
    entries(&sys.join("class/thermal"), "thermal_zone")
        .into_iter()
        .filter(|zone| read_trimmed(&zone.join("type")).is_some_and(|t| matches(&t)))
        .map(|zone| zone.join("temp"))
        .find(|temp| temp.is_file())
}

/// Thermal zone types that report the CPU (x86 package, ARM `cpu-thermal`, …).
fn is_cpu_zone(kind: &str) -> bool {
    kind == "x86_pkg_temp"
        || kind.contains("cpu")
        || CPU_DRIVERS.contains(&kind)
        || kind.contains("package")
}

/// `<prefix>N` entries of `dir`, in numeric order (hwmon10 after hwmon2).
fn entries(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut numbered: Vec<(u32, PathBuf)> = read
        .flatten()
        .filter_map(|entry| {
            let n = entry
                .file_name()
                .to_str()?
                .strip_prefix(prefix)?
                .parse()
                .ok()?;
            Some((n, entry.path()))
        })
        .collect();
    numbered.sort();
    numbered.into_iter().map(|(_, path)| path).collect()
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}
