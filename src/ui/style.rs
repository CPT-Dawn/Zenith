//! Stylesheet layers, lowest priority first: theme tokens, component CSS,
//! config overrides, (motion sheet), the user's style.css.

use std::path::Path;

use gtk4::{CssProvider, gdk};

use crate::RESOURCE_PREFIX;
use crate::config::ResolvedConfig;

const THEME: u32 = gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION;
const BASE: u32 = THEME + 1;
const OVERRIDES: u32 = THEME + 2;
const USER: u32 = gtk4::STYLE_PROVIDER_PRIORITY_USER;

/// Keeps the providers alive and lets later reloads swap individual layers.
pub struct Style {
    pub theme: CssProvider,
    pub base: CssProvider,
    pub overrides: CssProvider,
    pub user: CssProvider,
}

impl Style {
    pub fn install(display: &gdk::Display, config: &ResolvedConfig, user_style: &Path) -> Self {
        gtk4::IconTheme::for_display(display)
            .add_resource_path(&format!("{RESOURCE_PREFIX}/icons"));

        let style = Self {
            theme: provider("theme"),
            base: provider("base"),
            overrides: provider("overrides"),
            user: provider("style.css"),
        };
        style
            .theme
            .load_from_resource(&format!("{RESOURCE_PREFIX}/themes/midnight-bloom.css"));
        style
            .base
            .load_from_resource(&format!("{RESOURCE_PREFIX}/css/base.css"));
        style.overrides.load_from_string(&config.css_overrides);
        if user_style.is_file() {
            style.user.load_from_path(user_style);
        }

        for (provider, priority) in [
            (&style.theme, THEME),
            (&style.base, BASE),
            (&style.overrides, OVERRIDES),
            (&style.user, USER),
        ] {
            gtk4::style_context_add_provider_for_display(display, provider, priority);
        }
        style
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
