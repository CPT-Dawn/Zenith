use std::rc::Rc;

use gtk4::prelude::*;

use crate::modules::{Shared, ViewState};
use crate::ui::binding::bind;

pub fn build(state: &Rc<Shared<ViewState>>) -> gtk4::Widget {
    let label = gtk4::Label::new(None);
    bind(&label, state, |label, state| label.set_label(&state.label));
    label.upcast()
}
