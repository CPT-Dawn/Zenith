//! Stylesheet layers: one CSS provider per [`StyleLayer`], loaded from
//! [`config::stylesheets`]. Which sheets load is decided there; this file only applies it.
//!
//! The motion layer follows the system's reduced-motion preference live.
//! Hot reload calls [`Style::apply`] with the new config and [`Style::reload_user`]
//! when `style.css` changes; only layers whose source changed are reloaded.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use gtk4::glib::SignalHandlerId;
use gtk4::prelude::*;
use gtk4::{CssProvider, ReducedMotion, gdk};

use crate::RESOURCE_PREFIX;
use crate::config::{self, Css, ResolvedConfig, StyleLayer};

const LAYERS: usize = 5;

/// Keeps the providers alive and swaps individual layers on change.
pub struct Style {
    inner: Rc<Inner>,
    settings: gtk4::Settings,
    reduced_motion_handler: Option<SignalHandlerId>,
}

struct Inner {
    providers: [CssProvider; LAYERS],
    loaded: RefCell<[Option<Css>; LAYERS]>,
    config: RefCell<ResolvedConfig>,
    user_style: PathBuf,
    settings: gtk4::Settings,
}

impl Style {
    pub fn install(display: &gdk::Display, config: &ResolvedConfig, user_style: &Path) -> Self {
        gtk4::IconTheme::for_display(display)
            .add_resource_path(&format!("{RESOURCE_PREFIX}/icons"));

        let settings = gtk4::Settings::for_display(display);
        let inner = Rc::new(Inner {
            providers: [
                provider("theme"),
                provider("base.css"),
                provider("overrides"),
                provider("motion"),
                provider("style.css"),
            ],
            loaded: RefCell::new(Default::default()),
            config: RefCell::new(config.clone()),
            user_style: user_style.to_owned(),
            settings: settings.clone(),
        });
        inner.apply();
        let layers = config::stylesheets(config, false).map(|(layer, _)| layer);
        for (layer, provider) in layers.into_iter().zip(&inner.providers) {
            gtk4::style_context_add_provider_for_display(display, provider, priority(layer));
        }

        let weak: Weak<Inner> = Rc::downgrade(&inner);
        let handler = settings.connect_gtk_interface_reduced_motion_notify(move |_| {
            if let Some(inner) = weak.upgrade() {
                log::info!("reduced-motion preference changed");
                inner.apply();
            }
        });

        Self {
            inner,
            settings,
            reduced_motion_handler: Some(handler),
        }
    }

    /// Switch to a new config, reloading only the layers it changes.
    pub fn apply(&self, config: &ResolvedConfig) {
        *self.inner.config.borrow_mut() = config.clone();
        self.inner.apply();
    }

    /// Re-read the user's `style.css`.
    pub fn reload_user(&self) {
        self.inner.load(StyleLayer::User as usize, &Css::UserFile);
    }
}

impl Drop for Style {
    fn drop(&mut self) {
        if let Some(handler) = self.reduced_motion_handler.take() {
            self.settings.disconnect(handler);
        }
    }
}

impl Inner {
    fn apply(&self) {
        let reduced = self.settings.gtk_interface_reduced_motion() == ReducedMotion::Reduce;
        let sheets = config::stylesheets(&self.config.borrow(), reduced);
        for (index, (_, css)) in sheets.into_iter().enumerate() {
            if self.loaded.borrow()[index].as_ref() != Some(&css) {
                self.load(index, &css);
            }
        }
    }

    fn load(&self, index: usize, css: &Css) {
        let provider = &self.providers[index];
        match css {
            Css::Resource(path) => {
                provider.load_from_resource(&format!("{RESOURCE_PREFIX}/{path}"))
            }
            Css::Text(text) => provider.load_from_string(text),
            Css::UserFile if self.user_style.is_file() => provider.load_from_path(&self.user_style),
            Css::UserFile | Css::Empty => provider.load_from_string(""),
        }
        self.loaded.borrow_mut()[index] = Some(css.clone());
    }
}

/// Layers stack above GTK's own theme; the user's sheet sits at USER priority.
fn priority(layer: StyleLayer) -> u32 {
    match layer {
        StyleLayer::User => gtk4::STYLE_PROVIDER_PRIORITY_USER,
        layer => gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + layer as u32,
    }
}

fn provider(name: &'static str) -> CssProvider {
    let provider = CssProvider::new();
    provider.connect_parsing_error(move |_, section, error| {
        let start = section.start_location();
        log::warn!(
            "{name}:{}:{}: {error}",
            start.lines() + 1,
            start.line_chars() + 1
        );
    });
    provider
}
