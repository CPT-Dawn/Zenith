use anyhow::{Context, Result};
use gdk4::Display;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CenterBox, CssProvider};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::config::ZenithConfig;
use crate::modules;
use crate::style;

/// Build and present the bar window for the given GTK `Application`.
pub fn build_bar(app: &Application, cfg: &ZenithConfig) -> Result<()> {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Zenith")
        .default_height(cfg.bar.height)
        .build();

    // ── Layer-shell setup ────────────────────────────────────────────
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some("zenith"));

    // Anchor to top, left, and right so the bar stretches across the monitor.
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, false);

    // Margins (gaps).
    window.set_margin(Edge::Top, cfg.bar.gap_top);
    window.set_margin(Edge::Left, cfg.bar.gap_horizontal);
    window.set_margin(Edge::Right, cfg.bar.gap_horizontal);

    // Exclusive zone: reserve space so tiled windows don't overlap.
    window.auto_exclusive_zone_enable();

    // ── Target a specific monitor if configured ──────────────────────
    if let Some(ref connector) = cfg.bar.monitor {
        if let Some(monitor) = find_monitor_by_connector(connector) {
            window.set_monitor(Some(&monitor));
        } else {
            log::warn!(
                "Monitor '{}' not found – falling back to default",
                connector
            );
        }
    }

    // ── CSS ──────────────────────────────────────────────────────────
    load_css(&cfg.bar)?;

    // ── Widget tree ──────────────────────────────────────────────────
    let outer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    outer.add_css_class("zenith-border");
    outer.set_hexpand(true);

    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    inner.add_css_class("zenith-inner");
    inner.set_hexpand(true);

    let center_box = CenterBox::new();
    center_box.set_hexpand(true);

    // Center: Date │  │ Time
    if cfg.modules.clock {
        let time_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
        time_container.set_halign(gtk4::Align::Center);

        // Date (clickable → calendar popover)
        let calendar = modules::calendar::create();
        time_container.append(&calendar);

        // Arch logo separator
        let logo = gtk4::Label::new(Some("\u{f303}")); // Nerd Font: 
        logo.add_css_class("zenith-logo");
        time_container.append(&logo);

        // Clock (ticking time)
        let clock = modules::clock::create(&cfg.modules.clock_format);
        time_container.append(&clock);

        center_box.set_center_widget(Some(&time_container));
    }

    // Right: System stats / tray placeholder
    if cfg.modules.system_stats {
        let sys = modules::system::create();
        center_box.set_end_widget(Some(&sys));
    }

    inner.append(&center_box);
    outer.append(&inner);
    window.set_child(Some(&outer));
    window.present();

    Ok(())
}

/// Load the generated CSS into the default GTK display.
fn load_css(bar: &crate::config::BarConfig) -> Result<()> {
    let css_text = style::build_css(bar);
    let provider = CssProvider::new();
    provider.load_from_string(&css_text);

    let display = Display::default().context("Could not get default GDK display")?;

    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    Ok(())
}

/// Walk through connected GDK monitors and return the first whose connector
/// string matches `name` (e.g. `"DP-1"`, `"eDP-1"`, `"HDMI-A-1"`).
fn find_monitor_by_connector(name: &str) -> Option<gdk4::Monitor> {
    let display = Display::default()?;
    let monitors = display.monitors();

    for i in 0..monitors.n_items() {
        if let Some(obj) = monitors.item(i) {
            if let Ok(mon) = obj.downcast::<gdk4::Monitor>() {
                if mon.connector().as_deref() == Some(name) {
                    return Some(mon);
                }
            }
        }
    }

    None
}
