use chrono::Local;
use glib;
use gtk4::prelude::*;
use gtk4::Label;
use std::time::Duration;

/// Create a clock label that ticks every second.
///
/// The returned `Label` updates itself via a `glib::timeout_add_local` timer
/// so it always shows the current time in the requested `format`.
pub fn create(format: &str) -> Label {
    let label = Label::new(None);
    label.add_css_class("zenith-module");
    label.add_css_class("zenith-module-surface");
    label.add_css_class("zenith-module-center");

    // Immediately show the current time so there's no blank frame.
    let now = Local::now().format(format).to_string();
    label.set_label(&now);

    // Keep an owned copy of the format string for the closure.
    let fmt = format.to_owned();
    let tick_seconds = if format_uses_seconds(format) { 1 } else { 60 };

    // Tick at 1s only when needed; otherwise once per minute.
    let weak_label = label.downgrade();
    glib::timeout_add_local(Duration::from_secs(tick_seconds), move || {
        if let Some(lbl) = weak_label.upgrade() {
            let text = Local::now().format(&fmt).to_string();
            lbl.set_label(&text);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    label
}

fn format_uses_seconds(format: &str) -> bool {
    format.contains("%S") || format.contains("%T") || format.contains("%X")
}
