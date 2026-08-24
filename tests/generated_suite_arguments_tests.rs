// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The powertest and Nix-generated suites only run where `tests/test_dir`
//! exists, which in practice means the Nix build. Everywhere else they are
//! skipped, so drift in them stays invisible until CI: nine cases went stale
//! when `--color` began requiring an equals sign, and `always` quietly turned
//! from that flag's value into a path the case then failed to list.
//!
//! Checking that does not need the fixture. A generated case should only ever
//! point at the fixture it was written for, so parse each case's arguments and
//! assert nothing else ended up in the positional slot. A flag that stops (or
//! starts) swallowing the word behind it shows up here, in a test that runs
//! everywhere, rather than a CI round-trip later.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Every fixture root the generated suites are allowed to name.
const FIXTURE_ROOTS: [&str; 3] = [
    "tests/test_dir",
    "tests/timestamp_test_dir",
    "nonexistentdir",
];

/// The cases that feed the parser something invalid on purpose and snapshot
/// the complaint. They have no positional slot to check, so they are listed
/// rather than waved through: a case that starts failing to parse without
/// being named here is drift, not intent.
const EXPECTED_REJECTIONS: [&str; 2] = [
    "long_time_style_custom_non_recent_empty_nix.toml",
    "long_time_style_custom_non_recent_none_nix.toml",
];

fn case_files() -> Vec<PathBuf> {
    let mut cases = Vec::new();
    for dir in ["tests/ptests", "tests/gen"] {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toml") {
                cases.push(path);
            }
        }
    }
    cases.sort();
    cases
}

/// Pull the `args = "…"` value out of a trycmd case, undoing the TOML
/// basic-string escapes these files actually use.
fn args_of(case: &Path) -> Option<String> {
    let text = fs::read_to_string(case).ok()?;
    let line = text.lines().find(|line| line.starts_with("args = \""))?;
    let body = line.strip_prefix("args = \"")?.strip_suffix('"')?;

    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => break,
        }
    }
    Some(out)
}

/// Split the way a shell would, so a quoted value containing spaces stays a
/// single argument.
fn split_args(line: &str) -> Vec<OsString> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quoted = false;

    for c in line.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(OsString::from(std::mem::take(&mut current)));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(OsString::from(current));
    }
    args
}

#[test]
fn every_generated_case_points_only_at_its_fixture() {
    let cases = case_files();
    assert!(
        cases.len() > 150,
        "found only {} generated cases — has the layout moved?",
        cases.len()
    );

    let mut checked = 0;
    let mut rejected = Vec::new();
    for case in &cases {
        let Some(args) = args_of(case) else {
            panic!("no `args` line in {}", case.display());
        };
        let words = split_args(&args);
        if words.is_empty() {
            continue;
        }

        let command = lsr::options::parser::get_command().no_binary_name(true);
        let normalized = lsr::options::parser::normalize_args(words.clone(), &command);
        let matches = match command.try_get_matches_from(normalized) {
            Ok(matches) => matches,
            Err(e)
                if matches!(
                    e.kind(),
                    clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
                ) =>
            {
                checked += 1;
                continue;
            }
            Err(_) => {
                rejected.push(case.file_name().unwrap().to_string_lossy().into_owned());
                checked += 1;
                continue;
            }
        };

        let paths: Vec<String> = matches
            .get_many::<OsString>("FILE")
            .into_iter()
            .flatten()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        for path in &paths {
            assert!(
                FIXTURE_ROOTS.contains(&path.as_str()),
                "{} lists {path:?}, which is not a fixture: a flag stopped taking \
                 the word behind it as its value.\n  args: {args}\n  paths: {paths:?}",
                case.display()
            );
        }
        checked += 1;
    }

    assert_eq!(
        checked,
        cases.len(),
        "every case should have been checked, not just {checked}"
    );

    rejected.sort();
    let mut expected: Vec<String> = EXPECTED_REJECTIONS
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    expected.sort();
    assert_eq!(
        rejected, expected,
        "the set of cases the parser rejects has moved"
    );
}
