//! Zenith: the lightest beautiful status bar for Wayland.
//!
//! Everything lives in the library so integration tests in `tests/` use the
//! same public API as the binary. See `docs/adr/` for the ground rules.

pub mod cli;
pub mod commands;
pub mod config;
pub mod modules;
pub mod paths;
pub mod ui;

/// GApplication id; also the single-instance D-Bus name.
pub const APP_ID: &str = "dev.cptdawn.ZenithBar";
/// GResource prefix for everything bundled into the binary.
pub const RESOURCE_PREFIX: &str = "/dev/cptdawn/ZenithBar";
