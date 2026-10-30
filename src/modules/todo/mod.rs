//! Todo: a compact count of open todos on the bar and a popover to manage them.

pub mod model;
pub mod source;
pub mod store;
pub mod view;

/// Where the todos live: `$XDG_DATA_HOME/zenith-bar/todos.json`.
pub fn data_file() -> std::path::PathBuf {
    crate::paths::data_dir().join("todos.json")
}
