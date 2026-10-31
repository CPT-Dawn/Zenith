//! Clock: the current time, redrawn exactly on the minute (or second) boundary.

pub mod config;
pub mod model;
pub mod source;
pub mod view;

pub use config::ClockConfig;
