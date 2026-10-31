//! Which GSK renderer the bar draws with. Cairo by default, because it is
//! the cheapest renderer that still draws every theme correctly; see
//! docs/adr/0004-cairo-renderer-by-default.md and docs/performance.md.

use std::ffi::OsStr;

/// The renderer Zenith asks GTK for when the user has not chosen one.
pub const DEFAULT: &str = "cairo";

/// The value to give `GSK_RENDERER`, or `None` when the user already set it
/// (their choice always wins: `GSK_RENDERER=ngl zenith-bar`).
pub fn default_for(current: Option<&OsStr>) -> Option<&'static str> {
    match current {
        Some(value) if !value.is_empty() => None,
        _ => Some(DEFAULT),
    }
}
