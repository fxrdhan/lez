// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--stdin` and `--stdin0`: paths read from standard input. `--stdin`
//! splits on the separator `LEZ_STDIN_SEPARATOR` (or, when that is unset or
//! empty, `EZA_STDIN_SEPARATOR`) names, a newline by default; `--stdin0`
//! always splits on NUL. Without either flag stdin is not read at all.
//!
//! The fixture holds `a.txt` and `b.txt`. A separator that lez failed to
//! honour would leave the input as one path naming neither, which exits 2,
//! so a listing of exactly the two files shows the split happened.

use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process::{Output, Stdio};

use crate::common::{TempTestDir, lez_in};

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("a.txt", b"a");
    dir.create_file("b.txt", b"b");
    dir
}

/// Runs lez in `dir` with `input` on stdin. Without a stdin flag lez may
/// exit before reading, so a broken pipe while writing is not an error.
fn with_stdin(dir: &TempTestDir, env: &[(&str, &str)], args: &[&str], input: &[u8]) -> Output {
    let mut cmd = lez_in(dir.path());
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in env {
        cmd.env(name, value);
    }
    let mut child = cmd.spawn().expect("spawn lez");
    if let Err(error) = child.stdin.take().expect("a stdin pipe").write_all(input) {
        assert_eq!(error.kind(), ErrorKind::BrokenPipe, "{error}");
    }
    child.wait_with_output().expect("wait for lez")
}

/// The stdout of a run that succeeded with nothing on stderr.
fn listed(output: Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    String::from_utf8(output.stdout).expect("UTF-8 stdout")
}

const BOTH: &str = "a.txt\nb.txt\n";

#[test]
fn without_the_flag_stdin_is_not_read() {
    let dir = fixture("unread");
    let input = b"missing_1.txt\nmissing_2.txt\n";
    assert_eq!(listed(with_stdin(&dir, &[], &["-1"], input)), BOTH);
    assert_eq!(
        listed(with_stdin(&dir, &[], &["-1", "b.txt"], input)),
        "b.txt\n"
    );
}

#[test]
fn the_flag_lists_the_paths_read_one_per_line() {
    let dir = fixture("lines");
    dir.create_file("left_out.txt", b"x");
    let stdin = |input: &[u8]| listed(with_stdin(&dir, &[], &["--stdin", "-1"], input));

    assert_eq!(stdin(b"a.txt\nb.txt\n"), BOTH);
    assert_eq!(stdin(b"a.txt\r\nb.txt\r\n"), BOTH);
    assert_eq!(stdin(b""), "");
    // Paths on the command line are listed with those read.
    assert_eq!(
        listed(with_stdin(
            &dir,
            &[],
            &["--stdin", "-1", "left_out.txt"],
            b"b.txt\n"
        )),
        "b.txt\nleft_out.txt\n"
    );
}

/// The variables to set, and what to write to stdin.
type Case = (Vec<(&'static str, &'static str)>, &'static [u8]);

/// Each way of spelling a separator, and the input it splits into the two
/// files.
#[test]
fn every_separator_spelling_splits_the_input() {
    let dir = fixture("separators");
    let nul = b"a.txt\0b.txt\0";
    let mut cases: Vec<Case> = vec![
        (vec![("LEZ_STDIN_SEPARATOR", ",")], b"a.txt,b.txt"),
        (vec![("EZA_STDIN_SEPARATOR", ";")], b"a.txt;b.txt"),
        // LEZ_ wins over EZA_ ...
        (
            vec![("LEZ_STDIN_SEPARATOR", ":"), ("EZA_STDIN_SEPARATOR", ";")],
            b"a.txt:b.txt",
        ),
        // ... unless it is empty, as `""` is.
        (
            vec![("LEZ_STDIN_SEPARATOR", ""), ("EZA_STDIN_SEPARATOR", ",")],
            b"a.txt,b.txt",
        ),
        (
            vec![
                ("LEZ_STDIN_SEPARATOR", "\"\""),
                ("EZA_STDIN_SEPARATOR", ","),
            ],
            b"a.txt,b.txt",
        ),
        // An empty separator is the default newline.
        (vec![("LEZ_STDIN_SEPARATOR", "")], b"a.txt\nb.txt\n"),
        (vec![("LEZ_STDIN_SEPARATOR", "\"\"")], b"a.txt\nb.txt\n"),
        (vec![("LEZ_STDIN_SEPARATOR", r"\t")], b"a.txt\tb.txt\t"),
        (vec![("LEZ_STDIN_SEPARATOR", r"\n")], b"a.txt\nb.txt\n"),
        (
            vec![("LEZ_STDIN_SEPARATOR", "🔥")],
            "a.txt🔥b.txt".as_bytes(),
        ),
        (
            vec![("LEZ_STDIN_SEPARATOR", r"\u{1f525}")],
            "a.txt🔥b.txt".as_bytes(),
        ),
        // Runs of separators leave empty entries, which are skipped.
        (
            vec![("LEZ_STDIN_SEPARATOR", r"\0")],
            b"a.txt\0\0\0b.txt\0\0",
        ),
        (vec![("LEZ_STDIN_SEPARATOR", ",")], b",,a.txt,,b.txt,"),
    ];
    for nul_spelling in [
        r"\0",
        r"\x00",
        r"\x0",
        r"\X00",
        r"\X0",
        r"\u0000",
        r"\u{0}",
        r"\u{0000}",
        r"\U00000000",
        "null",
        "NUL",
        "NULL",
        "  null  ",
        "  NUL  ",
        r#""\0""#,
        r#"'\0'"#,
        r#""\x00""#,
        r#""null""#,
    ] {
        cases.push((vec![("LEZ_STDIN_SEPARATOR", nul_spelling)], nul));
    }

    for (env, input) in cases {
        assert_eq!(
            listed(with_stdin(&dir, &env, &["--stdin", "-1"], input)),
            BOTH,
            "{env:?}"
        );
    }
}

/// With a separator set, a newline is part of a path like any other byte.
#[test]
fn a_set_separator_replaces_the_newline() {
    let dir = fixture("not_newline");
    let output = with_stdin(
        &dir,
        &[("LEZ_STDIN_SEPARATOR", ",")],
        &["--stdin", "-1"],
        b"a.txt\nb.txt",
    );
    let not_found =
        std::fs::symlink_metadata(dir.path().join("a.txt\nb.txt")).expect_err("no such file");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("{:?}: {not_found}\n", Path::new("a.txt\nb.txt"))
    );
}

/// Input that is not UTF-8 is taken as raw path bytes, not rejected; here
/// they name no file.
#[test]
#[cfg(unix)]
fn input_that_is_not_utf8_is_a_path_like_any_other() {
    use std::os::unix::ffi::OsStrExt;

    let dir = fixture("raw_bytes");
    let bytes = [0xFF, 0xFE, 0xFD];
    let output = with_stdin(&dir, &[], &["--stdin"], &bytes);
    let path = Path::new(std::ffi::OsStr::from_bytes(&bytes));
    let not_found = std::fs::symlink_metadata(dir.path().join(path)).expect_err("no such file");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("{path:?}: {not_found}\n")
    );
}

#[test]
fn stdin0_splits_on_nul_whatever_the_separator() {
    let dir = fixture("stdin0");
    let stdin0 = |env: &[(&str, &str)], input: &[u8]| {
        listed(with_stdin(&dir, env, &["--stdin0", "-1"], input))
    };
    assert_eq!(stdin0(&[], b"a.txt\0b.txt\0"), BOTH);
    assert_eq!(stdin0(&[], b"\0\0a.txt\0\0b.txt"), BOTH);
    assert_eq!(
        stdin0(&[("LEZ_STDIN_SEPARATOR", ",")], b"a.txt\0b.txt\0"),
        BOTH
    );
}

/// Which is what makes it safe for names holding a newline.
#[test]
#[cfg(unix)]
fn stdin0_keeps_a_newline_inside_a_name() {
    let dir = TempTestDir::new("stdin0_newline");
    dir.create_file("a\nb", b"x");
    assert_eq!(
        listed(with_stdin(&dir, &[], &["--stdin0", "-1"], b"a\nb\0")),
        "a\\nb\n"
    );
}
