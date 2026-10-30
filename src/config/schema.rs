//! The JSON Schema for the config file, generated from the config types.
//!
//! Committed as `schema/config.schema.json` (kept in sync by `tests/schema_drift.rs`)
//! so TOML editors can offer autocomplete and docs. Draft-07 for the widest editor support.

use schemars::generate::SchemaSettings;
use serde_json::Value;

use super::raw::RawConfig;

/// Where the committed schema is published; referenced by the starter config's `#:schema` line.
pub const URL: &str =
    "https://raw.githubusercontent.com/CPT-Dawn/Zenith/main/schema/config.schema.json";

/// The schema as pretty-printed JSON with a trailing newline.
pub fn generate() -> String {
    let schema = SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<RawConfig>();
    let mut value = schema.to_value();
    close_objects(&mut value);
    if let Value::Object(root) = &mut value {
        root.insert("$id".into(), URL.into());
        root.insert("title".into(), "Zenith bar config".into());
    }
    let mut text = serde_json::to_string_pretty(&value).expect("schema serializes");
    text.push('\n');
    text
}

/// Mark every object with known properties as closed, so editors flag unknown
/// keys exactly as `zenith-bar check` does.
fn close_objects(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("properties") && !map.contains_key("additionalProperties") {
                map.insert("additionalProperties".into(), Value::Bool(false));
            }
            map.values_mut().for_each(close_objects);
        }
        Value::Array(items) => items.iter_mut().for_each(close_objects),
        _ => {}
    }
}
