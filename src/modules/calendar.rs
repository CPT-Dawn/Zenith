use chrono::Local;
use glib;
use gtk4::prelude::*;
use gtk4::{Button, Calendar, Popover};
use std::time::Duration;

/// Create a clickable date button that opens a slide-down calendar popover.
///
/// Displays the current date as "21 Feb". Clicking it toggles a popover
/// containing a full GTK4 Calendar widget.
pub fn create() -> Button {
    // The button label *is* the date text – no separate icon.
    let btn = Button::new();
    btn.add_css_class("zenith-calendar-btn");
    btn.add_css_class("zenith-module");
    btn.add_css_class("zenith-module-center");
    update_button_label(&btn);

    // ── Calendar popover ────────────────────────────────────────
    let calendar = Calendar::new();
    calendar.add_css_class("zenith-calendar");

    let popover = Popover::new();
    popover.set_child(Some(&calendar));
    popover.set_autohide(true);
    popover.set_cascade_popdown(true);
    popover.set_has_arrow(false);
    popover.set_position(gtk4::PositionType::Bottom);
    popover.add_css_class("zenith-calendar-popup");
    popover.set_parent(&btn);

    // Toggle on click
    btn.connect_clicked({
        let popover = popover.clone();
        move |_| {
            if popover.is_visible() {
                popover.popdown();
            } else {
                popover.popup();
            }
        }
    });

    // ── Tick every 60 s to keep the date current ────────────────
    let weak_btn = btn.downgrade();
    glib::timeout_add_local(Duration::from_secs(60), move || {
        if let Some(b) = weak_btn.upgrade() {
            update_button_label(&b);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    btn
}

/// Set the button label to the current date in "DD Mon" format.
fn update_button_label(btn: &Button) {
    let now = Local::now();
    btn.set_label(&now.format("%d %b").to_string());
}
