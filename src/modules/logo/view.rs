use std::rc::Rc;

use gtk4::prelude::*;

use crate::modules::{Shared, ViewState};
use crate::ui::binding::bind;

pub fn build(state: &Rc<Shared<ViewState>>) -> gtk4::Widget {
    let image = gtk4::Image::new();
    bind(&image, state, |image, state| {
        image.set_icon_name(state.icon.as_deref());
    });
    image.upcast()
}
