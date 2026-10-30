use std::rc::Rc;

use gtk4::prelude::*;

use crate::modules::{Shared, ViewState};
use crate::ui::binding::bind;

/// A state icon (only while warm or hot) next to the metrics label.
pub fn build(state: &Rc<Shared<ViewState>>) -> gtk4::Widget {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let icon = gtk4::Image::new();
    let label = gtk4::Label::new(None);
    row.append(&icon);
    row.append(&label);
    bind(&row, state, move |_, state| {
        icon.set_icon_name(state.icon.as_deref());
        icon.set_visible(state.icon.is_some());
        label.set_label(&state.label);
    });
    row.upcast()
}
