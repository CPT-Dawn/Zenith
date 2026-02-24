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

    // CPU label
    let cpu_label = Label::new(Some("CPU: --%"));
    cpu_label.add_css_class("zenith-module");
    cpu_label.add_css_class("zenith-module-right");
    container.append(&cpu_label);

    // Memory label
    let mem_label = Label::new(Some("MEM: --%"));
    mem_label.add_css_class("zenith-module");
    mem_label.add_css_class("zenith-module-right");
    container.append(&mem_label);

    // Temperature label
    let temp_label = Label::new(Some("TEMP: --°C"));
    temp_label.add_css_class("zenith-module");
    temp_label.add_css_class("zenith-module-right");
    container.append(&temp_label);

    // GPU label (if available)
    let gpu_label = Label::new(Some("GPU: --%"));
    gpu_label.add_css_class("zenith-module");
    gpu_label.add_css_class("zenith-module-right");
    container.append(&gpu_label);

    // Shared system state
    let sys = Rc::new(RefCell::new(System::new_all()));

    // Update every 2 seconds
    {
        let sys = Rc::clone(&sys);
        let cpu_label = cpu_label.downgrade();
        let mem_label = mem_label.downgrade();
        let temp_label = temp_label.downgrade();
        let gpu_label = gpu_label.downgrade();

        glib::timeout_add_local(Duration::from_secs(2), move || {
            let mut sys = sys.borrow_mut();
            sys.refresh_all();

            // CPU usage (global average)
            if let Some(lbl) = cpu_label.upgrade() {
                let cpu_pct = (sys.global_cpu_usage() as i32).min(100);
                lbl.set_label(&format!("CPU: {}%", cpu_pct));
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
                lbl.set_label(&format!("MEM: {}%", mem_pct.min(100)));
            }

            // Temperature (CPU package / highest core)
            if let Some(lbl) = temp_label.upgrade() {
                if let Some(temp) = read_cpu_temperature() {
                    lbl.set_label(&format!("TEMP: {:.0}°C", temp));
                }
            }

            // GPU usage (NVIDIA first, then AMD)
            if let Some(lbl) = gpu_label.upgrade() {
                if let Some(usage) = query_nvidia_gpu() {
                    lbl.set_label(&format!("GPU: {}%", usage));
                } else if let Some(usage) = query_amd_gpu() {
                    lbl.set_label(&format!("GPU: {}%", usage));
                }
            }

            glib::ControlFlow::Continue
        });
    }

    container
}

/// Read CPU temperature from sysfs (/sys/class/thermal).
/// Returns the highest temperature found (in Celsius).
fn read_cpu_temperature() -> Option<f64> {
    let thermal_zone_dir = Path::new("/sys/class/thermal");
    if !thermal_zone_dir.exists() {
        return None;
    }

    let mut max_temp: f64 = 0.0;
    let mut found_any = false;

    // Scan all thermal zones
    if let Ok(entries) = fs::read_dir(thermal_zone_dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Look for thermal_zoneX directories
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default();
                if let Some(name_str) = name.to_str() {
                    if name_str.starts_with("thermal_zone") {
                        // Try to read temperature
                        let temp_path = path.join("temp");
                        if let Ok(contents) = fs::read_to_string(temp_path) {
                            if let Ok(millidegrees) = contents.trim().parse::<f64>() {
                                let temp_celsius = millidegrees / 1000.0;
                                // Focus on actual CPU temperatures (< 150°C sanity check)
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

/// Query NVIDIA GPU usage via `nvidia-smi`.
/// Returns GPU utilization percentage if available.
fn query_nvidia_gpu() -> Option<i32> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let usage_str = stdout.trim().split('\n').next()?;
    usage_str.parse::<i32>().ok()
}

/// Query AMD GPU usage via sysfs or amdgpu metrics.
/// Returns GPU utilization percentage if available.
fn query_amd_gpu() -> Option<i32> {
    // Try to read AMD GPU sysfs metrics
    let amdgpu_path = Path::new("/sys/class/drm/card0/device");

    if !amdgpu_path.exists() {
        return None;
    }

    // Try to read GPU busy percentage (AMD AMDGPU driver)
    let busy_path = amdgpu_path.join("gpu_busy_percent");
    if let Ok(contents) = fs::read_to_string(busy_path) {
        if let Ok(busy) = contents.trim().parse::<i32>() {
            return Some(busy.min(100));
        }
    }

    // Fallback: try card1
    let amdgpu_path_alt = Path::new("/sys/class/drm/card1/device");
    let busy_path_alt = amdgpu_path_alt.join("gpu_busy_percent");
    if let Ok(contents) = fs::read_to_string(busy_path_alt) {
        if let Ok(busy) = contents.trim().parse::<i32>() {
            return Some(busy.min(100));
        }
    }

    None
}
