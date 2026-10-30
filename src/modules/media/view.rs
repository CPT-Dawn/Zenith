use std::rc::Rc;

use gtk4::prelude::*;

use super::source::MediaSource;
use crate::ui::binding::bind;

/// Icon, label and a thin progress line. Left click plays/pauses the followed player,
/// in the bubble phase so a configured `on_click` command takes precedence.
pub fn build(source: &Rc<MediaSource>) -> gtk4::Widget {
    let icon = gtk4::Image::new();
    icon.add_css_class("zb-media-icon");
    let label = gtk4::Label::builder()
        .single_line_mode(true)
        .xalign(0.0)
        .build();
    let progress = gtk4::ProgressBar::builder()
        .css_classes(["zb-media-progress"])
        .build();
    let text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .valign(gtk4::Align::Center)
        .build();
    text.append(&label);
    text.append(&progress);
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    root.append(&icon);
    root.append(&text);

    bind(&root, &source.state(), move |_, state| {
        icon.set_icon_name(state.icon.as_deref());
        label.set_label(&state.label);
        progress.set_visible(state.progress.is_some());
        progress.set_fraction(state.progress.unwrap_or(0.0));
    });

    let click = gtk4::GestureClick::builder()
        .button(gtk4::gdk::BUTTON_PRIMARY)
        .build();
    let weak = Rc::downgrade(source);
    click.connect_pressed(move |_, _, _, _| {
        if let Some(source) = weak.upgrade() {
            source.play_pause();
        }
    });
    root.add_controller(click);

    root.upcast()
}
