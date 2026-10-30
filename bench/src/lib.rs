//! Zenith benchmark harness: the compositor-free, testable parts.
//!
//! The binary (`zenith-bench`) wires these together around a headless Sway
//! session; everything here is pure and works on text or numbers.

pub mod measure;
pub mod procfs;
