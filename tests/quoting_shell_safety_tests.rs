// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The point of quoting a file name is that a shell reads back the name that
//! is on disk. Asserting against a string someone typed only proves the code
//! agrees with whoever wrote the test, so hand each printed name to `sh` and
//! ask whether the file it names exists.
//!
//! A name holding both an apostrophe and a double quote used to fail this:
//! `julia's "file".txt` printed as `"julia's "file".txt"`, which the shell
//! reads as three words.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Names a shell would mangle if they were printed bare.
const AWKWARD_NAMES: [&str; 6] = [
    r#"julia's "file".txt"#,
    "it's.txt",
    r#"say"hi".txt"#,
    "plain space.txt",
    r#"both'and" spaced.txt"#,
    "plain.txt",
];

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_quoting_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        for name in AWKWARD_NAMES {
            fs::write(path.join(name), b"").unwrap();
        }
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn listing(dir: &PathBuf, args: &[&str]) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute lez binary");
    assert!(
        output.status.success(),
        "lez {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
        .collect()
}

/// `sh -c "test -e <printed name>"`: the shell has to resolve the printed
/// form back to a file that exists.
fn shell_resolves(dir: &PathBuf, printed: &str) -> bool {
    Command::new("sh")
        .current_dir(dir)
        .arg("-c")
        .arg(format!("test -e {printed}"))
        .status()
        .expect("Failed to run sh")
        .success()
}

fn assert_every_name_round_trips(args: &[&str]) {
    let dir = TempDir::new("roundtrip");
    let lines = listing(&dir.path, args);
    assert_eq!(
        lines.len(),
        AWKWARD_NAMES.len(),
        "expected one line per file, got {lines:?}"
    );

    for line in &lines {
        assert!(
            shell_resolves(&dir.path, line),
            "sh cannot resolve {line} back to a file (lez {args:?})"
        );
    }
}

#[test]
fn printed_names_survive_the_shell_under_auto() {
    assert_every_name_round_trips(&["-1", "--color=never"]);
}

#[test]
fn printed_names_survive_the_shell_under_always() {
    assert_every_name_round_trips(&["-1", "--color=never", "--quotes=always"]);
}

/// The exact form `ls` prints, so the fix is pinned to a known-good reference
/// rather than only to "some form the shell happens to accept".
#[test]
fn a_name_with_both_quotes_matches_what_ls_prints() {
    let dir = TempDir::new("gnu_form");
    let lines = listing(&dir.path, &["-1", "--color=never"]);

    assert!(
        lines.contains(&r#"'julia'\''s "file".txt'"#.to_owned()),
        "expected the ls form, got {lines:?}"
    );
}

/// `--quotes=never` is an explicit request for the bare name, and stays that
/// way — the shell cannot read it back, which is the point of the flag.
#[test]
fn never_still_prints_the_bare_name() {
    let dir = TempDir::new("never");
    let lines = listing(&dir.path, &["-1", "--color=never", "--quotes=never"]);

    assert!(
        lines.contains(&r#"julia's "file".txt"#.to_owned()),
        "expected the unquoted name, got {lines:?}"
    );
}
