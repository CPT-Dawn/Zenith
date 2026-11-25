//! The runners view: an icon and one dot per runner slot on the bar (filled while it
//! runs a job, hollow while idle, red when offline or missing, `+n` for queued jobs).
//! A primary click opens a card listing every slot with its job and how long it has run.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gio, glib};

use super::model::{self, Fleet, Health, SlotState};
use super::source::RunnersSource;
use crate::modules::Subscription;
use crate::ui::binding::bind;

/// More slots than this show as counts instead of dots.
const MAX_DOTS: usize = 16;

pub fn build(source: &Rc<RunnersSource>) -> gtk4::Widget {
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 7);
    let icon = gtk4::Image::from_icon_name("zb-runners-symbolic");
    icon.add_css_class("zb-runners-icon");
    root.append(&icon);
    root.append(&glance(source));

    bind(&root, &source.state(), |_, _| {});

    let popover = popover(source);
    popover.set_parent(&root);
    crate::ui::popover::open_on_click(&root, &popover, || {});

    root.connect_destroy(move |_| popover.unparent());
    root.upcast()
}

/// The fleet on the bar: slot dots (or counts) and the queued count, following the
/// fleet for as long as the box lives. The box hides itself when it has nothing to show.
pub fn glance(source: &Rc<RunnersSource>) -> gtk4::Box {
    let root = gtk4::Box::new(gtk4::Orientation::Horizontal, 7);
    root.add_css_class("zb-runners-glance");
    let dots = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    dots.add_css_class("zb-runners-dots");
    dots.set_valign(gtk4::Align::Center);
    let counts = gtk4::Label::new(None);
    counts.add_css_class("zb-runners-counts");
    let queued = gtk4::Label::new(None);
    queued.add_css_class("zb-runners-queued");
    root.append(&dots);
    root.append(&counts);
    root.append(&queued);

    let fleet = source.fleet();
    let weak = root.downgrade();
    let subscription = fleet.subscribe(move |fleet| {
        draw_bar(fleet, &dots, &counts, &queued);
        if let Some(root) = weak.upgrade() {
            root.set_visible(dots.is_visible() || counts.is_visible() || queued.is_visible());
        }
    });
    root.connect_destroy(move |_| fleet.unsubscribe(subscription));
    root
}

/// Whether a fleet has anything to show: a repo, a missing slot, or trouble.
/// With no repos configured it has none, and the cockpit leaves its runner card out.
pub fn shows(fleet: &Fleet) -> bool {
    !fleet.repos.is_empty()
        || fleet.missing > 0
        || !matches!(fleet.health, Health::Ok | Health::Waiting)
}

fn draw_bar(fleet: &Fleet, dots: &gtk4::Box, counts: &gtk4::Label, queued: &gtk4::Label) {
    while let Some(child) = dots.first_child() {
        dots.remove(&child);
    }
    let healthy = fleet.health == Health::Ok;
    let c = fleet.counts();
    let slots = c.running + c.idle + c.offline + c.missing;

    let as_dots = healthy && slots > 0 && slots <= MAX_DOTS;
    if as_dots {
        let states = fleet
            .repos
            .iter()
            .flat_map(|r| &r.slots)
            .map(|s| state_class(&s.state))
            .chain(std::iter::repeat_n("zb-missing", c.missing));
        for state in states {
            dots.append(&dot(state));
        }
    }
    dots.set_visible(as_dots);

    counts.set_visible(healthy && slots > MAX_DOTS);
    counts.set_label(&format!("{}/{}", c.running, slots));

    queued.set_visible(healthy && c.queued > 0);
    queued.set_label(&format!("+{}", c.queued));
}

fn dot(state: &'static str) -> gtk4::Widget {
    let dot = gtk4::Box::builder()
        .css_classes(["zb-runner-dot", state])
        .valign(gtk4::Align::Center)
        .build();
    dot.upcast()
}

fn state_class(state: &SlotState) -> &'static str {
    match state {
        SlotState::Running { .. } => "zb-running",
        SlotState::Idle => "zb-idle",
        SlotState::Offline => "zb-offline",
    }
}

/// Elapsed-time labels on the open card, with the start time each counts from.
pub type Clocks = Rc<RefCell<Vec<(gtk4::Label, jiff::Timestamp)>>>;

/// While open, the card follows the fleet; nothing of it exists while closed. Only the
/// elapsed times tick (once a second, in place), so hover and clicks are never reset.
fn popover(source: &Rc<RunnersSource>) -> gtk4::Popover {
    let popover = gtk4::Popover::builder()
        .has_arrow(false)
        .css_classes(["zb-popover", "zb-runners-popover"])
        // Set, so the card's expanding rows don't stretch the module on the bar.
        .hexpand(false)
        .vexpand(false)
        .build();

    let weak = Rc::downgrade(source);
    let open: Rc<Cell<Option<(glib::SourceId, Subscription)>>> = Rc::default();
    let clocks: Clocks = Rc::default();
    popover.connect_show({
        let open = open.clone();
        let clocks = clocks.clone();
        move |popover| {
            let Some(source) = weak.upgrade() else { return };
            let fill = {
                let popover = popover.downgrade();
                let source = Rc::downgrade(&source);
                let clocks = clocks.clone();
                move |fleet: &Fleet| {
                    if let (Some(popover), Some(source)) = (popover.upgrade(), source.upgrade()) {
                        clocks.borrow_mut().clear();
                        let card = card(fleet, &source, &popover, &clocks);
                        popover.set_child(Some(&card));
                    }
                }
            };
            let subscription = source.fleet().subscribe(fill);
            let clocks = clocks.clone();
            let ticker = glib::timeout_add_seconds_local(1, move || {
                let now = jiff::Timestamp::now();
                for (label, started) in clocks.borrow().iter() {
                    label.set_label(&model::elapsed(*started, now));
                }
                glib::ControlFlow::Continue
            });
            open.set(Some((ticker, subscription)));
        }
    });
    let weak = Rc::downgrade(source);
    popover.connect_closed(move |popover| {
        if let Some((ticker, subscription)) = open.take() {
            ticker.remove();
            if let Some(source) = weak.upgrade() {
                source.fleet().unsubscribe(subscription);
            }
        }
        clocks.borrow_mut().clear();
        popover.set_child(gtk4::Widget::NONE);
    });
    popover
}

/// The runner card: every slot with its job, per repo. `clocks` collects the elapsed
/// labels for the caller's once-a-second ticker.
pub fn card(
    fleet: &Fleet,
    source: &Rc<RunnersSource>,
    popover: &gtk4::Popover,
    clocks: &Clocks,
) -> gtk4::Widget {
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.add_css_class("zb-runners-card");
    content.set_width_request(340);

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    header.add_css_class("zb-runners-header");
    header.append(&label("Runners", &["zb-runners-title"], true));
    let summary = model::view_state(fleet).tooltip.unwrap_or_default();
    let summary = label(&summary, &["zb-runners-summary"], false);
    summary.set_xalign(1.0);
    summary.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    header.append(&summary);
    content.append(&header);

    if fleet.health != Health::Ok && fleet.repos.is_empty() {
        let note = label(
            &model::view_state(fleet).tooltip.unwrap_or_default(),
            &["zb-runners-note"],
            true,
        );
        note.set_wrap(true);
        content.append(&note);
        return content.upcast();
    }

    let now = jiff::Timestamp::now();
    for repo in &fleet.repos {
        let rows = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["zb-runners-list"])
            .build();
        for slot in &repo.slots {
            rows.append(&row(slot, now, popover, clocks));
        }

        let section = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        section.add_css_class("zb-runners-repo");
        section.append(&label(&repo.repo, &["zb-runners-repo-name"], true));
        let notes: Vec<String> = [(repo.queued, "queued"), (repo.stale, "stale")]
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| format!("{n} {what}"))
            .collect();
        let open = gtk4::Button::builder()
            .label(if notes.is_empty() {
                "Actions ↗".to_string()
            } else {
                format!("{}  ·  Actions ↗", notes.join(" · "))
            })
            .css_classes(["zb-runners-open", "flat"])
            .build();
        let url = source.actions_url(&repo.repo);
        let weak = popover.downgrade();
        open.connect_clicked(move |_| {
            if let Some(popover) = weak.upgrade() {
                popover.popdown();
            }
            launch(&url);
        });
        section.append(&open);

        content.append(&section);
        content.append(&rows);
    }

    if fleet.missing > 0 {
        let what = if fleet.missing == 1 { "slot" } else { "slots" };
        content.append(&label(
            &format!("{} expected {what} not registered", fleet.missing),
            &["zb-runners-note", "zb-missing"],
            true,
        ));
    }
    if fleet.health != Health::Ok {
        let note = label(
            &model::view_state(fleet).tooltip.unwrap_or_default(),
            &["zb-runners-note", "zb-error"],
            true,
        );
        note.set_wrap(true);
        content.append(&note);
    }
    content.upcast()
}

fn row(
    slot: &model::Slot,
    now: jiff::Timestamp,
    popover: &gtk4::Popover,
    clocks: &Clocks,
) -> gtk4::ListBoxRow {
    let line = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    line.append(&dot(state_class(&slot.state)));

    let names = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
    names.set_hexpand(true);
    names.append(&label(&slot.name, &["zb-runners-slot"], true));
    if !slot.labels.is_empty() {
        names.append(&label(
            &slot.labels.join(" · "),
            &["zb-runners-labels"],
            true,
        ));
    }
    line.append(&names);

    let (what, started, url) = match &slot.state {
        SlotState::Running { job: Some(job) } => (job.name.clone(), job.started, job.url.clone()),
        SlotState::Running { job: None } => ("starting…".into(), None, None),
        SlotState::Idle => ("idle".into(), None, None),
        SlotState::Offline => ("offline".into(), None, None),
    };
    let status = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
    status.set_valign(gtk4::Align::Center);
    let job = label(&what, &["zb-runners-job", state_class(&slot.state)], false);
    job.set_xalign(1.0);
    job.set_max_width_chars(22);
    job.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    status.append(&job);
    if let Some(started) = started {
        let elapsed = label(
            &model::elapsed(started, now),
            &["zb-runners-elapsed"],
            false,
        );
        elapsed.set_xalign(1.0);
        status.append(&elapsed);
        clocks.borrow_mut().push((elapsed, started));
    }
    line.append(&status);

    let row = gtk4::ListBoxRow::builder()
        .child(&line)
        .css_classes(["zb-runners-row"])
        .activatable(url.is_some())
        .build();
    if let Some(url) = url {
        row.set_tooltip_text(Some("Open the job"));
        let click = gtk4::GestureClick::new();
        let weak = popover.downgrade();
        click.connect_released(move |_, _, _, _| {
            if let Some(popover) = weak.upgrade() {
                popover.popdown();
            }
            launch(&url);
        });
        row.add_controller(click);
    }
    row
}

fn label(text: &str, classes: &[&str], left: bool) -> gtk4::Label {
    let label = gtk4::Label::builder()
        .label(text)
        .single_line_mode(true)
        .css_classes(classes.iter().map(|c| c.to_string()).collect::<Vec<_>>())
        .build();
    if left {
        label.set_xalign(0.0);
        label.set_hexpand(true);
    }
    label
}

fn launch(url: &str) {
    if let Err(e) = gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>) {
        log::warn!("runners: cannot open {url}: {e}");
    }
}
