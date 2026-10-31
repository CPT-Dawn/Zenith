//! Pure logo model: the configured icon, or nothing.

use super::LogoConfig;
use crate::modules::ViewState;

pub fn view_state(config: &LogoConfig) -> ViewState {
    let icon = Some(config.icon.trim()).filter(|icon| !icon.is_empty());
    ViewState {
        visible: icon.is_some(),
        icon: icon.map(str::to_string),
        ..ViewState::default()
    }
}
