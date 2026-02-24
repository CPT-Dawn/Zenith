use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;
use sysinfo::System;

/// Create a system stats widget showing real-time CPU, memory, and temperature.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 12);
    container.set_halign(gtk4::Align::End);

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
    let temp_label = Label::new(Some(" TEMP: --°C"));
    temp_label.add_css_class("zenith-module");
    temp_label.add_css_class("zenith-module-right");
    container.append(&temp_label);

    // Shared system state (Using new() instead of new_all() to save memory)
    let sys = Rc::new(RefCell::new(System::new()));

    // Update every 2 seconds
    {
        let sys = Rc::clone(&sys);
        let cpu_label = cpu_label.downgrade();
        let mem_label = mem_label.downgrade();
        let temp_label = temp_label.downgrade();

        glib::timeout_add_local(Duration::from_secs(2), move || {
            let mut sys = sys.borrow_mut();

            // PERFORMANCE FIX: Only refresh exactly what we need
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            // CPU usage (sysinfo 0.30+ syntax)
            if let Some(lbl) = cpu_label.upgrade() {
                let cpu_pct = (sys.global_cpu_usage() as i32).min(100);
                lbl.set_label(&format!(" {:>3}%", cpu_pct)); // Pad to 3 chars to stop UI jitter
            }

            // Memory usage
            if let Some(lbl) = mem_label.upgrade() {
                let total = sys.total_memory();
                let used = sys.used_memory();
                let mem_pct = if total > 0 {
                    ((used as f64 / total as f64) * 100.0) as i32
                } else {
                    0
                };
                lbl.set_label(&format!("  {:>3}%", mem_pct.min(100)));
            }

            // Temperature
            if let Some(lbl) = temp_label.upgrade() {
                if let Some(temp) = read_cpu_temperature() {
                    lbl.set_label(&format!(" {:.0}°C", temp));
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

    let mut max_temp: f64 = 0.0;
    let mut found_any = false;

    if let Ok(entries) = fs::read_dir(thermal_zone_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().unwrap_or_default();
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with("thermal_zone") {
                        // OPTIONAL PRO-TIP: You can read the "type" file here to filter out
                        // battery temps or Wi-Fi card temps if your numbers look weird.
                        // e.g., if fs::read_to_string(path.join("type")) == "x86_pkg_temp"

                        let temp_path = path.join("temp");
                        if let Ok(contents) = fs::read_to_string(temp_path) {
                            if let Ok(millidegrees) = contents.trim().parse::<f64>() {
                                let temp_celsius = millidegrees / 1000.0;
                                if temp_celsius > 0.0 && temp_celsius < 150.0 {
                                    max_temp = max_temp.max(temp_celsius);
                                    found_any = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if found_any {
        Some(max_temp)
    } else {
        None
    }
}
