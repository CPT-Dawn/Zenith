//! The committed JSON Schema must match the one generated from the config types.
//! Regenerate with: `UPDATE_SCHEMA=1 cargo test --test schema_drift`

use std::path::Path;

use pretty_assertions::assert_eq;
use serde_json::Value;
use zenith_bar::config::schema;

const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/config.schema.json");

#[test]
fn committed_schema_matches_the_config_types() {
    let generated = schema::generate();

    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::create_dir_all(Path::new(COMMITTED).parent().unwrap()).unwrap();
        std::fs::write(COMMITTED, &generated).unwrap();
        return;
    }

    let committed = std::fs::read_to_string(COMMITTED).unwrap_or_default();
    assert_eq!(
        committed, generated,
        "schema/config.schema.json is stale; run `UPDATE_SCHEMA=1 cargo test --test schema_drift`"
    );
}

#[test]
fn schema_lets_editors_flag_unknown_keys() {
    let schema: Value = serde_json::from_str(&schema::generate()).unwrap();

    assert_eq!(schema["additionalProperties"], Value::Bool(false));
    let bar = &schema["definitions"]["BarConfig"];
    assert_eq!(
        bar["description"],
        "The `[bar]` table: placement, geometry and motion of every bar."
    );
    assert_eq!(bar["additionalProperties"], Value::Bool(false));
    assert_eq!(
        bar["properties"]["height"]["description"],
        "Bar height in pixels."
    );
}

#[test]
fn schema_has_a_stable_id_matching_the_starter_config() {
    let schema: Value = serde_json::from_str(&schema::generate()).unwrap();

    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    assert_eq!(
        schema["$id"],
        "https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json"
    );
}
