// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `[package.metadata.binstall]` in `Cargo.toml`, which tells `cargo binstall`
//! where a release keeps its prebuilt binaries.
//!
//! Nothing in this project reads that table. cargo-binstall reads the copy
//! crates.io holds, so a mistake in it shows only once a release is out, to
//! whoever runs `cargo binstall lez`. v0.29.0 went out with
//! `pkg-fmt = "tar.gz"`, which is not a format cargo-binstall has a name for,
//! and it refused the whole manifest. These hold the table to the formats
//! cargo-binstall reads and to the archives the release workflow uploads.

use std::fs;
use std::path::Path;

/// cargo-binstall's `PkgFmt`: each name `pkg-fmt` may take, with the suffixes
/// it tries one after another where `pkg-url` has `{ archive-suffix }`, or
/// `{ archive-format }` for the same without the dot.
const PKG_FMTS: &[(&str, &[&str])] = &[
    ("tar", &[".tar"]),
    ("tbz2", &[".tbz2", ".tar.bz2", ".tbz", ".tar.bz"]),
    ("tgz", &[".tgz", ".tar.gz"]),
    ("txz", &[".txz", ".tar.xz"]),
    ("tzstd", &[".tzstd", ".tzst", ".tar.zst"]),
    ("zip", &[".zip"]),
    ("bin", &[".bin", "", ".exe"]),
];

fn workspace_file(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()))
}

fn manifest() -> toml::Table {
    workspace_file("Cargo.toml")
        .parse()
        .expect("Cargo.toml should be valid TOML")
}

fn binstall(manifest: &toml::Table) -> &toml::Table {
    manifest["package"]["metadata"]["binstall"]
        .as_table()
        .expect("[package.metadata.binstall] should be a table")
}

/// What `key` is for `target`: its override's value, or else the table's own.
fn setting<'a>(binstall: &'a toml::Table, target: &str, key: &str) -> &'a str {
    binstall
        .get("overrides")
        .and_then(|overrides| overrides.get(target))
        .and_then(|table| table.get(key))
        .or_else(|| binstall.get(key))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("the binstall metadata should give `{key}` as a string"))
}

/// Each target the release workflow builds, with the archive it packs it in.
fn release_archives() -> Vec<(String, String)> {
    let workflow: serde_norway::Value =
        serde_norway::from_str(&workspace_file(".github/workflows/release.yml"))
            .expect("release.yml should be valid YAML");
    let rows = workflow["jobs"]["binaries"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("the binaries job should build a matrix of targets");
    rows.iter()
        .map(|row| {
            let field = |name: &str| {
                row[name]
                    .as_str()
                    .unwrap_or_else(|| panic!("each matrix row should give `{name}`"))
                    .to_owned()
            };
            (field("target"), field("archive"))
        })
        .collect()
}

/// Fills in a template as cargo-binstall does, where a key is written
/// `{ key }`. A key it is not given is one cargo-binstall does not fill in
/// either, and it refuses the template.
fn render(template: &str, values: &[(&str, &str)]) -> String {
    let mut rendered = String::new();
    let mut rest = template;
    while let Some((before, after)) = rest.split_once('{') {
        rendered.push_str(before);
        let (key, tail) = after
            .split_once('}')
            .unwrap_or_else(|| panic!("`{template}` opens a key it does not close"));
        let key = key.trim();
        let (_, value) = values
            .iter()
            .find(|(name, _)| *name == key)
            .unwrap_or_else(|| panic!("`{template}` uses `{key}`, which is not filled in here"));
        rendered.push_str(value);
        rest = tail;
    }
    rendered.push_str(rest);
    rendered
}

/// `pkg-fmt`, in the table and in each target's override, is a format
/// cargo-binstall has a name for. Given any other, it reads nothing else of
/// the manifest.
#[test]
fn every_pkg_fmt_is_one_cargo_binstall_reads() {
    let manifest = manifest();
    let binstall = binstall(&manifest);
    let overrides = binstall
        .get("overrides")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .map(|(target, table)| {
            let table = table
                .as_table()
                .unwrap_or_else(|| panic!("the override for {target} should be a table"));
            (format!("the override for {target}"), table)
        });
    let tables = std::iter::once(("[package.metadata.binstall]".to_owned(), binstall));

    let mut formats = 0;
    for (place, table) in tables.chain(overrides) {
        let Some(format) = table.get("pkg-fmt") else {
            continue;
        };
        let format = format
            .as_str()
            .unwrap_or_else(|| panic!("{place}: pkg-fmt should be a string"));
        assert!(
            PKG_FMTS.iter().any(|(name, _)| *name == format),
            "{place} has pkg-fmt = \"{format}\", which cargo-binstall does not read"
        );
        formats += 1;
    }
    assert!(formats > 0, "the binstall metadata should give a pkg-fmt");
}

/// For each target the release workflow builds, cargo-binstall asks for the
/// archive the workflow uploads: one of the suffixes it tries for the
/// target's `pkg-fmt` is the workflow's, and `pkg-url` with it is the file's
/// address in the release.
#[test]
fn each_release_archive_is_one_cargo_binstall_asks_for() {
    let manifest = manifest();
    let binstall = binstall(&manifest);
    let package = |key: &str| {
        manifest["package"][key]
            .as_str()
            .unwrap_or_else(|| panic!("[package] should give `{key}`"))
    };
    let (name, version, repo) = (package("name"), package("version"), package("repository"));
    let workflow = workspace_file(".github/workflows/release.yml");

    let archives = release_archives();
    assert!(
        !archives.is_empty(),
        "the release workflow builds no target"
    );
    for (target, archive) in archives {
        // The name the workflow gives the file it uploads.
        assert!(
            workflow.contains(&format!("lez_${{{{ matrix.target }}}}.{archive}")),
            "{target}: no step of the workflow packs a lez_<target>.{archive}"
        );

        let format = setting(binstall, &target, "pkg-fmt");
        let suffixes = PKG_FMTS
            .iter()
            .find(|(name, _)| *name == format)
            .map(|(_, suffixes)| *suffixes)
            .unwrap_or_else(|| panic!("{target}: pkg-fmt = \"{format}\" is not a format"));
        let suffix = format!(".{archive}");
        assert!(
            suffixes.contains(&suffix.as_str()),
            "{target}: the workflow uploads a {suffix}, and for pkg-fmt = \"{format}\" \
             cargo-binstall asks only for {suffixes:?}"
        );

        let url = render(
            setting(binstall, &target, "pkg-url"),
            &[
                ("repo", repo),
                ("name", name),
                ("version", version),
                ("target", &target),
                ("archive-suffix", &suffix),
                ("archive-format", &archive),
            ],
        );
        assert_eq!(
            url,
            format!("{repo}/releases/download/v{version}/lez_{target}.{archive}"),
            "{target}"
        );
    }
}
