use chrono::Local;
use glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Calendar, Label, Orientation, Popover};
use std::time::Duration;

/// Create a calendar widget showing the current date.
/// When clicked, opens a popover with a full calendar.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 6);
    container.add_css_class("zenith-module");
    container.add_css_class("zenith-module-center");

    // Date label (e.g. "21 Feb")
    let date_label = Label::new(None);
    update_date_label(&date_label);
    container.append(&date_label);

    // Button to open calendar
    let calendar_btn = Button::new();
    calendar_btn.set_label("📅");
    calendar_btn.add_css_class("zenith-calendar-btn");

    // Create the calendar widget that will be shown in a popover
    let calendar = Calendar::new();
    calendar.add_css_class("zenith-calendar");

    // Create popover for the calendar
    let popover = Popover::new();
    popover.set_child(Some(&calendar));
    popover.set_autohide(true);
    popover.set_cascade_popdown(true);
    popover.set_position(gtk4::PositionType::Bottom);

    // Connect button click to show/hide popover
    calendar_btn.connect_clicked({
        let popover = popover.clone();
        move |_| {
            if popover.is_visible() {
                popover.popdown();
            } else {
                popover.popup();
            }
        }
    });

    popover.set_parent(&calendar_btn);
    let weak_label = date_label.downgrade();
    glib::timeout_add_local(Duration::from_secs(60), move || {
        if let Some(lbl) = weak_label.upgrade() {
            update_date_label(&lbl);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    container
}

/// Update the date label to display current date in "DD Mon" format
fn update_date_label(label: &Label) {
    let now = Local::now();
    let date_str = now.format("%d %b").to_string();
    label.set_label(&date_str);
}
