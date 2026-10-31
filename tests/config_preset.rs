//! Seam 2 for presets: every built-in preset resolves cleanly, and each carries its look.
//!
//! A preset is a complete config file (`init --preset` writes it verbatim), so these
//! tests resolve the preset's text exactly as the bar would after `init`.

use pretty_assertions::assert_eq;
use zenith_bar::config::{Motion, Preset, ResolvedConfig, ThemeName, resolve, starter};
use zenith_bar::modules::ModuleId::*;

fn ok(preset: Preset) -> ResolvedConfig {
    match resolve(starter(Some(preset))) {
        Ok(config) => config,
        Err(diagnostics) => panic!("expected {preset:?} to be valid, got {diagnostics:#?}"),
    }
}

#[test]
fn every_built_in_preset_resolves_cleanly() {
    for preset in Preset::ALL {
        ok(preset);
    }
}

#[test]
fn init_writes_the_presets_own_file() {
    for preset in Preset::ALL {
        assert_eq!(starter(Some(preset)), preset.text(), "{preset:?}");
    }
}

#[test]
fn bloom_is_floating_midnight_bloom_glass_with_a_glowing_clock() {
    let config = ok(Preset::Bloom);

    assert_eq!(config.theme.name, ThemeName::MidnightBloom);
    assert_eq!(config.bar.focal, Some(Clock));
    assert_eq!(config.bar.motion, Motion::Subtle);
    assert!(config.bar.margin > 0, "floating");
    assert!(config.bar.radius > 0, "floating");
}

#[test]
fn minimal_is_flush_without_glow_or_motion() {
    let config = ok(Preset::Minimal);

    assert_eq!(config.bar.margin, 0);
    assert_eq!(config.bar.radius, 0);
    assert_eq!(config.bar.focal, None);
    assert_eq!(config.bar.motion, Motion::Off);
}

#[test]
fn classic_is_the_v1_layout_in_tokyo_night() {
    let config = ok(Preset::Classic);

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
