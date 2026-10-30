//! The theming contract: every theme defines every variable, component CSS reads only
//! those, and docs/theming.md documents every variable and class.

use std::collections::BTreeSet;
use std::path::Path;

use zenith_bar::config::{REQUIRED_VARS, ThemeName};
use zenith_bar::modules::ModuleId;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Identifiers following each occurrence of `prefix`, e.g. `--zb-` → `--zb-accent`.
fn names(text: &str, prefix: &str) -> BTreeSet<String> {
    text.match_indices(prefix)
        .map(|(start, _)| {
            let rest = &text[start + prefix.len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .unwrap_or(rest.len());
            format!("{prefix}{}", &rest[..end])
        })
        .collect()
}

/// Declared variables: `--zb-x:`.
fn declared(css: &str) -> BTreeSet<String> {
    names(css, "--zb-")
        .into_iter()
        .filter(|var| css.contains(&format!("{var}:")))
        .collect()
}

const COMPONENT_SHEETS: [&str; 3] = ["css/base.css", "css/ambient.css", "css/motion-off.css"];

/// All component sheets, comments removed.
fn component_css() -> String {
    let css = COMPONENT_SHEETS
        .map(|sheet| read(&format!("resources/{sheet}")))
        .join("\n");
    css.split("/*")
        .map(|chunk| chunk.split_once("*/").map_or(chunk, |(_, rest)| rest))
        .collect()
}

fn required() -> BTreeSet<String> {
    REQUIRED_VARS.iter().map(|v| v.to_string()).collect()
}

#[test]
fn every_theme_defines_exactly_the_required_variables() {
    for theme in ThemeName::ALL {
        let css = read(&format!("resources/{}", theme.resource()));
        assert_eq!(declared(&css), required(), "{theme:?}");
    }
}

#[test]
fn component_css_reads_only_required_variables_and_declares_none() {
    let css = component_css();
    let unknown: Vec<_> = names(&css, "--zb-")
        .difference(&required())
        .cloned()
        .collect();

    assert_eq!(unknown, Vec::<String>::new());
    assert_eq!(declared(&css), BTreeSet::new());
}

#[test]
fn component_css_has_no_literal_colours() {
    let css = component_css();

    for literal in ["#", "rgb(", "rgba(", "hsl("] {
        assert!(!css.contains(literal), "found `{literal}`");
    }
}

#[test]
fn the_theming_docs_list_every_variable() {
    let docs = read("docs/theming.md");
    let missing: Vec<_> = REQUIRED_VARS
        .iter()
        .filter(|var| !docs.contains(&format!("`{var}`")))
        .collect();

    assert_eq!(missing, Vec::<&&str>::new());
}

#[test]
fn the_theming_docs_list_every_class_the_stylesheets_use() {
    let docs = read("docs/theming.md");
    let css = component_css();
    let classes: BTreeSet<String> = names(&css, ".zb-")
        .into_iter()
        .map(|class| class[1..].to_string())
        .chain(
            [
                "zb-window",
                "zb-bar",
                "zb-section",
                "zb-left",
                "zb-center",
                "zb-right",
                "zb-module",
                "zb-focal",
            ]
            .map(String::from),
        )
        .chain(ModuleId::ALL.map(|id| format!("zb-{id}")))
        .collect();
    let missing: Vec<_> = classes
        .iter()
        .filter(|class| !docs.contains(&format!("`{class}`")))
        .collect();

    assert_eq!(missing, Vec::<&String>::new());
}
