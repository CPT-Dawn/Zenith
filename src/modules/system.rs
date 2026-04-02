use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use sysinfo::System;

const REFRESH_INTERVAL_SECONDS: u64 = 2;
const TEMP_READ_EVERY_TICKS: u8 = 5;

/// Create a system stats widget showing real-time CPU, memory, and temperature.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 12);
    container.set_halign(gtk4::Align::End);
    container.add_css_class("zenith-module-surface");

    // CPU label with Nerd Font Icon
    let cpu_label = Label::new(Some(" CPU: --%"));
    cpu_label.add_css_class("zenith-module");
    cpu_label.add_css_class("zenith-module-right");
    container.append(&cpu_label);

    // Memory label with Nerd Font Icon
    let mem_label = Label::new(Some("  MEM: --%"));
    mem_label.add_css_class("zenith-module");
    mem_label.add_css_class("zenith-module-right");
    container.append(&mem_label);

    // Temperature label with Nerd Font Icon
    let temp_label = Label::new(Some(" --°C"));
    temp_label.add_css_class("zenith-module");
    temp_label.add_css_class("zenith-module-right");
    temp_label.add_css_class("zenith-module-temp");
    container.append(&temp_label);

    // Shared system state (Using new() instead of new_all() to save memory)
    let sys = Rc::new(RefCell::new(System::new()));
    let temp_state = Rc::new(RefCell::new((String::new(), 0_u8, None::<f64>)));

    // Update system usage every 2 seconds; temperature is sampled less often.
    {
        let sys = Rc::clone(&sys);
        let temp_state = Rc::clone(&temp_state);
        let cpu_label = cpu_label.downgrade();
        let mem_label = mem_label.downgrade();
        let temp_label = temp_label.downgrade();

        glib::timeout_add_local(Duration::from_secs(REFRESH_INTERVAL_SECONDS), move || {
            // Stop refreshing once all labels are gone (widget destroyed).
            let cpu_label = cpu_label.upgrade();
            let mem_label = mem_label.upgrade();
            let temp_label = temp_label.upgrade();

            if cpu_label.is_none() && mem_label.is_none() && temp_label.is_none() {
                return glib::ControlFlow::Break;
            }

            let mut sys = sys.borrow_mut();

            // PERFORMANCE FIX: Only refresh exactly what we need
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            // CPU usage (sysinfo 0.30+ syntax)
            if let Some(lbl) = cpu_label {
                let cpu_pct = sys.global_cpu_usage().clamp(0.0, 100.0);
                lbl.set_label(&format!(" {cpu_pct:>3.0}%")); // Pad to 3 chars to stop UI jitter
            }

            // Memory usage
            if let Some(lbl) = mem_label {
                let total = sys.total_memory();
                let used = sys.used_memory();
                let mem_pct = if total > 0 {
                    used.saturating_mul(100) / total
                } else {
                    0
                };
                let mem_pct = mem_pct.min(100);
                lbl.set_label(&format!("  {mem_pct:>3}%"));
            }

            // Temperature
            if let Some(lbl) = temp_label {
                let mut state = temp_state.borrow_mut();
                state.1 = state.1.wrapping_add(1);
                if state.1 >= TEMP_READ_EVERY_TICKS {
                    state.1 = 0;
                    state.2 = read_cpu_temperature();
                }

                if let Some(temp) = state.2 {
                    let class = if temp < 50.0 {
                        "zenith-module-temp-cool"
                    } else if temp < 75.0 {
                        "zenith-module-temp-warm"
                    } else {
                        "zenith-module-temp-hot"
                    };

                    if state.0 != class {
                        if !state.0.is_empty() {
                            lbl.remove_css_class(&state.0);
                        }
                        lbl.add_css_class(class);
                        state.0 = class.to_string();
                    }

                    lbl.set_label(&format!(" {temp:>3.0}°C"));
                } else {
                    if !state.0.is_empty() {
                        lbl.remove_css_class(&state.0);
                        state.0.clear();
                    }
                    lbl.set_label(" --°C");
                }
            }

            glib::ControlFlow::Continue
        });
    }

    container
}

/// Read CPU temperature from sysfs (/sys/class/thermal).
fn read_cpu_temperature() -> Option<f64> {
    let thermal_zone_dir = Path::new("/sys/class/thermal");
    if !thermal_zone_dir.exists() {
        return None;
    }

    let mut cpu_temp: Option<f64> = None;
    let mut fallback_temp: Option<f64> = None;

    if let Ok(entries) = fs::read_dir(thermal_zone_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().unwrap_or_default();
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with("thermal_zone") {
                        let Some(temp) = read_zone_temp(&path) else {
                            continue;
                        };
                        let zone_type = fs::read_to_string(path.join("type"))
                            .unwrap_or_default()
                            .to_lowercase();

                        if is_cpu_zone_type(&zone_type) {
                            cpu_temp = Some(cpu_temp.map_or(temp, |current| current.max(temp)));
                        } else {
                            fallback_temp =
                                Some(fallback_temp.map_or(temp, |current| current.max(temp)));
                        }
                    }
                }
            }
        }
    }

    cpu_temp.or(fallback_temp)
}

fn read_zone_temp(path: &Path) -> Option<f64> {
    let contents = fs::read_to_string(path.join("temp")).ok()?;
    let millidegrees = contents.trim().parse::<f64>().ok()?;
    let temp_celsius = millidegrees / 1000.0;

    if (0.0..150.0).contains(&temp_celsius) {
        Some(temp_celsius)
    } else {
        None
    }
}

fn is_cpu_zone_type(zone_type: &str) -> bool {
    let ty = zone_type.trim();

    ty.contains("cpu")
        || ty.contains("x86_pkg_temp")
        || ty.contains("coretemp")
        || ty.contains("k10temp")
        || ty.contains("package")
}
