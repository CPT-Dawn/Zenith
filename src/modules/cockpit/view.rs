//! The cockpit view. On the bar: an icon, the runner fleet's dots and queued count,
//! then each feed's glance chips. A primary click opens the cockpit: one card per feed
//! section, in feed order, then the runner card. Countdowns, ages and job times tick
//! once a second only while it is open (ADR 0003).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use jiff::Timestamp;

use super::model::{self, Chip, FeedState, Row, Section};
use super::source::CockpitSource;
use crate::modules::Subscription;
use crate::modules::runners::view as runner_view;
use crate::ui::binding::bind;

/// The card's width, wider than the runner popover's.
const WIDTH: i32 = 400;
/// Kept free above and below the cockpit when it is taller than the output.
const SCREEN_ROOM: i32 = 120;

pub fn build(source: &Rc<CockpitSource>) -> gtk4::Widget {
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 7);
    let icon = gtk4::Image::from_icon_name("zb-cockpit-symbolic");
    icon.add_css_class("zb-cockpit-icon");
    root.append(&icon);
    root.append(&runner_view::glance(source.runners()));

    let chips = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
    chips.add_css_class("zb-cockpit-glance");
    chips.set_valign(gtk4::Align::Center);
    root.append(&chips);

    bind(&root, &source.state(), |_, _| {});
    let shared = source.chips();
    let subscription = shared.subscribe(move |list| draw_chips(list, &chips));
    root.connect_destroy(move |_| shared.unsubscribe(subscription));

    let popover = popover(source);
    popover.set_parent(&root);
    crate::ui::popover::open_on_click(&root, &popover, || {});
    root.connect_destroy(move |_| popover.unparent());
    root.upcast()
}

fn draw_chips(list: &[Chip], chips: &gtk4::Box) {
    while let Some(child) = chips.first_child() {
        chips.remove(&child);
    }
    for chip in list {
        let item = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        item.add_css_class("zb-cockpit-chip");
        item.add_css_class(chip.tone.css_class());
        if chip.failed {
            item.add_css_class("zb-cockpit-failed");
        }
        if !chip.label.is_empty() {
            item.append(&label(&chip.label, &["zb-cockpit-chip-label"], false));
        }
        item.append(&label(&chip.text, &["zb-cockpit-chip-text"], false));
        item.set_tooltip_text(chip.tooltip.as_deref());
        chips.append(&item);
    }
    chips.set_visible(!list.is_empty());
}

/// A label that re-renders from a time, once a second while the cockpit is open.
type Ticks = Rc<RefCell<Vec<(gtk4::Label, Timestamp, fn(Timestamp, Timestamp) -> String)>>>;

struct Open {
    ticker: glib::SourceId,
    feeds: Subscription,
    fleet: Subscription,
}

/// Nothing of the cockpit exists while it is closed. While open it follows the feeds
/// and the fleet, rebuilt on each change.
fn popover(source: &Rc<CockpitSource>) -> gtk4::Popover {
    let popover = gtk4::Popover::builder()
        .has_arrow(false)
        .css_classes(["zb-popover", "zb-cockpit-popover"])
        .hexpand(false)
        .vexpand(false)
        .build();

    let open: Rc<Cell<Option<Open>>> = Rc::default();
    let ticks: Ticks = Rc::default();
    let clocks: runner_view::Clocks = Rc::default();

    let weak = Rc::downgrade(source);
    popover.connect_show({
        let open = open.clone();
        let ticks = ticks.clone();
        let clocks = clocks.clone();
        move |popover| {
            let Some(source) = weak.upgrade() else { return };
            let fill = {
                let popover = popover.downgrade();
                let source = Rc::downgrade(&source);
                let ticks = ticks.clone();
                let clocks = clocks.clone();
                Rc::new(move || {
                    if let (Some(popover), Some(source)) = (popover.upgrade(), source.upgrade()) {
                        ticks.borrow_mut().clear();
                        clocks.borrow_mut().clear();
                        let view = cockpit(&source, &popover, &ticks, &clocks);
                        popover.set_child(Some(&view));
                    }
                })
            };
            let feeds = source.feeds().subscribe({
                let fill = fill.clone();
                move |_| fill()
            });
            let fleet = source.runners().fleet().subscribe(move |_| fill());
            let ticks = ticks.clone();
            let clocks = clocks.clone();
            let ticker = glib::timeout_add_seconds_local(1, move || {
                let now = Timestamp::now();
                for (label, at, format) in ticks.borrow().iter() {
                    label.set_label(&format(*at, now));
                }
                for (label, started) in clocks.borrow().iter() {
                    label.set_label(&crate::modules::runners::model::elapsed(*started, now));
                }
                glib::ControlFlow::Continue
            });
            open.set(Some(Open {
                ticker,
                feeds,
                fleet,
            }));
        }
    });
    let weak = Rc::downgrade(source);
    popover.connect_closed(move |popover| {
        if let Some(open) = open.take() {
            open.ticker.remove();
            if let Some(source) = weak.upgrade() {
                source.feeds().unsubscribe(open.feeds);
                source.runners().fleet().unsubscribe(open.fleet);
            }
        }
        ticks.borrow_mut().clear();
        clocks.borrow_mut().clear();
        popover.set_child(gtk4::Widget::NONE);
    });
    popover
}

fn cockpit(
    source: &Rc<CockpitSource>,
    popover: &gtk4::Popover,
    ticks: &Ticks,
    clocks: &runner_view::Clocks,
) -> gtk4::Widget {
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.add_css_class("zb-cockpit-view");
    content.set_width_request(WIDTH);

    let now = Timestamp::now();
    let feeds = source.feeds().with(Vec::clone);
    for feed in &feeds {
        feed_cards(feed, now, popover, ticks, &content);
    }
    let fleet = source.runners().fleet().with(Clone::clone);
    if runner_view::shows(&fleet) {
        let card = runner_view::card(&fleet, source.runners(), popover, clocks);
        card.add_css_class("zb-cockpit-card");
        content.append(&card);
    }
    if content.first_child().is_none() {
        let empty = label(
            "Nothing to show: add repos or feeds to [cockpit]",
            &["zb-cockpit-empty"],
            true,
        );
        empty.set_wrap(true);
        content.append(&empty);
    }

    let scroller = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .max_content_height(max_height(popover))
        .child(&content)
        .build();
    scroller.add_css_class("zb-cockpit-scroller");
    scroller.upcast()
}

/// The output's height less some room, so a tall cockpit scrolls instead of running off.
fn max_height(popover: &gtk4::Popover) -> i32 {
    let monitor = popover
        .parent()
        .and_then(|p| p.native())
        .and_then(|n| n.surface())
        .and_then(|s| gdk::Display::default()?.monitor_at_surface(&s));
    monitor.map_or(600, |m| (m.geometry().height() - SCREEN_ROOM).max(200))
}

/// A feed's sections as cards; a failed feed says so after them.
fn feed_cards(
    feed: &FeedState,
    now: Timestamp,
    popover: &gtk4::Popover,
    ticks: &Ticks,
    content: &gtk4::Box,
) {
    for section in feed.last.iter().flat_map(|f| &f.sections) {
        content.append(&section_card(section, now, popover, ticks));
    }
    if let Some(why) = &feed.failure {
        let text = if feed.last.is_some() {
            format!(
                "⚠ `{}` failed: {why} (showing its last answer)",
                feed.command
            )
        } else {
            format!("⚠ `{}` failed: {why}", feed.command)
        };
        let note = label(&text, &["zb-cockpit-feed-error"], true);
        note.set_wrap(true);
        content.append(&note);
    }
}

fn section_card(
    section: &Section,
    now: Timestamp,
    popover: &gtk4::Popover,
    ticks: &Ticks,
) -> gtk4::Box {
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    card.add_css_class("zb-cockpit-card");

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    header.add_css_class("zb-cockpit-header");
    header.append(&label(&section.title, &["zb-cockpit-title"], true));
    if let Some(since) = section.stale_since {
        let stale = label(&stale_text(since, now), &["zb-cockpit-stale"], false);
        ticks.borrow_mut().push((stale.clone(), since, stale_text));
        header.append(&stale);
    }
    if let Some(note) = &section.note {
        let note = label(note, &["zb-cockpit-note"], false);
        note.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        header.append(&note);
    }
    card.append(&header);

    if !section.rows.is_empty() {
        let grid = gtk4::Grid::builder()
            .column_spacing(10)
            .row_spacing(6)
            .css_classes(["zb-cockpit-rows"])
            .build();
        for (y, row) in section.rows.iter().enumerate() {
            let y = i32::try_from(y).unwrap_or(i32::MAX);
            match row {
                Row::Meter(meter) => meter_row(&grid, y, meter, now, ticks),
                Row::Text(text) => text_row(&grid, y, text, popover),
            }
        }
        card.append(&grid);
    }
    if let Some(error) = &section.error {
        let error = label(error, &["zb-cockpit-error"], true);
        error.set_wrap(true);
        card.append(&error);
    }
    card
}

fn stale_text(since: Timestamp, now: Timestamp) -> String {
    format!("⚠ {} old", model::age(since, now))
}

fn meter_row(grid: &gtk4::Grid, y: i32, meter: &model::Meter, now: Timestamp, ticks: &Ticks) {
    grid.attach(
        &label(&meter.label, &["zb-cockpit-label"], false),
        0,
        y,
        1,
        1,
    );
    let bar = gtk4::ProgressBar::builder()
        .fraction(meter.value)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .css_classes(["zb-cockpit-meter", meter.tone.css_class()])
        .build();
    grid.attach(&bar, 1, y, 1, 1);
    let text = label(
        &meter.text,
        &["zb-cockpit-value", meter.tone.css_class()],
        false,
    );
    text.set_xalign(1.0);
    grid.attach(&text, 2, y, 1, 1);
    if let Some(until) = meter.until {
        let left = label(
            &model::countdown(until, now),
            &["zb-cockpit-countdown"],
            false,
        );
        left.set_xalign(1.0);
        left.set_width_chars(6);
        ticks
            .borrow_mut()
            .push((left.clone(), until, model::countdown));
        grid.attach(&left, 3, y, 1, 1);
    }
}

fn text_row(grid: &gtk4::Grid, y: i32, row: &model::TextRow, popover: &gtk4::Popover) {
    let name = label(&row.label, &["zb-cockpit-label"], false);
    grid.attach(&name, 0, y, 1, 1);
    let tone = row.tone.unwrap_or(model::Tone::Muted);
    let text = label(&row.text, &["zb-cockpit-text", tone.css_class()], false);
    text.set_xalign(1.0);
    text.set_hexpand(true);
    text.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    grid.attach(&text, 1, y, 3, 1);
    if let Some(url) = row.url.clone() {
        for widget in [name.upcast_ref::<gtk4::Widget>(), text.upcast_ref()] {
            widget.add_css_class("zb-cockpit-link");
            widget.set_tooltip_text(Some(&url));
            widget.set_cursor_from_name(Some("pointer"));
            let click = gtk4::GestureClick::new();
            let weak = popover.downgrade();
            let url = url.clone();
            click.connect_released(move |_, _, _, _| {
                if let Some(popover) = weak.upgrade() {
                    popover.popdown();
                }
                launch(&url);
            });
            widget.add_controller(click);
        }
    }
}

fn label(text: &str, classes: &[&str], left: bool) -> gtk4::Label {
    let label = gtk4::Label::builder()
        .label(text)
        .single_line_mode(true)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build();
    label.set_xalign(0.0);
    if left {
        label.set_hexpand(true);
    }
    label
}

fn launch(url: &str) {
    if let Err(e) = gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>) {
        log::warn!("cockpit: cannot open {url}: {e}");
    }
}
