//! System: cpu, memory and temperature, read straight from `/proc` and `/sys` on one timer.

pub mod config;
pub mod model;
pub mod sensor;

pub use config::{Metric, SystemConfig};
