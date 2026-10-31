use pretty_assertions::assert_eq;
use zenith_bar::config::{Diagnostic, Monitors, Motion, Position, resolve};
use zenith_bar::modules::ModuleId::{self, *};

fn ok(text: &str) -> zenith_bar::config::ResolvedConfig {
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
fn empty_config_resolves_to_the_bloom_defaults() {
    let config = ok("");

    assert_eq!(config.bar.position, Position::Top);
    assert_eq!(config.bar.monitors, Monitors::All);
    assert_eq!(config.bar.height, 34);
    assert_eq!(config.bar.margin, 8);
    assert_eq!(config.bar.radius, 14);
    assert_eq!(config.bar.motion, Motion::Subtle);
    assert_eq!(config.bar.focal, Some(Clock));
    assert_eq!(config.layout.left, vec![Logo, Todo]);
    assert_eq!(config.layout.center, vec![Clock, Calendar]);
    assert_eq!(config.layout.right, vec![Media, System]);
    assert_eq!(config.clock.format, "%H:%M");
    assert_eq!(config.logo.icon, "zb-logo-symbolic");
}

#[test]
fn unknown_key_is_reported_at_its_position_with_a_suggestion() {
    let text = "[bar]\nheight = 30\n  heigth = 32\n";

    assert_eq!(
        resolve(text),
        Err(vec![Diagnostic {
            line: 3,
            column: 3,
            message: "unknown key `heigth` in [bar]".into(),
            suggestion: Some("height".into()),
        }])
    );
}

#[test]
fn every_unknown_key_and_table_is_reported_not_just_the_first() {
    let text =
        "[bar]\nradus = 3\n\n[clok]\nformat = \"%H\"\n\n[logo]\nicon = \"x\"\ncolour = \"red\"\n";

    assert_eq!(
        errors(text),
        vec![
            "2:1: unknown key `radus` in [bar] (did you mean `radius`?)",
            "4:2: unknown table [clok] (did you mean `clock`?)",
            "9:1: unknown key `colour` in [logo]",
        ]
    );
}

#[test]
fn unknown_keys_are_found_in_inline_and_dotted_tables() {
    let text = "bar = { hieght = 30 }\nclock.fromat = \"%H\"\n";

    assert_eq!(
        errors(text),
        vec![
            "1:9: unknown key `hieght` in [bar] (did you mean `height`?)",
            "2:7: unknown key `fromat` in [clock] (did you mean `format`?)",
        ]
    );
}

#[test]
fn a_module_in_two_sections_is_an_error_at_the_second_occurrence() {
    let text = "[layout]\nleft = [\"clock\", \"logo\"]\nright = [\"logo\"]\n";

    assert_eq!(
        errors(text),
        vec!["3:10: module `logo` is already in [layout].left"]
    );
}

#[test]
fn a_module_twice_in_one_section_is_an_error() {
    let text = "[layout]\ncenter = [\"clock\", \"clock\"]\n";

    assert_eq!(
        errors(text),
        vec!["2:20: module `clock` is already in [layout].center"]
    );
}

#[test]
fn an_unknown_module_name_is_an_error_with_a_suggestion() {
    let text = "[layout]\nleft = [\"clok\"]\n";

    assert_eq!(
        errors(text),
        vec!["2:9: unknown module `clok` (did you mean `clock`?)"]
    );
}

#[test]
fn a_syntax_error_is_one_diagnostic_at_the_fault() {
    let diagnostics = resolve("[bar]\nheight = = 3\n").unwrap_err();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 10));
}

#[test]
fn a_value_of_the_wrong_type_is_reported_at_the_value() {
    assert_eq!(
        errors("[bar]\nheight = \"tall\"\n"),
        vec!["2:10: invalid type: string \"tall\", expected u32"]
    );
}

#[test]
fn an_unknown_enum_value_lists_the_choices() {
    assert_eq!(
        errors("[bar]\nposition = \"left\"\n"),
        vec!["2:12: unknown variant `left`, expected `top` or `bottom`"]
    );
}

#[test]
fn monitors_takes_all_or_a_list_of_output_names() {
    assert_eq!(
        ok("[bar]\nmonitors = [\"DP-1\"]\n").bar.monitors,
        Monitors::Only(vec!["DP-1".into()])
    );
    assert_eq!(
        errors("[bar]\nmonitors = \"some\"\n"),
        vec!["2:12: invalid value: string \"some\", expected \"all\" or a list of output names"]
    );
}

#[test]
fn focal_can_be_none() {
    assert_eq!(ok("[bar]\nfocal = \"none\"\n").bar.focal, None);
}

#[test]
fn the_default_focal_steps_aside_when_its_module_is_not_on_the_bar() {
    assert_eq!(ok("[layout]\nleft = [\"logo\"]\n").bar.focal, None);
}

#[test]
fn an_explicit_focal_must_be_on_the_bar() {
    let text = "[bar]\nfocal = \"logo\"\n\n[layout]\ncenter = [\"clock\"]\n";

    assert_eq!(
        errors(text),
        vec!["2:9: focal module `logo` is not in [layout]"]
    );
}

#[test]
fn an_unknown_focal_module_is_an_error_with_a_suggestion() {
    assert_eq!(
        errors("[bar]\nfocal = \"clok\"\n"),
        vec!["2:9: unknown module `clok` (did you mean `clock`?)"]
    );
}

#[test]
fn an_invalid_clock_format_is_reported_at_the_format() {
    let diagnostics = resolve("[clock]\nformat = \"%H:%\"\n").unwrap_err();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 10));
    assert!(diagnostics[0].message.starts_with("invalid clock format"));
}

#[test]
fn diagnostics_render_with_the_file_name() {
    let diagnostics = resolve("[bar]\nheigth = 30\n").unwrap_err();

    assert_eq!(
        diagnostics[0].render("config.toml"),
        "config.toml:2:1: unknown key `heigth` in [bar] (did you mean `height`?)"
    );
}

#[test]
fn columns_count_characters_not_bytes() {
    let text = "[layout]\nleft = [\"✨\", \"clok\"]\n";

    assert_eq!(
        errors(text),
        vec![
            "2:9: unknown module `✨`",
            "2:14: unknown module `clok` (did you mean `clock`?)",
        ]
    );
}

#[test]
fn bar_geometry_reaches_the_theme_override_block() {
    let config = ok("[bar]\nheight = 40\nradius = 0\n");

    assert!(config.css_overrides.contains("--zb-bar-height: 40px;"));
    assert!(config.css_overrides.contains("--zb-bar-radius: 0px;"));
}

#[test]
fn layout_sections_left_out_default_to_empty_once_the_layout_is_given() {
    let config = ok("[layout]\ncenter = [\"clock\"]\n");

    assert_eq!(config.layout.left, Vec::<ModuleId>::new());
    assert_eq!(config.layout.center, vec![Clock]);
    assert_eq!(config.layout.right, Vec::<ModuleId>::new());
}
