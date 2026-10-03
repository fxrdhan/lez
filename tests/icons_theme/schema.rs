// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Guards for the shipped `theme.yml` JSON schema and its wiring into the
//! example theme file. The schema forbids keys it does not list, so an
//! editor validating against it rejects any section it leaves out; the
//! keys it should list are taken from the struct lez reads themes into.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use lez::options::config::UiStylesOverride;
use serde_json::{Value, json};

fn docs_file(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("docs/{name} should be readable: {e}"))
}

fn schema() -> Value {
    serde_json::from_str(&docs_file("theme-schema.json")).expect("the schema is valid JSON")
}

/// The sections that hold a fixed set of keys of their own.
const NESTED: [&str; 10] = [
    "filekinds",
    "perms",
    "size",
    "users",
    "links",
    "git",
    "git_repo",
    "security_context",
    "file_type",
    "tags",
];

/// Every key a theme can hold. An empty value for each nested section
/// deserializes to all `None`, and serializing that back names every field.
fn accepted_keys() -> Value {
    let empty = NESTED.map(|section| (section.to_owned(), json!({})));
    let parsed: UiStylesOverride =
        serde_json::from_value(Value::Object(empty.into_iter().collect()))
            .expect("empty sections deserialize");
    serde_json::to_value(parsed).expect("serialize")
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("an object: {value}"))
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn the_schema_documents_exactly_the_keys_a_theme_can_hold() {
    let schema = schema();
    let accepted = accepted_keys();
    assert_eq!(keys(&schema["properties"]), keys(&accepted));
    for section in NESTED {
        assert_eq!(
            keys(&schema["properties"][section]["properties"]),
            keys(&accepted[section]),
            "{section}"
        );
    }
}

#[test]
fn the_schema_is_draft_7_with_shared_definitions() {
    let schema = schema();
    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    assert_eq!(schema["additionalProperties"], false);
    for def in ["color", "style", "icon", "symlink_style", "symlink_color"] {
        assert!(schema["$defs"][def].is_object(), "$defs/{def}");
    }
}

/// A link's style may be a style, `target`, or `target` with ANSI codes.
#[test]
fn filekinds_symlink_allows_the_target_keyword() {
    let schema = schema();
    assert_eq!(
        schema["properties"]["filekinds"]["properties"]["symlink"]["oneOf"],
        json!([
            {"$ref": "#/$defs/symlink_style"},
            {"$ref": "#/$defs/style"},
            {"type": "string", "const": "target"},
            {"type": "string", "pattern": "^(?i:target)(;[0-9]+)+$"}
        ])
    );
    assert_eq!(
        schema["$defs"]["symlink_color"]["oneOf"],
        json!([
            {"$ref": "#/$defs/color"},
            {"type": "string", "pattern": "^(?i:target)(;[0-9]+)*$"}
        ])
    );
}

#[test]
fn example_theme_references_the_schema() {
    assert_eq!(
        docs_file("theme.yml").lines().next(),
        Some(
            "# yaml-language-server: $schema=https://raw.githubusercontent.com/fxrdhan/lez/main/docs/theme-schema.json"
        )
    );
}
