// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Regression guards for the direct `palette_derive = "=0.7.5"` pin.
//!
//! `palette` declares its proc-macro dependency with a caret requirement, so
//! pinning `palette` alone still resolves `palette_derive` 0.7.6+, which pulls
//! in `by_address`. Only a direct pin on the derive crate has any effect.

use std::fs;
use std::path::Path;

fn workspace_file(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|e| panic!("{name} should be readable: {e}"))
}

fn packages(lockfile: &str) -> Vec<(&str, &str)> {
    lockfile
        .split("[[package]]")
        .skip(1)
        .filter_map(|block| {
            let name = field(block, "name")?;
            let version = field(block, "version")?;
            Some((name, version))
        })
        .collect()
}

fn field<'a>(block: &'a str, key: &str) -> Option<&'a str> {
    block.lines().find_map(|line| {
        line.strip_prefix(key)?
            .strip_prefix(" = \"")?
            .strip_suffix('"')
    })
}

#[test]
fn palette_derive_is_directly_pinned_in_manifest() {
    let manifest = workspace_file("Cargo.toml");
    assert!(
        manifest.contains("palette_derive = \"=0.7.5\""),
        "palette_derive must be pinned directly; pinning palette alone is ineffective"
    );
}

#[test]
fn palette_derive_resolves_to_pinned_version() {
    let lockfile = workspace_file("Cargo.lock");
    let derived = packages(&lockfile)
        .into_iter()
        .find(|(name, _)| *name == "palette_derive")
        .expect("palette_derive must appear in Cargo.lock");
    assert_eq!(derived.1, "0.7.5");
}

#[test]
fn palette_stays_on_known_good_version() {
    let lockfile = workspace_file("Cargo.lock");
    let derived = packages(&lockfile)
        .into_iter()
        .find(|(name, _)| *name == "palette")
        .expect("palette must appear in Cargo.lock");
    assert_eq!(derived.1, "0.7.5");
}

#[test]
fn by_address_never_enters_the_dependency_graph() {
    let lockfile = workspace_file("Cargo.lock");
    assert!(
        !packages(&lockfile)
            .iter()
            .any(|(name, _)| *name == "by_address"),
        "by_address must not resolve into the dependency graph"
    );
}
