//! The clipboard view: an icon on the bar; left click opens the recent Clips, right
//! click pauses or resumes recording. Both are bubble-phase gestures, so configured
//! `on_click` / `on_right_click` commands take precedence.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, glib};

use super::model::{Kind, Recent};
use super::source::ClipboardSource;
use crate::ui::binding::bind;

/// Thumbnails are at most 256 px; rows show them this big.
const THUMB_SIZE: i32 = 40;
/// How long after the popover closes a click on the module counts as the closing click.
const REOPEN_GUARD_US: i64 = 300_000;

pub fn build(source: &Rc<ClipboardSource>) -> gtk4::Widget {
    let icon = gtk4::Image::new();
    icon.add_css_class("zb-clipboard-icon");
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    root.append(&icon);
    bind(&root, &source.state(), move |_, state| {
        icon.set_icon_name(state.icon.as_deref());
    });

    let (popover, refresh) = popover(source);
    popover.set_parent(&root);
    let closed_at = Rc::new(Cell::new(0i64));
    popover.connect_closed({
        let closed_at = closed_at.clone();
        move |_| closed_at.set(glib::monotonic_time())
    });

    let click = gtk4::GestureClick::builder()
        .button(gdk::BUTTON_PRIMARY)
        .build();
    let weak = popover.downgrade();
    click.connect_released(move |_, _, _, _| {
        let Some(popover) = weak.upgrade() else {
            return;
        };
        // The press that closed it must not open it again.
        if glib::monotonic_time() - closed_at.get() > REOPEN_GUARD_US {
            refresh();
            crate::ui::popover::place(&popover);
            popover.popup();
        }
    });
    root.add_controller(click);

    let right_click = gtk4::GestureClick::builder()
        .button(gdk::BUTTON_SECONDARY)
        .build();
    let weak = Rc::downgrade(source);
    right_click.connect_released(move |_, _, _, _| {
        if let Some(source) = weak.upgrade() {
            source.toggle_pause();
        }
    });
    root.add_controller(right_click);

    root.connect_destroy(move |_| popover.unparent());
    root.upcast()
}

/// The recent Clips, and a `refresh` to call before it opens. Rows are built then, and rebuilt while it is open if
/// the list changes; thumbnails are fetched then and dropped when it closes.
fn popover(source: &Rc<ClipboardSource>) -> (gtk4::Popover, Rc<dyn Fn()>) {
    let popover = gtk4::Popover::builder()
        .has_arrow(false)
        .css_classes(["zb-popover", "zb-clipboard-popover"])
        .build();

    let rows = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .activate_on_single_click(true)
        .css_classes(["zb-clipboard-list"])
        .build();
    let empty = gtk4::Label::new(None);
    empty.add_css_class("zb-clipboard-empty");
    let search = gtk4::Button::builder()
        .label("Search all…")
        .css_classes(["zb-clipboard-search"])
        .build();

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    content.set_width_request(320);
    content.append(&empty);
    content.append(&rows);
    content.append(&search);
    popover.set_child(Some(&content));

    let thumbs: Rc<RefCell<HashMap<i64, gdk::Texture>>> = Rc::default();
    let fill = {
        let source = Rc::downgrade(source);
        let rows = rows.clone();
        let search = search.clone();
        let thumbs = thumbs.clone();
        move |recent: &Recent| {
            let Some(source) = source.upgrade() else {
                return;
            };
            empty.set_label(if recent.online {
                "Nothing copied yet"
            } else {
                "Hoarder isn't running"
            });
            empty.set_visible(recent.rows.is_empty());
            rows.set_visible(!recent.rows.is_empty());
            search.set_visible(recent.online);
            fill_rows(&rows, recent, &source, &thumbs);
        }
    };
    let fill = Rc::new(fill);

    let recent = source.recent();
    let subscription = recent.subscribe({
        let popover = popover.downgrade();
        let fill = fill.clone();
        move |recent| {
            if popover.upgrade().is_some_and(|p| p.is_visible()) {
                fill(recent);
            }
        }
    });
    // Closed, it holds no rows and no thumbnails.
    popover.connect_closed({
        let rows = rows.clone();
        move |_| {
            rows.remove_all();
            thumbs.borrow_mut().clear();
        }
    });
    popover.connect_destroy({
        let recent = recent.clone();
        move |_| recent.unsubscribe(subscription)
    });

    let weak = Rc::downgrade(source);
    let popover_weak = popover.downgrade();
    rows.connect_row_activated(move |_, row| {
        let id: i64 = row.widget_name().parse().unwrap_or_default();
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }
        if let Some(source) = weak.upgrade() {
            source.restore(id);
        }
    });

    let weak = Rc::downgrade(source);
    let popover_weak = popover.downgrade();
    search.connect_clicked(move |_| {
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }
        if let Some(source) = weak.upgrade() {
            crate::ui::actions::spawn(&source.search_command());
        }
    });

    let refresh: Rc<dyn Fn()> = Rc::new(move || recent.with(|recent| fill(recent)));
    (popover, refresh)
}

fn fill_rows(
    rows: &gtk4::ListBox,
    recent: &Recent,
    source: &Rc<ClipboardSource>,
    thumbs: &Rc<RefCell<HashMap<i64, gdk::Texture>>>,
) {
    rows.remove_all();
    thumbs
        .borrow_mut()
        .retain(|id, _| recent.rows.iter().any(|row| row.id == *id));
    for clip in &recent.rows {
        let line = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        if clip.kind == Kind::Image {
            line.append(&thumb(clip.id, source, thumbs));
        }
        let label = gtk4::Label::builder()
            .label(&clip.label)
            .xalign(0.0)
            .hexpand(true)
            .single_line_mode(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["zb-clipboard-preview"])
            .build();
        line.append(&label);
        let row = gtk4::ListBoxRow::builder()
            .child(&line)
            .css_classes(["zb-clipboard-row"])
            // The Clip's id, for row-activated.
            .name(clip.id.to_string())
            .build();
        rows.append(&row);
    }
}

/// An image Clip's thumbnail: from this opening's cache, or fetched now.
fn thumb(
    id: i64,
    source: &Rc<ClipboardSource>,
    thumbs: &Rc<RefCell<HashMap<i64, gdk::Texture>>>,
) -> gtk4::Image {
    let image = gtk4::Image::builder()
        .pixel_size(THUMB_SIZE)
        .css_classes(["zb-clipboard-thumb"])
        .build();
    if let Some(texture) = thumbs.borrow().get(&id) {
        image.set_paintable(Some(texture));
        return image;
    }
    let weak = image.downgrade();
    let cache = Rc::downgrade(thumbs);
    source.thumb(id, move |png| {
        let Some(texture) = png.and_then(|png| gdk::Texture::from_bytes(&png).ok()) else {
            return;
        };
        // A row that is gone (the popover closed) keeps nothing.
        let Some(image) = weak.upgrade() else { return };
        image.set_paintable(Some(&texture));
        if let Some(thumbs) = cache.upgrade() {
            thumbs.borrow_mut().insert(id, texture);
        }
    });
    image
}
