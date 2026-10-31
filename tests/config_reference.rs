//! The configuration reference in docs/configuration.md must match the config types:
//! it documents every key the schema has (and no others), its defaults are the real
//! defaults, and its examples are valid configs.
//!
//! Reference format: a `### `[table]`` heading, then rows of
//! `| `key` | `default` | `example` | description |`. A default that is not in
//! backticks (such as "unset") means the key has no value unless you write one.
//! Rows under `### Click actions` apply to every table whose schema has those keys.

use std::collections::BTreeSet;
use std::path::Path;

use pretty_assertions::assert_eq;
use serde_json::Value;
use zenith_bar::config::{resolve, schema};

const CLICK_ACTIONS: &str = "Click actions";

#[derive(Debug)]
struct Row {
    table: String,
    key: String,
    default: Option<String>,
    example: Option<String>,
}

fn docs() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/configuration.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The text between the first pair of backticks, if the cell is one code span.
fn code(cell: &str) -> Option<String> {
    let cell = cell.trim();
    let inner = cell.strip_prefix('`')?.strip_suffix('`')?;
    Some(inner.to_string())
}

fn rows() -> Vec<Row> {
    let mut table = None;
    let mut rows = Vec::new();
    for line in docs().lines() {
        if let Some(heading) = line.strip_prefix("### ") {
            table = if heading.trim() == CLICK_ACTIONS {
                Some(CLICK_ACTIONS.to_string())
            } else {
                code(heading)
                    .and_then(|h| Some(h.strip_prefix('[')?.strip_suffix(']')?.to_string()))
            };
            continue;
        }
        if line.starts_with("## ") {
            table = None;
            continue;
        }
        let (Some(table), Some(rest)) = (&table, line.strip_prefix('|')) else {
            continue;
        };
        let cells: Vec<&str> = rest.split(" | ").collect();
        let Some(key) = cells.first().and_then(|c| code(c)) else {
            continue;
        };
        rows.push(Row {
            table: table.clone(),
            key,
            default: cells.get(1).and_then(|c| code(c)),
            example: cells.get(2).and_then(|c| code(c)),
        });
    }
    rows
}

/// Every `table.key` the schema accepts, with nested tables flattened
/// (`system.temperature.warm`).
fn schema_keys() -> BTreeSet<(String, String)> {
    let schema: Value = serde_json::from_str(&schema::generate()).unwrap();
    let table_of = |property: &Value| -> Option<Value> {
        let reference = property["$ref"]
            .as_str()
            .or_else(|| property["allOf"][0]["$ref"].as_str())?;
        let name = reference.strip_prefix("#/definitions/")?;
        let definition = &schema["definitions"][name];
        definition["properties"]
            .is_object()
            .then(|| definition.clone())
    };

    let mut keys = BTreeSet::new();
    let mut pending: Vec<(String, Value)> = schema["properties"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, property)| (name.clone(), table_of(property).unwrap()))
        .collect();
    while let Some((table, definition)) = pending.pop() {
        for (key, property) in definition["properties"].as_object().unwrap() {
            match table_of(property) {
                Some(nested) => pending.push((format!("{table}.{key}"), nested)),
                None => {
                    keys.insert((table.clone(), key.clone()));
                }
            }
        }
    }
    keys
}

/// Documented rows with the click-action rows expanded onto every table that has them.
fn documented() -> Vec<Row> {
    let schema = schema_keys();
    rows()
        .into_iter()
        .flat_map(|row| {
            if row.table != CLICK_ACTIONS {
                return vec![row];
            }
            schema
                .iter()
                .filter(|(_, key)| *key == row.key)
                .map(|(table, key)| Row {
                    table: table.clone(),
                    key: key.clone(),
                    default: row.default.clone(),
                    example: row.example.clone(),
                })
                .collect()
        })
        .collect()
}

#[test]
fn the_reference_documents_every_key_and_no_others() {
    let documented: BTreeSet<_> = documented()
        .into_iter()
        .map(|row| (row.table, row.key))
        .collect();

    assert_eq!(documented, schema_keys());
}

#[test]
fn every_key_has_an_example() {
    let missing: Vec<_> = documented()
        .into_iter()
        .filter(|row| row.example.is_none())
        .map(|row| format!("{}.{}", row.table, row.key))
        .collect();

    assert_eq!(missing, Vec::<String>::new());
}

#[test]
fn the_documented_defaults_are_the_real_defaults() {
    let mut text = String::new();
    let mut current = None;
    for row in documented() {
        let Some(default) = row.default else { continue };
        if current.as_ref() != Some(&row.table) {
            text.push_str(&format!("\n[{}]\n", row.table));
            current = Some(row.table.clone());
        }
        text.push_str(&format!("{} = {default}\n", row.key));
    }

    let written = resolve(&text, None)
        .unwrap_or_else(|e| panic!("documented defaults are invalid: {e:?}\n{text}"));
    assert_eq!(written, resolve("", None).unwrap(), "\n{text}");
}

#[test]
fn every_documented_example_is_a_valid_config() {
    for row in documented() {
        let Some(example) = row.example else { continue };
        let text = format!("[{}]\n{} = {example}\n", row.table, row.key);

        assert!(
            resolve(&text, None).is_ok(),
            "{:?}\n{text}",
            resolve(&text, None).err()
        );
    }
}
