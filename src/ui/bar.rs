//! One bar: a layer-shell window on one output with three sections of module views.

use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use super::sources::Sources;
use crate::config::{Position, ResolvedConfig};
use crate::modules::calendar;
use crate::modules::{ModuleId, clock, logo};

pub fn build(
    app: &gtk4::Application,
    monitor: Option<&gdk::Monitor>,
    config: &ResolvedConfig,
    sources: &Sources,
) -> gtk4::ApplicationWindow {
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("zenith-bar")
        .css_classes(["zb-window"])
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some("zenith-bar"));
    window.set_keyboard_mode(KeyboardMode::None);
    if let Some(monitor) = monitor {
        window.set_monitor(Some(monitor));
    }

    let bar = &config.bar;
    let edge = match bar.position {
        Position::Top => Edge::Top,
        Position::Bottom => Edge::Bottom,
    };
    for side in [edge, Edge::Left, Edge::Right] {
        window.set_anchor(side, true);
        window.set_margin(side, bar.margin as i32);
    }
    window.auto_exclusive_zone_enable();

    let root = gtk4::CenterBox::builder().css_classes(["zb-bar"]).build();
    root.set_start_widget(Some(&section(
        "zb-left",
        &config.layout.left,
        config,
        sources,
    )));
    root.set_center_widget(Some(&section(
        "zb-center",
        &config.layout.center,
        config,
        sources,
    )));
    root.set_end_widget(Some(&section(
        "zb-right",
        &config.layout.right,
        config,
        sources,
    )));
    window.set_child(Some(&root));

    window
}

fn section(
    class: &str,
    modules: &[ModuleId],
    config: &ResolvedConfig,
    sources: &Sources,
) -> gtk4::Box {
    let section = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .css_classes(["zb-section", class])
        .build();
    for &id in modules {
        let Some(widget) = view(id, sources) else {
            log::debug!("module `{id}` has no view yet, skipping");
            continue;
        };
        for class in config.module_classes(id) {
            widget.add_css_class(&class);
        }
        widget.set_valign(gtk4::Align::Center);
        super::actions::attach(&widget, config.actions(id));
        section.append(&widget);
    }
    section
}

/// A fresh view of one module, bound to its shared source. One line per module.
fn view(id: ModuleId, sources: &Sources) -> Option<gtk4::Widget> {
    match id {
        ModuleId::Clock => sources
            .clock
            .as_ref()
            .map(|s| clock::view::build(&s.state())),
        ModuleId::Logo => sources.logo.as_ref().map(|s| logo::view::build(&s.state())),
        ModuleId::Calendar => sources
            .calendar
            .as_ref()
            .map(|s| calendar::view::build(&s.state())),
        _ => None,
    }
}
