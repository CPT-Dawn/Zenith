//! Clipboard: Hoarder's recent Clips on the bar, over its socket API. No polling.

pub mod config;
pub mod model;

pub use config::ClipboardConfig;
pub use model::{Backoff, Clipboard, Kind, RECENT, Recent, Row};
