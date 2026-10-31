use std::rc::Rc;

use gtk4::prelude::*;

use crate::modules::{Shared, ViewState};
use crate::ui::binding::bind;

/// The date label; a primary click opens a month popover with today highlighted.
pub fn build(state: &Rc<Shared<ViewState>>) -> gtk4::Widget {
    let label = gtk4::Label::new(None);
    bind(&label, state, |label, state| label.set_label(&state.label));

    let popover = gtk4::Popover::builder()
        .has_arrow(false)
        .css_classes(["zb-popover", "zb-calendar-popover"])
        .build();
    popover.set_parent(&label);
    // The month grid exists only while the popover is open, built fresh so
    // GtkCalendar's notion of "today" is never a stale day.
    popover.connect_closed(|popover| popover.set_child(gtk4::Widget::NONE));

    let click = gtk4::GestureClick::builder()
        .button(gtk4::gdk::BUTTON_PRIMARY)
        .build();
    let weak = popover.downgrade();
    click.connect_released(move |_, _, _, _| {
        let Some(popover) = weak.upgrade() else {
            return;
        };
        popover.set_child(Some(&gtk4::Calendar::new()));
        popover.popup();
    });
    label.add_controller(click);

    label.connect_destroy(move |_| popover.unparent());
    label.upcast()
}
