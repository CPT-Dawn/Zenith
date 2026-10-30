//! Seam 2 for presets: every built-in preset resolves cleanly, and each carries its look.

use pretty_assertions::assert_eq;
use zenith_bar::config::{Diagnostic, Motion, Preset, ResolvedConfig, ThemeName, resolve, starter};
use zenith_bar::modules::ModuleId::*;

fn ok(text: &str, preset: Option<Preset>) -> ResolvedConfig {
    match resolve(text, preset) {
        Ok(config) => config,
        Err(diagnostics) => panic!("expected a valid config, got {diagnostics:#?}"),
    }
}

fn errors(text: &str, preset: Preset) -> Vec<String> {
    match resolve(text, Some(preset)) {
        Ok(_) => panic!("expected diagnostics"),
        Err(diagnostics) => diagnostics.iter().map(Diagnostic::to_string).collect(),
    }
}

#[test]
fn every_built_in_preset_resolves_cleanly() {
    for preset in Preset::ALL {
        let written = ok(starter(Some(preset)), None);

        assert_eq!(written, ok("", Some(preset)), "{preset:?}");
    }
}

#[test]
fn bloom_is_floating_midnight_bloom_glass_with_a_glowing_clock() {
    let config = ok("", Some(Preset::Bloom));

    assert_eq!(config.theme.name, ThemeName::MidnightBloom);
    assert_eq!(config.bar.focal, Some(Clock));
    assert_eq!(config.bar.motion, Motion::Subtle);
    assert!(config.bar.margin > 0, "floating");
    assert!(config.bar.radius > 0, "floating");
}

#[test]
fn minimal_is_flush_without_glow_or_motion() {
    let config = ok("", Some(Preset::Minimal));

    assert_eq!(config.bar.margin, 0);
    assert_eq!(config.bar.radius, 0);
    assert_eq!(config.bar.focal, None);
    assert_eq!(config.bar.motion, Motion::Off);
}

#[test]
fn classic_is_the_v1_layout_in_tokyo_night() {
    let config = ok("", Some(Preset::Classic));

    assert_eq!(config.theme.name, ThemeName::TokyoNight);
    assert_eq!(config.layout.left, vec![Todo]);
    assert_eq!(config.layout.center, vec![Clock, Logo, Calendar]);
    assert_eq!(config.layout.right, vec![Media, System]);
}

#[test]
fn presets_are_named_in_kebab_case() {
    for (name, preset) in [
        ("bloom", Preset::Bloom),
        ("minimal", Preset::Minimal),
        ("classic", Preset::Classic),
    ] {
        assert_eq!(name.parse(), Ok(preset));
        assert_eq!(preset.as_str(), name);
    }
    assert_eq!("glassy".parse::<Preset>(), Err(()));
}

#[test]
fn the_users_keys_win_and_the_rest_of_the_preset_stays() {
    let config = ok("[theme]\nname = \"gruvbox-dark\"\n", Some(Preset::Minimal));

    assert_eq!(config.theme.name, ThemeName::GruvboxDark);
    assert_eq!(config.theme.surface_opacity, Some(1.0));
    assert_eq!(config.bar.motion, Motion::Off);
    assert_eq!(config.bar.margin, 0);
}

#[test]
fn the_users_layout_replaces_the_presets_layout_whole() {
    let config = ok("[layout]\nleft = [\"clock\"]\n", Some(Preset::Classic));

    assert_eq!(config.layout.left, vec![Clock]);
    assert_eq!(config.layout.center, vec![]);
    assert_eq!(config.layout.right, vec![]);
}

#[test]
fn the_presets_focal_steps_aside_when_the_users_layout_drops_it() {
    let config = ok("[layout]\nleft = [\"todo\"]\n", Some(Preset::Bloom));

    assert_eq!(config.bar.focal, None);
}

#[test]
fn diagnostics_point_into_the_users_text_not_the_preset() {
    assert_eq!(
        errors("\n[bar]\nheight = \"tall\"\n", Preset::Bloom),
        vec!["3:10: invalid type: string \"tall\", expected u32"]
    );
    assert_eq!(
        errors("[thme]\n", Preset::Classic),
        vec!["1:2: unknown table [thme] (did you mean `theme`?)"]
    );
}
