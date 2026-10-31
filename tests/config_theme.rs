use pretty_assertions::assert_eq;
use zenith_bar::config::{
    Css, Diagnostic, ResolvedConfig, StyleLayer, ThemeName, resolve, stylesheets,
};
use zenith_bar::modules::ModuleId::{self, *};

fn ok(text: &str) -> ResolvedConfig {
    match resolve(text) {
        Ok(config) => config,
        Err(diagnostics) => panic!("expected a valid config, got {diagnostics:#?}"),
    }
}

fn errors(text: &str) -> Vec<String> {
    match resolve(text) {
        Ok(_) => panic!("expected diagnostics"),
        Err(diagnostics) => diagnostics.iter().map(Diagnostic::to_string).collect(),
    }
}

#[test]
fn without_theme_overrides_the_block_carries_only_bar_geometry() {
    assert_eq!(
        ok("").css_overrides,
        ":root {\n  --zb-bar-height: 34px;\n  --zb-bar-radius: 14px;\n}\n"
    );
}

#[test]
fn theme_overrides_become_root_variables() {
    let text = "[theme]\naccent = \"#ff79c6\"\nsurface_opacity = 0.5\nmodule_radius = 6\n";

    assert_eq!(
        ok(text).css_overrides,
        ":root {\n  --zb-bar-height: 34px;\n  --zb-bar-radius: 14px;\n  \
         --zb-accent: #ff79c6;\n  --zb-surface-opacity: 0.5;\n  --zb-module-radius: 6px;\n}\n"
    );
}

#[test]
fn only_the_overrides_that_are_set_are_written() {
    let css = ok("[theme]\nsurface_opacity = 1\n").css_overrides;

    assert!(css.contains("--zb-surface-opacity: 1;"));
    assert!(!css.contains("--zb-accent"));
    assert!(!css.contains("--zb-module-radius"));
}

#[test]
fn the_default_theme_is_midnight_bloom() {
    assert_eq!(ok("").theme.name, ThemeName::MidnightBloom);
    assert_eq!(
        ok("[theme]\nname = \"midnight-bloom\"\n").theme.name,
        ThemeName::MidnightBloom
    );
}

#[test]
fn an_unknown_theme_name_lists_the_choices() {
    let errors = errors("[theme]\nname = \"dracula\"\n");

    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].starts_with("2:8: unknown variant `dracula`, expected"),
        "{errors:?}"
    );
    assert!(errors[0].contains("`midnight-bloom`"), "{errors:?}");
}

#[test]
fn accent_accepts_hex_functional_and_named_colours() {
    for colour in [
        "#8de3dc",
        "#fff",
        "#8de3dc80",
        "rgb(255 121 198)",
        "rgba(1, 2, 3, 0.5)",
        "hotpink",
    ] {
        let config = ok(&format!("[theme]\naccent = \"{colour}\"\n"));
        assert!(
            config
                .css_overrides
                .contains(&format!("--zb-accent: {colour};")),
            "{colour}"
        );
    }
}

#[test]
fn an_accent_that_is_not_a_colour_is_rejected_at_the_value() {
    for colour in [
        "",
        "#12",
        "#ggg",
        "red; } * { color: red",
        "url(x)",
        "rgb(1, 2",
    ] {
        assert_eq!(
            errors(&format!("[theme]\naccent = \"{colour}\"\n")),
            vec![format!(
                "2:10: `{colour}` is not a colour (expected e.g. `#8de3dc`, `rgb(141 227 220)` or a colour name)"
            )],
            "{colour}"
        );
    }
}

#[test]
fn surface_opacity_must_be_between_zero_and_one() {
    assert_eq!(
        errors("[theme]\nsurface_opacity = 1.5\n"),
        vec!["2:19: surface_opacity must be between 0 and 1"]
    );
    assert_eq!(
        errors("[theme]\nsurface_opacity = -0.1\n"),
        vec!["2:19: surface_opacity must be between 0 and 1"]
    );
    assert!(
        ok("[theme]\nsurface_opacity = 0\n")
            .css_overrides
            .contains("--zb-surface-opacity: 0;")
    );
}

#[test]
fn stylesheets_layer_theme_then_base_then_overrides_then_motion_then_user() {
    let config = ok("[theme]\naccent = \"#ff79c6\"\n");

    assert_eq!(
        stylesheets(&config, false),
        [
            (
                StyleLayer::Theme,
                Css::Resource("themes/midnight-bloom.css")
            ),
            (StyleLayer::Base, Css::Resource("css/base.css")),
            (
                StyleLayer::Overrides,
                Css::Text(config.css_overrides.clone())
            ),
            (StyleLayer::Motion, Css::Empty),
            (StyleLayer::User, Css::UserFile),
        ]
    );
}

#[test]
fn the_motion_layer_follows_motion_and_reduced_motion() {
    let motion = |text: &str, reduced: bool| stylesheets(&ok(text), reduced)[3].1.clone();

    assert_eq!(motion("", false), Css::Empty);
    assert_eq!(
        motion("[bar]\nmotion = \"subtle\"\n", true),
        Css::Resource("css/motion-off.css")
    );
    assert_eq!(
        motion("[bar]\nmotion = \"off\"\n", false),
        Css::Resource("css/motion-off.css")
    );
    assert_eq!(
        motion("[bar]\nmotion = \"ambient\"\n", false),
        Css::Resource("css/ambient.css")
    );
    assert_eq!(
        motion("[bar]\nmotion = \"ambient\"\n", true),
        Css::Resource("css/motion-off.css")
    );
}

#[test]
fn the_focal_class_lands_on_the_default_focal_module_only() {
    let config = ok("");

    assert_eq!(
        config.module_classes(Clock),
        ["zb-module", "zb-clock", "zb-focal"]
    );
    assert_eq!(config.module_classes(Logo), ["zb-module", "zb-logo"]);
}

#[test]
fn changing_focal_moves_the_glow_to_the_named_module() {
    let config = ok("[bar]\nfocal = \"logo\"\n");

    assert_eq!(
        config.module_classes(Logo),
        ["zb-module", "zb-logo", "zb-focal"]
    );
    assert_eq!(config.module_classes(Clock), ["zb-module", "zb-clock"]);
}

#[test]
fn with_focal_none_no_module_glows() {
    let config = ok("[bar]\nfocal = \"none\"\n");

    for id in ModuleId::ALL {
        assert!(!config.module_classes(id).contains(&"zb-focal".to_string()));
    }
}

#[test]
fn unknown_theme_keys_get_suggestions() {
    assert_eq!(
        errors("[theme]\naccnet = \"#fff\"\n"),
        vec!["2:1: unknown key `accnet` in [theme] (did you mean `accent`?)"]
    );
}

#[test]
fn every_built_in_theme_is_picked_by_name_and_loads_its_own_sheet() {
    for (name, theme, sheet) in [
        (
            "midnight-bloom",
            ThemeName::MidnightBloom,
            "themes/midnight-bloom.css",
        ),
        (
            "tokyo-night",
            ThemeName::TokyoNight,
            "themes/tokyo-night.css",
        ),
        (
            "catppuccin-mocha",
            ThemeName::CatppuccinMocha,
            "themes/catppuccin-mocha.css",
        ),
        (
            "gruvbox-dark",
            ThemeName::GruvboxDark,
            "themes/gruvbox-dark.css",
        ),
    ] {
        let config = ok(&format!("[theme]\nname = \"{name}\"\n"));

        assert_eq!(config.theme.name, theme);
        assert_eq!(
            stylesheets(&config, false)[0],
            (StyleLayer::Theme, Css::Resource(sheet))
        );
    }
}
