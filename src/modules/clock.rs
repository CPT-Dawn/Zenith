use chrono::Local;
use glib;
use gtk4::prelude::*;
use gtk4::Label;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Create a clock label that ticks every second.
///
/// The returned `Label` updates itself via a `glib::timeout_add_local` timer
/// so it always shows the current time in the requested `format`.
pub fn create(format: &str) -> Label {
    let label = Label::new(None);
    label.add_css_class("zenith-module");
    label.add_css_class("zenith-module-center");

    // Immediately show the current time so there's no blank frame.
    let now = Local::now().format(format).to_string();
    label.set_label(&now);

    // Keep an owned copy of the format string for the closure.
    let fmt = Rc::new(RefCell::new(format.to_owned()));

    // Tick every second.
    let weak_label = label.downgrade();
    let fmt_clone = Rc::clone(&fmt);
    glib::timeout_add_local(Duration::from_secs(1), move || {
        if let Some(lbl) = weak_label.upgrade() {
            let text = Local::now().format(&fmt_clone.borrow()).to_string();
            lbl.set_label(&text);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    label
}
