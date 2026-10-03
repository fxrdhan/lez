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

/// Names a shell would mangle if they were printed bare: quotes, spaces,
/// every other character a shell reads, `#` and `~` where they start a
/// word, and an apostrophe beside what double quotes would still expand.
const AWKWARD_NAMES: [&str; 26] = [
    r#"julia's "file".txt"#,
    "it's.txt",
    r#"say"hi".txt"#,
    "plain space.txt",
    r#"both'and" spaced.txt"#,
    "plain.txt",
    "amp&er.txt",
    r"back\slash.txt",
    "bang!.txt",
    "br[ack]et.txt",
    "caret^.txt",
    "dollar$HOME.txt",
    "eq=sign.txt",
    "lt<gt>.txt",
    "paren(s).txt",
    "pipe|x.txt",
    "q?.txt",
    "semi;colon.txt",
    "star*.txt",
    "tick`x`.txt",
    "#hash.txt",
    "~tilde.txt",
    "it's $HOME.txt",
    "it's `x`.txt",
    r"it's \x.txt",
    "it's !x.txt",
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
    let output = crate::common::lez_cmd()
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

/// The exact forms GNU `ls --quoting-style=shell` prints for the same
/// names, so the quoting is pinned to a known-good reference rather than
/// only to "some form the shell happens to accept". Names a shell reads as
/// they are stay bare.
#[test]
fn every_name_takes_the_form_ls_prints() {
    let dir = TempDir::new("gnu_form");
    for name in [
        "at@x.txt",
        "brace{x}.txt",
        "mid#hash.txt",
        "mid~tilde.txt",
        "pct%x.txt",
    ] {
        fs::write(dir.path.join(name), b"").unwrap();
    }
    let mut lines = listing(&dir.path, &["-1", "--color=never"]);
    lines.sort();
    let mut expected: Vec<String> = [
        r#""it's.txt""#,
        r"'#hash.txt'",
        r"'amp&er.txt'",
        r"'back\slash.txt'",
        r"'bang!.txt'",
        r#"'both'\''and" spaced.txt'"#,
        r"'br[ack]et.txt'",
        r"'caret^.txt'",
        r"'dollar$HOME.txt'",
        r"'eq=sign.txt'",
        r"'it'\''s !x.txt'",
        r"'it'\''s $HOME.txt'",
        r"'it'\''s \x.txt'",
        r"'it'\''s `x`.txt'",
        r#"'julia'\''s "file".txt'"#,
        r"'lt<gt>.txt'",
        r"'paren(s).txt'",
        r"'pipe|x.txt'",
        r"'plain space.txt'",
        r"'q?.txt'",
        r#"'say"hi".txt'"#,
        r"'semi;colon.txt'",
        r"'star*.txt'",
        r"'tick`x`.txt'",
        r"'~tilde.txt'",
        "at@x.txt",
        "brace{x}.txt",
        "mid#hash.txt",
        "mid~tilde.txt",
        "pct%x.txt",
        "plain.txt",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect();
    expected.sort();
    assert_eq!(lines, expected);
}

/// `--quotes=never` is an explicit request for the bare name, and stays that
/// way; the shell cannot read it back, which is the point of the flag.
#[test]
fn never_still_prints_the_bare_name() {
    let dir = TempDir::new("never");
    let mut lines = listing(&dir.path, &["-1", "--color=never", "--quotes=never"]);
    lines.sort();
    let mut names: Vec<String> = AWKWARD_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    names.sort();
    assert_eq!(lines, names);
}
