//! Bar popovers open off the bar's outer edge, not off the module inside it.
//!
//! A popover points at its parent widget by default: a module sits inset inside the
//! bar, so the card would land on the bar's own border. Instead it points at the
//! module's column of the whole bar, grown by a gap on both sides, and opens away
//! from the screen edge the bar is anchored to.

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, LayerShell};

/// The gap for a flush bar (`margin = 0`), so the card never touches it.
const MIN_GAP: i32 = 6;

/// Aim `popover` (parented to a module on a bar) at the bar's outer edge. Call it
/// right before `popup()`, once the bar is laid out.
pub fn place(popover: &gtk4::Popover) {
    let Some(anchor) = popover.parent() else {
        return;
    };
    let Some(window) = anchor.root().and_downcast::<gtk4::Window>() else {
        return;
    };
    let Some(bar) = window.child() else {
        return;
    };
    // The bar's border box, in the module's coordinates.
    let Some(bounds) = bar.compute_bounds(&anchor) else {
        return;
    };
    // The same breathing room the bar keeps from the screen edge.
    let gap = window.margin(Edge::Left).max(MIN_GAP);
    let top = bounds.y().floor() as i32;
    let bottom = (bounds.y() + bounds.height()).ceil() as i32;
    popover.set_pointing_to(Some(&gdk::Rectangle::new(
        0,
        top - gap,
        anchor.width().max(1),
        bottom - top + 2 * gap,
    )));
    let bottom_bar = window.is_anchor(Edge::Bottom) && !window.is_anchor(Edge::Top);
    popover.set_position(if bottom_bar {
        gtk4::PositionType::Top
    } else {
        gtk4::PositionType::Bottom
    });
}
