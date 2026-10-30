use pretty_assertions::assert_eq;
use zenith_bar::modules::ViewState;
use zenith_bar::modules::logo::{LogoConfig, model};

fn logo(icon: &str) -> LogoConfig {
    LogoConfig { icon: icon.into() }
}

#[test]
fn shows_the_configured_icon_and_never_wakes() {
    assert_eq!(
        model::view_state(&logo("archlinux-logo")),
        ViewState {
            icon: Some("archlinux-logo".into()),
            visible: true,
            next_wake: None,
            ..ViewState::default()
        }
    );
}

#[test]
fn defaults_to_the_bundled_zenith_mark() {
    assert_eq!(
        model::view_state(&LogoConfig::default()).icon.as_deref(),
        Some("zb-logo-symbolic")
    );
}

#[test]
fn an_empty_icon_hides_the_logo() {
    let state = model::view_state(&logo(""));

    assert_eq!(state.icon, None);
    assert!(!state.visible);
}
