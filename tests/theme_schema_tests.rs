// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Guards for the shipped `theme.yml` JSON schema and its wiring into the
//! example theme file.

use std::fs;
use std::path::Path;

fn docs_file(name: &str) -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("docs/{name} should be readable: {e}"))
}

#[test]
fn schema_is_valid_json_with_expected_structure() {
    let schema: serde_json::Value =
        serde_json::from_str(&docs_file("theme-schema.json")).expect("schema must parse");
    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    for def in ["color", "style", "icon"] {
        assert!(schema["$defs"][def].is_object(), "$defs/{def} must exist");
    }
}

#[test]
fn schema_covers_every_theme_section_lsr_accepts() {
    let schema: serde_json::Value = serde_json::from_str(&docs_file("theme-schema.json")).unwrap();
    let props = schema["properties"].as_object().unwrap();
    for section in [
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
        "punctuation",
        "date",
        "inode",
        "blocks",
        "header",
        "octal",
        "flags",
        "symlink_path",
        "control_char",
        "broken_symlink",
        "filenames",
        "extensions",
        "directorynames",
        "mimetypes",
    ] {
        assert!(
            props.contains_key(section),
            "schema is missing the {section} section"
        );
    }
}

#[test]
fn filekinds_symlink_allows_the_target_keyword() {
    let schema: serde_json::Value = serde_json::from_str(&docs_file("theme-schema.json")).unwrap();
    let symlink = &schema["properties"]["filekinds"]["properties"]["symlink"];
    let text = serde_json::to_string(symlink).unwrap();
    assert!(text.contains("\"target\""), "symlink must allow target");
}

#[test]
fn example_theme_references_the_schema() {
    let yml = docs_file("theme.yml");
    assert!(
        yml.contains("yaml-language-server: $schema="),
        "example theme should reference the schema for editor validation"
    );
    assert!(
        yml.contains("theme-schema.json"),
        "schema reference must point at theme-schema.json"
    );
}
