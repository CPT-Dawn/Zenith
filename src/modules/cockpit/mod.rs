//! Cockpit: the runner fleet plus whatever user-configured feed commands describe.
//!
//! A **feed** is a shell command that prints one JSON document (contract v1, see
//! [`model::parse`] and docs/adr/0005-cockpit-feeds.md). Zenith knows nothing about
//! what a feed watches: it renders the feed's glance chips on the bar and its sections
//! as cards in the cockpit popover, next to the built-in runner card.

pub mod config;
pub mod model;
pub mod source;
pub mod view;

pub use config::{CockpitConfig, FeedConfig};
