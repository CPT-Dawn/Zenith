//! Media: what's playing, straight from MPRIS over gio D-Bus. No `playerctl`, no polling.

pub mod config;
pub mod model;
pub mod source;
pub mod view;

pub use config::MediaConfig;
pub use model::{Media, PlaybackStatus, PropertyMap, Value};
