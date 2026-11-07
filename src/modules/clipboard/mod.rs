//! Clipboard: Hoarder's recent Clips on the bar, over its socket API. No polling.

pub mod model;

pub use model::{Backoff, Clipboard, Kind, RECENT, Recent, Row};
