//! Walks the parsed document and collects every problem, with positions.

use serde_json::{Map, Value};
use toml_edit::{Document, Item, TableLike};

use super::Diagnostic;
use super::actions::ClickActions;
use super::layout::SECTIONS;
use super::raw::RawConfig;
use crate::modules::ModuleId;

/// Structural checks on the document before deserializing.
pub fn document(text: &str, document: &Document<&str>) -> Vec<Diagnostic> {
    let schema = schemars::schema_for!(RawConfig);
    let root = schema.as_value();
    let mut out = Vec::new();
    unknown_keys(
        text,
        root,
        root,
        document.as_table(),
        &mut Vec::new(),
        &mut out,
    );
    layout(text, document, &mut out);
    actions(text, document, &mut out);
    out
}

/// Click actions are strings. Checked here because serde loses positions inside
/// the flattened `ClickActions`.
fn actions(text: &str, document: &Document<&str>, out: &mut Vec<Diagnostic>) {
    for id in ModuleId::ALL {
        for key in ClickActions::KEYS {
            let Some(item) = lookup(document, &[id.as_str(), key]) else {
                continue;
            };
            if item.as_str().is_none() {
                out.push(Diagnostic::at(
                    text,
                    item.span(),
                    format!("`{key}` must be a string holding a shell command"),
                ));
            }
        }
    }
}

/// Module names must exist and appear at most once across all sections.
fn layout(text: &str, document: &Document<&str>, out: &mut Vec<Diagnostic>) {
    let mut seen: Vec<(ModuleId, &str)> = Vec::new();
    for section in SECTIONS {
        let Some(array) = lookup(document, &["layout", section]).and_then(Item::as_array) else {
            continue;
        };
        for value in array.iter() {
            let Some(name) = value.as_str() else { continue };
            let span = value.span();
            match name.parse::<ModuleId>() {
                Err(()) => {
                    let names = ModuleId::ALL.map(ModuleId::as_str);
                    out.push(
                        Diagnostic::at(text, span, format!("unknown module `{name}`"))
                            .suggest(closest(name, names)),
                    );
                }
                Ok(id) => match seen.iter().find(|(seen_id, _)| *seen_id == id) {
                    Some((_, first)) => out.push(Diagnostic::at(
                        text,
                        span,
                        format!("module `{name}` is already in [layout].{first}"),
                    )),
                    None => seen.push((id, section)),
                },
            }
        }
    }
}

/// Report every key the schema does not know, recursing into known tables.
/// The schema is the single source of truth for which keys exist.
fn unknown_keys(
    text: &str,
    root: &Value,
    schema: &Value,
    table: &dyn TableLike,
    path: &mut Vec<String>,
    out: &mut Vec<Diagnostic>,
) {
    let Some(known) = properties(root, schema) else {
        return;
    };
    for (key, item) in table.iter() {
        let span = table.key(key).and_then(|k| k.span());
        match known.get(key) {
            Some(child) => {
                if let Some(child_table) = item.as_table_like() {
                    path.push(key.to_string());
                    unknown_keys(text, root, child, child_table, path, out);
                    path.pop();
                }
            }
            None => {
                let message = if path.is_empty() && item.is_table_like() {
                    format!("unknown table [{key}]")
                } else if path.is_empty() {
                    format!("unknown key `{key}`")
                } else {
                    format!("unknown key `{key}` in [{}]", path.join("."))
                };
                let suggestion = closest(key, known.keys().map(String::as_str));
                out.push(Diagnostic::at(text, span, message).suggest(suggestion));
            }
        }
    }
}

/// The `properties` of an object schema, following `$ref` and `*Of` wrappers.
fn properties<'s>(root: &'s Value, schema: &'s Value) -> Option<&'s Map<String, Value>> {
    let schema = resolve_ref(root, schema);
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        return Some(props);
    }
    ["allOf", "anyOf", "oneOf"]
        .iter()
        .filter_map(|k| schema.get(*k)?.as_array())
        .flatten()
        .find_map(|s| properties(root, s))
}

fn resolve_ref<'s>(root: &'s Value, schema: &'s Value) -> &'s Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(reference) => reference
            .strip_prefix("#")
            .and_then(|pointer| root.pointer(pointer))
            .map_or(schema, |target| resolve_ref(root, target)),
        None => schema,
    }
}

/// The candidate most similar to `word`, if any is close enough to be a likely typo.
pub fn closest<'a>(word: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    candidates
        .into_iter()
        .map(|c| (strsim::jaro_winkler(word, c), c))
        .filter(|(score, _)| *score >= 0.8)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c.to_string())
}

/// Checks on deserialized values that need the document for positions.
pub fn values(text: &str, document: &Document<&str>, raw: &RawConfig) -> Vec<Diagnostic> {
    let at = |path: &[&str], message: String| {
        let span = lookup(document, path).and_then(Item::span);
        Diagnostic::at(text, span, message)
    };
    let mut out = Vec::new();

    let focal = raw.bar.focal.as_str();
    if focal != "none" {
        match focal.parse::<ModuleId>() {
            Err(()) => out.push(
                at(&["bar", "focal"], format!("unknown module `{focal}`"))
                    .suggest(closest(focal, ModuleId::ALL.map(ModuleId::as_str))),
            ),
            Ok(_) => {
                let layout = &raw.layout;
                let on_bar = [&layout.left, &layout.center, &layout.right]
                    .into_iter()
                    .flatten()
                    .any(|name| name == focal);
                if !on_bar && lookup(document, &["bar", "focal"]).is_some() {
                    out.push(at(
                        &["bar", "focal"],
                        format!("focal module `{focal}` is not in [layout]"),
                    ));
                }
            }
        }
    }

    // Per-module checks: (table, key, message).
    let module_problems = [("clock", raw.clock.problems())];
    for (table, problems) in module_problems {
        for (key, message) in problems {
            out.push(at(&[table, key], message));
        }
    }

    out
}

/// The item at `path`, if the user wrote it.
pub fn lookup<'d>(document: &'d Document<&str>, path: &[&str]) -> Option<&'d Item> {
    let mut item = document.as_item();
    for key in path {
        item = item.as_table_like()?.get(key)?;
    }
    Some(item)
}

/// Diagnostic messages are lowercase with no trailing period and a single line.
pub fn tidy(message: &str) -> String {
    let message = message.lines().next().unwrap_or_default().trim();
    let message = message.trim_end_matches('.');
    let mut chars = message.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}
