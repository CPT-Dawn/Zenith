use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar};
use std::cell::Cell;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

const METADATA_FORMAT: &str = "{{status}}\t{{artist}}\t{{title}}\t{{position}}\t{{mpris:length}}";

#[derive(Debug, Clone)]
struct PlayerSnapshot {
    status: String,
    artist: String,
    title: String,
    position_us: u64,
    length_us: u64,
}

/// Create a playerctl-powered now-playing module with a progress bar.
pub fn create() -> GtkBox {
    let container = GtkBox::new(Orientation::Horizontal, 0);
    container.set_halign(Align::End);

    let button = Button::new();
    button.add_css_class("zenith-player-btn");
    button.add_css_class("zenith-module");
    button.add_css_class("zenith-module-surface");
    button.add_css_class("zenith-module-right");

    let content = GtkBox::new(Orientation::Vertical, 2);
    content.set_width_request(220);
    content.set_hexpand(false);
    content.set_valign(Align::End);

    let title = Label::new(Some("󰝛 No media"));
    title.add_css_class("zenith-player-title");
    title.set_halign(Align::Start);
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_single_line_mode(true);
    title.set_max_width_chars(32);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    content.append(&title);

    let progress = ProgressBar::new();
    progress.add_css_class("zenith-player-progress");
    progress.set_show_text(false);
    progress.set_hexpand(true);
    progress.set_fraction(0.0);
    content.append(&progress);

    button.set_child(Some(&content));
    container.append(&button);

    // Animate progress smoothly between `playerctl` metadata refreshes.
    // We only query `playerctl` once per second, but we interpolate the bar
    // at a higher frame rate for a more polished feel.
    let current_fraction = Rc::new(Cell::new(0.0));
    let target_fraction = Rc::new(Cell::new(0.0));

    refresh_widgets(&title, &button, target_fraction.as_ref());
    let init = target_fraction.get();
    current_fraction.set(init);
    progress.set_fraction(init);

    button.connect_clicked({
        let title = title.downgrade();
        let button = button.downgrade();
        let target_fraction = target_fraction.clone();
        move |_| {
            if let Err(err) = Command::new("playerctl").arg("play-pause").output() {
                log::debug!("playerctl play-pause failed: {err}");
            }

            if let (Some(t), Some(b)) = (title.upgrade(), button.upgrade())
            {
                refresh_widgets(&t, &b, target_fraction.as_ref());
            }
        }
    });

    let title_weak = title.downgrade();
    let button_weak = button.downgrade();
    let target_fraction = target_fraction.clone();
    let current_fraction_anim = current_fraction.clone();
    let target_fraction_anim = target_fraction.clone();
    let progress_weak_anim = progress.downgrade();

    // Frame-rate animation loop for the progress bar.
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(p) = progress_weak_anim.upgrade() else {
            return glib::ControlFlow::Break;
        };

        let current = current_fraction_anim.get();
        let target = target_fraction_anim.get();
        let delta = target - current;

        // Exponential smoothing: fast response, but it never "jumps".
        let next = if delta.abs() < 0.0004 {
            target
        } else {
            current + delta * 0.25
        };

        current_fraction_anim.set(next);
        p.set_fraction(next);
        glib::ControlFlow::Continue
    });

    glib::timeout_add_local(Duration::from_secs(1), move || {
        let Some(t) = title_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let Some(b) = button_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };

        refresh_widgets(&t, &b, target_fraction.as_ref());
        glib::ControlFlow::Continue
    });

    container
}

fn refresh_widgets(title: &Label, button: &Button, target_fraction: &Cell<f64>) {
    if let Some(snapshot) = read_player_snapshot() {
        let icon = match snapshot.status.as_str() {
            "Playing" => "",
            "Paused" => "",
            "Stopped" => "",
            _ => "󰎈",
        };

        let display_title = if snapshot.title.trim().is_empty() {
            "Unknown title".to_string()
        } else {
            snapshot.title.trim().to_string()
        };
        let display_artist = snapshot.artist.trim();

        let label_text = if display_artist.is_empty() {
            format!("{icon} {display_title}")
        } else {
            format!("{icon} {display_artist} - {display_title}")
        };
        title.set_label(&label_text);

        let elapsed = format_microseconds(snapshot.position_us);
        let total = format_microseconds(snapshot.length_us);
        let tooltip = if display_artist.is_empty() {
            format!("{display_title}\n{elapsed} / {total}\nLeft click: play/pause")
        } else {
            format!(
                "{display_artist} - {display_title}\n{elapsed} / {total}\nLeft click: play/pause"
            )
        };
        button.set_tooltip_text(Some(&tooltip));

        target_fraction.set(progress_fraction(
            snapshot.position_us,
            snapshot.length_us,
        ));
    } else {
        title.set_label("󰝛 No media");
        target_fraction.set(0.0);
        button.set_tooltip_text(Some("No active player found\nLeft click: play/pause"));
    }
}

fn read_player_snapshot() -> Option<PlayerSnapshot> {
    let output = Command::new("playerctl")
        .args(["metadata", "--format", METADATA_FORMAT])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let line = String::from_utf8(output.stdout).ok()?;
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let mut parts = line.splitn(5, '\t');
    let status = parts.next().unwrap_or_default().trim().to_string();
    let artist = parts.next().unwrap_or_default().trim().to_string();
    let title = parts.next().unwrap_or_default().trim().to_string();
    let position_us = parse_microseconds(parts.next().unwrap_or_default());
    let length_us = parse_microseconds(parts.next().unwrap_or_default());

    Some(PlayerSnapshot {
        status,
        artist,
        title,
        position_us,
        length_us,
    })
}

fn parse_microseconds(input: &str) -> u64 {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return 0;
    }

    if let Ok(value) = trimmed.parse::<u64>() {
        return value;
    }

    // Fallback: handle values like "12345.0" without lossy float casting.
    if let Some((whole, _fraction)) = trimmed.split_once('.') {
        return whole.trim().parse::<u64>().unwrap_or(0);
    }

    0
}

fn progress_fraction(position_us: u64, length_us: u64) -> f64 {
    if length_us == 0 {
        return 0.0;
    }

    // Compute the ratio in milliseconds to avoid lossy wide-int -> float casts.
    let current_ms_u64 = position_us / 1_000;
    let total_ms_u64 = length_us / 1_000;
    if total_ms_u64 == 0 {
        return 0.0;
    }

    let current_ms_u64 = current_ms_u64.min(total_ms_u64);
    let current_ms = u32::try_from(current_ms_u64).unwrap_or(u32::MAX);
    let total_ms = u32::try_from(total_ms_u64).unwrap_or(u32::MAX);

    (f64::from(current_ms) / f64::from(total_ms)).clamp(0.0, 1.0)
}

fn format_microseconds(microseconds: u64) -> String {
    let seconds = microseconds / 1_000_000;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let rem_seconds = seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{rem_seconds:02}")
    } else {
        format!("{minutes:02}:{rem_seconds:02}")
    }
}
