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

/// Deterministic PRNG generating 64 adversarial printable filenames (combining
/// quotes, command substitution, globs, spaces, backslashes, leading dashes,
/// `#`, `~`, and Unicode) and verifying byte-for-byte lossless roundtrip
/// through POSIX `sh` under both `--quotes=auto` and `--quotes=always`.
#[test]
fn randomized_adversarial_names_roundtrip_byte_for_byte_through_sh() {
    use std::collections::BTreeSet;

    const ALPHABET: &[char] = &[
        '\'', '"', '`', '$', ';', '&', '|', '*', '?', '[', ']', '(', ')', '{', '}', '\\', '#', '~',
        '!', '=', '<', '>', ' ', '-', 'a', 'Z', '0', '日', 'é',
    ];

    let mut state: u64 = 0x5EED_CAFE_BABE_1234;
    let mut next_u64 = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut generated = BTreeSet::new();
    // Explicit high-risk shell-injection / expansion probes.
    for probe in [
        "$(echo pwned)",
        "`uname`",
        "$HOME$PATH",
        "''\"\"''",
        "-rf",
        "--help",
        "~root",
        "#comment",
        "a\\b\\c\"d'e",
        "日本語 'quoted' \"$HOME\"",
    ] {
        generated.insert(probe.to_owned());
    }
    while generated.len() < 64 {
        let len = 3 + (next_u64() % 14) as usize;
        let mut s = String::with_capacity(len + 4);
        for _ in 0..len {
            let idx = (next_u64() as usize) % ALPHABET.len();
            s.push(ALPHABET[idx]);
        }
        // Avoid `.` / `..` or leading `.` (hidden) so every entry is listed without `-a`.
        if s.starts_with('.') {
            s.insert(0, 'x');
        }
        generated.insert(s);
    }

    let dir = crate::common::TempTestDir::new("quoting_prop");
    for name in &generated {
        dir.create_file(name, b"");
    }

    for args in [
        &["-1", "--color=never"][..],
        &["-1", "--color=never", "--quotes=always"],
    ] {
        let lines = listing(&dir.path().to_path_buf(), args);
        assert_eq!(
            lines.len(),
            generated.len(),
            "expected {} lines for {args:?}, got {}",
            generated.len(),
            lines.len()
        );

        let mut recovered = BTreeSet::new();
        for printed in &lines {
            let out = Command::new("sh")
                .current_dir(dir.path())
                .env("HOME", "/SHOULD_NOT_EXPAND")
                .arg("-c")
                .arg(format!("printf -- '%s' {printed}"))
                .output()
                .expect("run sh");
            assert!(
                out.status.success(),
                "sh failed on {printed} ({args:?}): {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let value = String::from_utf8(out.stdout).expect("UTF-8 roundtrip");
            recovered.insert(value);
        }
        assert_eq!(
            recovered, generated,
            "shell roundtrip diverged from disk filenames for {args:?}"
        );
    }
}
