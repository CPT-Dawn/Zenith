//! Deserializing the config so that every bad value is reported, not just the first.
//!
//! serde stops at the first error in a struct, so each key of each table is
//! deserialized on its own (every field has a default, so a one-key table is a
//! complete value). The keys that pass are then deserialized together.
//!
//! Limit: inside a nested table (`[system.temperature]`) or an array, serde still
//! stops at the first bad value, so only one error per nested value is reported.

use serde::Deserialize;
use serde::de::{DeserializeOwned, IntoDeserializer};
use toml::Spanned;
use toml::de::{DeTable, DeValue};

use super::Diagnostic;
use super::actions::ClickActions;
use super::bar::BarConfig;
use super::layout::LayoutConfig;
use super::raw::RawConfig;
use super::theme::ThemeConfig;
use super::validate::tidy;
use crate::modules::calendar::CalendarConfig;
use crate::modules::media::MediaConfig;
use crate::modules::todo::TodoConfig;
use crate::modules::{clock::ClockConfig, logo::LogoConfig, system::SystemConfig};

/// The keys of one table that fail to deserialize, with their errors.
type Check = fn(&Spanned<DeValue<'_>>) -> Vec<(Option<String>, toml::de::Error)>;

/// One line per [`RawConfig`] field.
const TABLES: [(&str, Check); 9] = [
    ("bar", each_key::<BarConfig>),
    ("layout", each_key::<LayoutConfig>),
    ("clock", each_key::<ClockConfig>),
    ("logo", each_key::<LogoConfig>),
    ("media", each_key::<MediaConfig>),
    ("theme", each_key::<ThemeConfig>),
    ("calendar", each_key::<CalendarConfig>),
    ("system", each_key::<SystemConfig>),
    ("todo", each_key::<TodoConfig>),
];

/// What deserializing produced: the config built from every value that is valid,
/// a diagnostic per invalid one, and the tables that had any.
pub struct Deserialized {
    pub raw: RawConfig,
    pub diagnostics: Vec<Diagnostic>,
    pub broken_tables: Vec<&'static str>,
}

/// Deserialize `text`, which already parsed as TOML.
pub fn deserialize(text: &str) -> Deserialized {
    let mut out = Deserialized {
        raw: RawConfig::default(),
        diagnostics: Vec::new(),
        broken_tables: Vec::new(),
    };
    let root = match DeTable::parse(text) {
        Ok(root) => root,
        Err(e) => {
            out.diagnostics.push(diagnostic(text, &e));
            return out;
        }
    };
    let span = root.span();
    let mut valid = DeTable::new();
    for (key, value) in root.into_inner() {
        let Some(&(name, check)) = TABLES.iter().find(|(name, _)| *name == key.get_ref()) else {
            // Unknown tables are reported by the structural checks.
            valid.insert(key, value);
            continue;
        };
        let failures = check(&value);
        if failures.is_empty() {
            valid.insert(key, value);
            continue;
        }
        out.broken_tables.push(name);
        let failed: Vec<Option<String>> = failures.iter().map(|(k, _)| k.clone()).collect();
        // Click actions are flattened, so serde has no position for them; the
        // structural checks report those at the value instead.
        let action = |k: &Option<String>| {
            k.as_deref()
                .is_some_and(|k| ClickActions::KEYS.contains(&k))
        };
        out.diagnostics.extend(
            failures
                .iter()
                .filter(|(k, _)| !action(k))
                .map(|(_, e)| diagnostic(text, e)),
        );
        // Keep the table's good keys; a table that is not a table at all goes.
        if let (DeValue::Table(table), false) = (value.get_ref(), failed.contains(&None)) {
            let kept = table
                .iter()
                .filter(|(k, _)| !failed.contains(&Some(k.get_ref().to_string())))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            valid.insert(key, Spanned::new(value.span(), DeValue::Table(kept)));
        }
    }
    let valid = Spanned::new(span, DeValue::Table(valid));
    match RawConfig::deserialize(valid.into_deserializer()) {
        Ok(raw) => out.raw = raw,
        // Keys that pass alone pass together; this is only a last resort.
        Err(e) => out.diagnostics.push(diagnostic(text, &e)),
    }
    out
}

/// Deserialize a table one key at a time; `None` means the value is not a table.
fn each_key<T: DeserializeOwned>(
    value: &Spanned<DeValue<'_>>,
) -> Vec<(Option<String>, toml::de::Error)> {
    let DeValue::Table(table) = value.get_ref() else {
        return T::deserialize(value.clone().into_deserializer())
            .err()
            .map(|e| (None, e))
            .into_iter()
            .collect();
    };
    table
        .iter()
        .filter_map(|(key, item)| {
            let mut one = DeTable::new();
            one.insert(key.clone(), item.clone());
            let one = Spanned::new(value.span(), DeValue::Table(one));
            let error = T::deserialize(one.into_deserializer()).err()?;
            Some((Some(key.get_ref().to_string()), error))
        })
        .collect()
}

fn diagnostic(text: &str, e: &toml::de::Error) -> Diagnostic {
    Diagnostic::at(text, e.span(), tidy(e.message()))
}
