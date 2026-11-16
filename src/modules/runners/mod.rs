//! Runners: the GitHub Actions self-hosted runner slots of some repos, polled over
//! GitHub's REST API with conditional requests.

pub mod config;
pub mod model;
pub mod source;
pub mod view;

pub use config::RunnersConfig;
