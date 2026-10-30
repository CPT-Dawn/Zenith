//! Click actions on the GTK side: gestures on a module widget run the configured shell
//! commands, detached, and swallow the event so the module's built-in action never sees it.
//!
//! Generic over modules: the bar calls [`attach`] on every module widget. A module's
//! built-in actions are whatever its view wires up itself (a click gesture, a `MenuButton`
//! popover, …) in the normal bubble phase. Configured commands are handled in the capture
//! phase on the same widget, so they run first and claim the event; unconfigured gestures
//! are not intercepted and reach the built-in action untouched.

use std::ffi::OsStr;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use crate::config::{Action, ClickActions, Gesture};

/// Install gesture handlers on `widget` for every gesture that has a configured command.
pub fn attach(widget: &impl IsA<gtk4::Widget>, actions: &ClickActions) {
    for (gesture, button) in [
        (Gesture::Click, gdk::BUTTON_PRIMARY),
        (Gesture::RightClick, gdk::BUTTON_SECONDARY),
    ] {
        let Action::Run(command) = actions.action(gesture) else {
            continue;
        };
        let command = command.to_owned();
        let click = gtk4::GestureClick::builder()
            .button(button)
            .propagation_phase(gtk4::PropagationPhase::Capture)
            .build();
        click.connect_pressed(move |click, _, _, _| {
            click.set_state(gtk4::EventSequenceState::Claimed);
            spawn(&command);
        });
        widget.add_controller(click);
    }

    let up = owned(actions.action(Gesture::ScrollUp));
    let down = owned(actions.action(Gesture::ScrollDown));
    if up.is_none() && down.is_none() {
        return;
    }
    // DISCRETE turns smooth touchpad scrolling into steps, so one swipe is not a spawn storm.
    let scroll = gtk4::EventControllerScroll::new(
        gtk4::EventControllerScrollFlags::VERTICAL | gtk4::EventControllerScrollFlags::DISCRETE,
    );
    scroll.set_propagation_phase(gtk4::PropagationPhase::Capture);
    scroll.connect_scroll(move |_, _, dy| {
        let command = match dy {
            dy if dy < 0.0 => &up,
            dy if dy > 0.0 => &down,
            _ => &None,
        };
        match command {
            Some(command) => {
                spawn(command);
                glib::Propagation::Stop
            }
            None => glib::Propagation::Proceed,
        }
    });
    widget.add_controller(scroll);
}

fn owned(action: Action<'_>) -> Option<String> {
    match action {
        Action::Run(command) => Some(command.to_owned()),
        Action::BuiltIn => None,
    }
}

/// Run `command` with `/bin/sh -c`, detached: returns at once and never blocks the main loop.
/// The child is reaped asynchronously when it exits. An empty command does nothing.
pub fn spawn(command: &str) {
    if command.trim().is_empty() {
        return;
    }
    let argv = [OsStr::new("/bin/sh"), OsStr::new("-c"), OsStr::new(command)];
    match gio::Subprocess::newv(&argv, gio::SubprocessFlags::STDOUT_SILENCE) {
        Ok(child) => {
            let command = command.to_owned();
            child.wait_check_async(None::<&gio::Cancellable>, move |result| {
                if let Err(e) = result {
                    log::info!("click command `{command}` failed: {e}");
                }
            });
        }
        Err(e) => log::warn!("cannot run click command `{command}`: {e}"),
    }
}
