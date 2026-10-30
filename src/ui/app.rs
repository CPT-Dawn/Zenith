//! The GTK application: single instance, config, sources, styles and bars.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};

use super::bar_manager::BarManager;
use super::sources::Sources;
use super::style::Style;
use crate::cli::Options;
use crate::config::{self, ResolvedConfig};
use crate::{APP_ID, paths};

/// Everything a running bar owns. Dropping it tears the bars down.
struct Runtime {
    _bars: BarManager,
    _style: Style,
    _sources: Sources,
    _hold: gio::ApplicationHoldGuard,
}

pub fn run(options: Options) -> glib::ExitCode {
    gio::resources_register_include!("zenith-bar.gresource").expect("bundled resources are valid");

    let config_path = paths::config_file(options.config.as_deref());
    let style_path = paths::style_file(options.style.as_deref());

    // Default flags make the application unique on the session bus: a second
    // launch forwards `activate` to the running instance and exits.
    let app = gtk4::Application::builder().application_id(APP_ID).build();
    let runtime: Rc<RefCell<Option<Runtime>>> = Rc::default();

    app.connect_activate(move |app| {
        if runtime.borrow().is_some() {
            log::info!("already running");
            return;
        }
        if !gtk4_layer_shell::is_supported() {
            log::error!("this compositor does not support the layer-shell protocol");
            app.quit();
            return;
        }
        let Some(display) = gdk::Display::default() else {
            log::error!("no display");
            app.quit();
            return;
        };

        let config = load_config(&config_path);
        let sources = Sources::start(&config);
        let style = Style::install(&display, &config, &style_path);
        let bars = BarManager::start(app, &display, &config, &sources);
        *runtime.borrow_mut() = Some(Runtime {
            _bars: bars,
            _style: style,
            _sources: sources,
            _hold: app.hold(),
        });
    });

    // Our own flags were parsed already; GTK sees only the program name.
    app.run_with_args(&["zenith-bar"])
}

/// A missing file means defaults. An invalid file is reported and replaced by defaults,
/// so a typo never leaves you without a bar.
fn load_config(path: &Path) -> ResolvedConfig {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            log::error!("{}: {e}; using defaults", path.display());
            String::new()
        }
    };
    config::resolve(&text, None).unwrap_or_else(|diagnostics| {
        let name = path.display().to_string();
        for diagnostic in diagnostics {
            log::error!("{}", diagnostic.render(&name));
        }
        log::error!("{name} has errors; using defaults");
        config::resolve("", None).expect("defaults are valid")
    })
}
