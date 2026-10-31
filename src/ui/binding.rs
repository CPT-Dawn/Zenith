//! Glue between a [`Shared`] view state and a widget. Every module view uses this.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::modules::{Shared, ViewState};

/// Call `apply` with every state the source publishes, for as long as `widget` lives.
/// Visibility, tooltip and state classes are handled here; `apply` does the rest.
pub fn bind<W: IsA<gtk4::Widget>>(
    widget: &W,
    state: &Rc<Shared<ViewState>>,
    apply: impl Fn(&W, &ViewState) + 'static,
) {
    let weak = widget.downgrade();
    let classes: RefCell<Vec<&'static str>> = RefCell::default();
    let subscription = state.subscribe(move |state| {
        let Some(widget) = weak.upgrade() else { return };
        let w = widget.upcast_ref::<gtk4::Widget>();
        w.set_visible(state.visible);
        w.set_tooltip_text(state.tooltip.as_deref());
        let mut old = classes.borrow_mut();
        for class in old.iter().filter(|c| !state.classes.contains(c)) {
            w.remove_css_class(class);
        }
        for class in &state.classes {
            w.add_css_class(class);
        }
        *old = state.classes.clone();
        apply(&widget, state);
    });

    let state = state.clone();
    widget.connect_destroy(move |_| state.unsubscribe(subscription));
}
