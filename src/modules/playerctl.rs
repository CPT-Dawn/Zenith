use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const METADATA_FORMAT: &str = "{{status}}\t{{artist}}\t{{title}}\t{{position}}\t{{mpris:length}}";
const POLL_INTERVAL_MS: u64 = 900;

#[derive(Debug, Clone, PartialEq)]
struct PlayerSnapshot {
    status: String,
    artist: String,
    title: String,
    position_us: u64,
    length_us: u64,
}

/// Create a playerctl-powered now-playing module with a progress bar.
///
/// Polling is performed on a **dedicated background thread** so that a slow or
/// hung `playerctl` process never blocks the GTK main loop.  The latest
/// snapshot is stored in a shared `Arc<Mutex<>>` which the UI timer reads
/// without blocking.
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

    // Shared latest snapshot: written by background thread, read by UI timer.
    // The mutex is held for nanoseconds (just a pointer swap), so there is
    // zero contention in practice.
    let latest: Arc<Mutex<Option<PlayerSnapshot>>> = Arc::new(Mutex::new(None));
    let last_applied: Arc<Mutex<Option<PlayerSnapshot>>> = Arc::new(Mutex::new(None));

    // Synchronous initial poll — one-time cost for an instant first frame.
    {
        let snap = read_player_snapshot();
        apply_snapshot(&title, &button, &progress, snap.as_ref());
        *latest.lock().unwrap_or_else(|e| e.into_inner()) = snap.clone();
        *last_applied.lock().unwrap_or_else(|e| e.into_inner()) = snap;
    }

    // ── Background polling thread ───────────────────────────────────────
    // The thread uses a simple `Arc<Mutex<bool>>` flag to know when to stop.
    let alive = Arc::new(Mutex::new(true));
    {
        let latest = Arc::clone(&latest);
        let alive = Arc::clone(&alive);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
                if !*alive.lock().unwrap_or_else(|e| e.into_inner()) {
                    break;
                }
                let snapshot = read_player_snapshot();
                *latest.lock().unwrap_or_else(|e| e.into_inner()) = snapshot;
            }
        });
    }

    // Click handler: fire-and-forget on a detached thread.
    button.connect_clicked({
        let latest = Arc::clone(&latest);
        move |_| {
            let latest = Arc::clone(&latest);
            std::thread::spawn(move || {
                if let Err(err) = Command::new("playerctl").arg("play-pause").output() {
                    log::debug!("playerctl play-pause failed: {err}");
                }
                // Brief pause for MPRIS state to propagate before re-polling.
                std::thread::sleep(Duration::from_millis(150));
                *latest.lock().unwrap_or_else(|e| e.into_inner()) = read_player_snapshot();
            });
        }
    });

    // ── UI update timer (reads latest snapshot, never blocks on I/O) ────
    {
        let title_weak = title.downgrade();
        let button_weak = button.downgrade();
        let progress_weak = progress.downgrade();
        let latest = Arc::clone(&latest);
        let last_applied = Arc::clone(&last_applied);
        let alive = Arc::clone(&alive);
        glib::timeout_add_local(Duration::from_millis(300), move || {
            let Some(t) = title_weak.upgrade() else {
                *alive.lock().unwrap_or_else(|e| e.into_inner()) = false;
                return glib::ControlFlow::Break;
            };
            let Some(b) = button_weak.upgrade() else {
                *alive.lock().unwrap_or_else(|e| e.into_inner()) = false;
                return glib::ControlFlow::Break;
            };
            let Some(p) = progress_weak.upgrade() else {
                *alive.lock().unwrap_or_else(|e| e.into_inner()) = false;
                return glib::ControlFlow::Break;
            };
            let snap = latest.lock().unwrap_or_else(|e| e.into_inner()).clone();

            // Avoid redundant label/progress writes when nothing changed.
            let mut prev = last_applied.lock().unwrap_or_else(|e| e.into_inner());
            if *prev != snap {
                apply_snapshot(&t, &b, &p, snap.as_ref());
                *prev = snap;
            }

            glib::ControlFlow::Continue
        });
    }

    container
}

/// Apply a pre-fetched player snapshot to the UI widgets.
///
/// This function performs **no I/O** — it only mutates GTK widget state and is
/// always called on the main thread.
fn apply_snapshot(
    title: &Label,
    _button: &Button,
    progress: &ProgressBar,
    snapshot: Option<&PlayerSnapshot>,
) {
    if let Some(snap) = snapshot {
        let icon = match snap.status.as_str() {
            "Playing" => "",
            "Paused" => "",
            "Stopped" => "",
            _ => "󰎈",
        };

        let display_title = if snap.title.trim().is_empty() {
            "Unknown title".to_string()
        } else {
            snap.title.trim().to_string()
        };
        let display_artist = snap.artist.trim();

        let label_text = if display_artist.is_empty() {
            format!("{icon} {display_title}")
        } else {
            format!("{icon} {display_artist} - {display_title}")
        };
        title.set_label(&label_text);

        progress.set_fraction(progress_fraction(snap.position_us, snap.length_us));
    } else {
        title.set_label("󰝛 No media");
        progress.set_fraction(0.0);
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

    // Compute the ratio in milliseconds to avoid lossy wide-int → float casts.
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

