use anyhow::{Context, Result};
use gdk4::Display;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CenterBox, CssProvider};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

use crate::config::ZenithConfig;
use crate::modules;
use crate::style;

const RIGHT_CLUSTER_SPACING: i32 = 8;

/// Build and present the bar window for the given GTK `Application`.
pub fn build_bar(app: &Application, cfg: &ZenithConfig) -> Result<()> {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Zenith")
        .default_height(cfg.bar.height)
        .build();

    // Keep height config authoritative; default_height is only an initial size hint.
    window.set_resizable(false);
    // Width is handled below once we know the target monitor size.
    window.set_default_size(1, cfg.bar.height);
    window.set_size_request(-1, cfg.bar.height);
    window.set_height_request(cfg.bar.height);

    // ── Layer-shell setup ────────────────────────────────────────────
    window.init_layer_shell();
    window.set_layer(Layer::Top);
    window.set_namespace(Some("zenith"));
    window.set_keyboard_mode(KeyboardMode::OnDemand);

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
    let mut target_monitor: Option<gdk4::Monitor> = None;
    if let Some(ref connector) = cfg.bar.monitor {
        target_monitor = find_monitor_by_connector(connector);
        if target_monitor.is_none() {
            log::warn!("Monitor '{connector}' not found – falling back to default");
        }
    }

    if target_monitor.is_none() {
        // Fallback: pick the first monitor available.
        // (This avoids relying on APIs that differ between GTK/GDK versions.)
        let display = Display::default();
        if let Some(display) = display {
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(obj) = monitors.item(i) {
                    if let Ok(mon) = obj.downcast::<gdk4::Monitor>() {
                        target_monitor = Some(mon);
                        break;
                    }
                }
            }
        }
    }

    if let Some(ref monitor) = target_monitor {
        window.set_monitor(Some(monitor));

        // For layer-shell windows, explicit width helps ensure the bar
        // stretches across the whole monitor (anchors + margins).
        let geom = monitor.geometry();
        let gap_total = (cfg.bar.gap_horizontal as i64) * 2;
        let width = (geom.width() as i64 - gap_total).max(1) as i32;
        window.set_default_size(width, cfg.bar.height);
        window.set_size_request(width, cfg.bar.height);
    }

    // ── CSS ──────────────────────────────────────────────────────────
    load_css(&cfg.bar)?;

    // ── Widget tree ──────────────────────────────────────────────────
    let outer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    outer.add_css_class("zenith-border");
    outer.set_size_request(-1, cfg.bar.height);
    outer.set_height_request(cfg.bar.height);
    outer.set_vexpand(false);
    outer.set_hexpand(true);
    outer.set_halign(gtk4::Align::Fill);

    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    inner.add_css_class("zenith-inner");
    let inner_height = (cfg.bar.height - (cfg.bar.border_width * 2)).max(1);
    inner.set_size_request(-1, inner_height);
    inner.set_height_request(inner_height);
    inner.set_vexpand(false);
    inner.set_hexpand(true);
    inner.set_halign(gtk4::Align::Fill);

    let center_box = CenterBox::new();
    center_box.set_hexpand(true);
    center_box.set_halign(gtk4::Align::Fill);

    // Center: Date │  │ Time
    if cfg.modules.clock {
        let time_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        time_container.set_homogeneous(true);
        time_container.set_halign(gtk4::Align::Center);

        // Date (clickable → calendar popover)
        let calendar = modules::calendar::create();
        let date_slot = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        date_slot.set_halign(gtk4::Align::End);
        date_slot.set_hexpand(true);
        date_slot.append(&calendar);

        // Arch logo separator inside a transparent module shell
        let logo_module = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        logo_module.add_css_class("zenith-module-surface");
        logo_module.add_css_class("zenith-logo-module");
        logo_module.set_halign(gtk4::Align::Center);
        logo_module.set_valign(gtk4::Align::Center);

        let logo = gtk4::Label::new(Some("\u{f303}")); // Nerd Font:
        logo.add_css_class("zenith-logo");
        logo.set_halign(gtk4::Align::Center);
        logo.set_valign(gtk4::Align::Center);
        logo_module.append(&logo);

        // Clock (ticking time)
        let clock = modules::clock::create(&cfg.modules.clock_format);
        let clock_slot = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        clock_slot.set_halign(gtk4::Align::Start);
        clock_slot.set_hexpand(true);
        clock_slot.append(&clock);

        time_container.append(&date_slot);
        time_container.append(&logo_module);
        time_container.append(&clock_slot);

        center_box.set_center_widget(Some(&time_container));
    }

    // Left: Todo module
    if cfg.modules.todo {
        let todo = modules::todo::create();
        center_box.set_start_widget(Some(&todo));
    }

    // Right: Player + system stats
    if cfg.modules.system_stats || cfg.modules.playerctl {
        let end_container = gtk4::Box::new(gtk4::Orientation::Horizontal, RIGHT_CLUSTER_SPACING);
        end_container.set_halign(gtk4::Align::End);

        if cfg.modules.playerctl {
            let player = modules::playerctl::create();
            end_container.append(&player);
        }

        if cfg.modules.system_stats {
            let sys = modules::system::create();
            end_container.append(&sys);
        }

        center_box.set_end_widget(Some(&end_container));
    }

    inner.append(&center_box);
    outer.append(&inner);
    window.set_child(Some(&outer));
    window.present();

    Ok(())
}

/// Load the generated CSS into the default GTK display.
fn load_css(bar: &crate::config::BarConfig) -> Result<()> {
    let css_text = style::load(bar)?;
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
