use pretty_assertions::assert_eq;
use zenith_bar::config::{Action, Diagnostic, Gesture, RawConfig, ResolvedConfig, resolve};
use zenith_bar::modules::ModuleId::{self, *};

fn ok(text: &str) -> ResolvedConfig {
    match resolve(text) {
        Ok(config) => config,
        Err(diagnostics) => panic!("expected a valid config, got {diagnostics:#?}"),
    }
}

#[test]
fn a_configured_command_replaces_the_built_in_action() {
    let config = ok("[logo]\non_click = \"rofi -show drun\"\n");

    assert_eq!(
        config.actions(Logo).action(Gesture::Click),
        Action::Run("rofi -show drun")
    );
}

/// Modules that have a config table. Every one of them must take the action keys.
fn module_tables() -> Vec<ModuleId> {
    let schema = schemars::schema_for!(RawConfig);
    let tables = schema.as_value()["properties"].as_object().unwrap().clone();
    ModuleId::ALL
        .into_iter()
        .filter(|id| tables.contains_key(id.as_str()))
        .collect()
}

#[test]
fn every_module_table_takes_every_action_key() {
    let tables = module_tables();
    assert!(tables.contains(&Clock) && tables.contains(&Logo));

    for id in tables {
        for (key, gesture) in [
            ("on_click", Gesture::Click),
            ("on_right_click", Gesture::RightClick),
            ("on_scroll_up", Gesture::ScrollUp),
            ("on_scroll_down", Gesture::ScrollDown),
        ] {
            let config = ok(&format!("[{id}]\n{key} = \"notify-send {id}\"\n"));
            assert_eq!(
                config.actions(id).action(gesture),
                Action::Run(&format!("notify-send {id}")),
                "[{id}].{key}"
            );
        }
    }
}

#[test]
fn the_action_keys_are_described_in_the_schema() {
    let schema = serde_json::to_string(&schemars::schema_for!(RawConfig)).unwrap();

    for key in [
        "on_click",
        "on_right_click",
        "on_scroll_up",
        "on_scroll_down",
    ] {
        assert!(schema.contains(&format!("\"{key}\"")), "{key} missing");
    }
    assert!(schema.contains("Shell command run on left click"));
}

#[test]
fn a_gesture_without_a_command_keeps_the_built_in_action() {
    let config = ok("[logo]\non_click = \"rofi -show drun\"\n");

    assert_eq!(
        config.actions(Logo).action(Gesture::RightClick),
        Action::BuiltIn
    );
    for id in ModuleId::ALL {
        for gesture in Gesture::ALL {
            assert_eq!(ok("").actions(id).action(gesture), Action::BuiltIn);
        }
    }
}

#[test]
fn an_empty_command_disables_the_built_in_action() {
    let config = ok("[clock]\non_scroll_down = \"\"\n");

    assert_eq!(
        config.actions(Clock).action(Gesture::ScrollDown),
        Action::Run("")
    );
}

#[test]
fn commands_belong_to_the_table_they_are_written_in() {
    let config = ok(
        "[clock]\non_scroll_up = \"brightnessctl s +5%\"\n\n[logo]\non_right_click = \"wlogout\"\n",
    );

    assert_eq!(
        config.actions(Clock).action(Gesture::ScrollUp),
        Action::Run("brightnessctl s +5%")
    );
    assert_eq!(
        config.actions(Clock).action(Gesture::RightClick),
        Action::BuiltIn
    );
    assert_eq!(
        config.actions(Logo).action(Gesture::ScrollUp),
        Action::BuiltIn
    );
    assert_eq!(
        config.actions(Logo).action(Gesture::RightClick),
        Action::Run("wlogout")
    );
    // The module's own keys still resolve alongside the action keys.
    assert_eq!(config.clock.format, "%H:%M");
}

#[test]
fn a_misspelt_action_key_is_reported_with_a_suggestion() {
    let text = "[logo]\non_clik = \"rofi\"\n";

    assert_eq!(
        resolve(text),
        Err(vec![Diagnostic {
            line: 2,
            column: 1,
            message: "unknown key `on_clik` in [logo]".into(),
            suggestion: Some("on_click".into()),
        }])
    );
}

#[test]
fn an_action_that_is_not_a_string_is_reported_at_the_value() {
    let errors: Vec<String> = resolve("[clock]\non_click = 5\n\n[logo]\non_scroll_up = [\"a\"]\n")
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect();

    assert_eq!(
        errors,
        vec![
            "2:12: `on_click` must be a string holding a shell command",
            "5:16: `on_scroll_up` must be a string holding a shell command",
        ]
    );
}
