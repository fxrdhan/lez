// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Exit codes, each with what is printed beside it:
//! - 0: success
//! - 2: a path named on the command line does not exist
//! - 3: an unknown option, or options strict mode refuses
//!
//! A denied path (13) is covered in `os_metadata/permissions_exit.rs` and
//! `adversarial/io_error_isolation.rs`.

use std::process::Output;

use crate::common::{TempTestDir, lez_cmd, lez_in, success_stdout};

fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("UTF-8 output")
}

/// The exit code, standard output and standard error of a run.
fn outcome(output: &Output) -> (Option<i32>, &str, &str) {
    (
        output.status.code(),
        text(&output.stdout),
        text(&output.stderr),
    )
}

#[test]
fn test_exit_code_0_on_success() {
    let dir = TempTestDir::new("success");
    dir.create_file("file.txt", b"test");

    assert_eq!(
        success_stdout(lez_cmd().arg("-1").arg(dir.path())),
        "file.txt\n"
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_long_only_options() {
    let dir = TempTestDir::new("strict_opt_err");
    let output = lez_cmd()
        .args(["--binary"])
        .arg(dir.path())
        .env("LEZ_STRICT", "1")
        .output()
        .expect("run lez");

    assert_eq!(
        outcome(&output),
        (
            Some(3),
            "",
            "lez: Option binary is useless without option long\n"
        )
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_conflicting_options() {
    let dir = TempTestDir::new("strict_conflict_err");
    let output = lez_cmd()
        .args(["-l", "-x"])
        .arg(dir.path())
        .env("EZA_STRICT", "1")
        .output()
        .expect("run lez");

    assert_eq!(
        outcome(&output),
        (
            Some(3),
            "",
            "lez: Option across is useless given option long\n"
        )
    );
}

#[test]
fn test_exit_code_3_on_invalid_cli_arguments() {
    let output = lez_cmd()
        .arg("--completely-invalid-nonexistent-flag-xyz")
        .output()
        .expect("run lez");

    assert_eq!(
        outcome(&output),
        (
            Some(3),
            "",
            "error: unexpected argument '--completely-invalid-nonexistent-flag-xyz' found\n\n  \
             tip: to pass '--completely-invalid-nonexistent-flag-xyz' as a value, use \
             '-- --completely-invalid-nonexistent-flag-xyz'\n\n\
             Usage: lez [OPTIONS] [FILE]...\n\n\
             For more information, try '--help'.\n"
        )
    );
}

#[test]
fn test_exit_code_on_missing_input_path() {
    let dir = TempTestDir::new("missing_path");
    let missing = dir.path().join("definitely_missing_subdir_12345");
    let not_found = std::fs::metadata(&missing).expect_err("missing");

    for view in [&[][..], &["--code"]] {
        let output = lez_cmd()
            .args(view)
            .arg(&missing)
            .output()
            .expect("run lez");
        assert_eq!(
            outcome(&output),
            (Some(2), "", format!("{missing:?}: {not_found}\n").as_str()),
            "{view:?}"
        );
    }
}

/// After `--` everything is a path, so a file whose name starts with a dash
/// can be listed; without it the name is read as bundled short flags, and
/// `-dash.txt` ends in `-s h.txt`, an unknown sort field.
#[test]
fn a_double_dash_ends_option_parsing() {
    let dir = TempTestDir::new("double_dash");
    dir.create_file("-dash.txt", b"x");

    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--", "-dash.txt"])),
        "-dash.txt\n"
    );

    let fields = if cfg!(unix) {
        "name, Name, .name, .Name, lexicographic, Lexicographic, path, Path, size, blocks, \
         ext, Ext, date, age, changed, accessed, created, inode, type, none"
    } else {
        "name, Name, .name, .Name, lexicographic, Lexicographic, path, Path, size, \
         ext, Ext, date, age, changed, accessed, created, type, none"
    };
    let output = lez_in(dir.path())
        .args(["-1", "-dash.txt"])
        .output()
        .expect("run lez");
    assert_eq!(
        outcome(&output),
        (
            Some(3),
            "",
            format!(
                "error: invalid value 'h.txt' for '--sort <FIELD>'\n  \
                 [possible values: {fields}]\n\n\
                 For more information, try '--help'.\n"
            )
            .as_str()
        )
    );
}

/// A whole number a digit count does not accept is out of range, in the
/// words clap uses for the flag; it used to be blamed on an invalid digit.
/// The ends of each range are accepted.
#[test]
fn a_variable_out_of_range_says_so() {
    let dir = TempTestDir::new("range_variable");
    dir.create_file("file.rs", b"fn main() {}\n");
    let run = |name: &str, value: &str, view: &[&str]| {
        lez_in(dir.path())
            .env(name, value)
            .args(view)
            .output()
            .expect("run lez")
    };

    for (names, view, range, values) in [
        (
            ["LEZ_SIZE_DIGITS", "EZA_SIZE_DIGITS", "EXA_SIZE_DIGITS"],
            &["-l"][..],
            "1..=8",
            &["0", "9", "300", "-1"][..],
        ),
        (
            [
                "LEZ_PERCENT_DIGITS",
                "EZA_PERCENT_DIGITS",
                "EXA_PERCENT_DIGITS",
            ],
            &["--code"],
            "0..=8",
            &["9", "300", "-1"],
        ),
    ] {
        for name in names {
            for value in values {
                let output = run(name, value, view);
                assert_eq!(output.status.code(), Some(3), "{name}={value}");
                assert!(output.stdout.is_empty(), "{name}={value}");
                assert_eq!(
                    String::from_utf8_lossy(&output.stderr),
                    format!(
                        "lez: Value {value:?} not valid for environment variable {name}: \
                         {value} is not in {range}\n"
                    ),
                );
            }
        }
    }

    for (name, view, value) in [
        ("LEZ_SIZE_DIGITS", &["-l"][..], "1"),
        ("LEZ_SIZE_DIGITS", &["-l"], "8"),
        ("LEZ_PERCENT_DIGITS", &["--code"], "0"),
        ("LEZ_PERCENT_DIGITS", &["--code"], "8"),
    ] {
        let output = run(name, value, view);
        assert_eq!(output.status.code(), Some(0), "{name}={value}");
        assert!(output.stderr.is_empty(), "{name}={value}");
    }
}

/// A numeric variable that does not parse is an option error naming the
/// variable it was read from. An empty `EZA_*` or `EXA_*` value used to be
/// blamed on the `LEZ_*` one, which was not even set.
#[test]
fn an_unparseable_variable_is_named_in_the_error() {
    let dir = TempTestDir::new("bad_variable");
    dir.create_file("file.rs", b"fn main() {}\n");

    for (prefixes, family, view) in [
        (["LEZ", "EZA", "EXA"], "GRID_ROWS", &["-lG"][..]),
        (
            ["LEZ", "EZA", "EXA"],
            "ICON_SPACING",
            &["--icons=always"][..],
        ),
        (["LEZ", "EZA", "EXA"], "SIZE_DIGITS", &["-l"][..]),
        (["LEZ", "EZA", "EXA"], "PERCENT_DIGITS", &["--code"][..]),
    ] {
        for prefix in prefixes {
            let name = format!("{prefix}_{family}");
            for (value, reason) in [
                ("", "cannot parse integer from empty string"),
                ("x", "invalid digit found in string"),
            ] {
                let output = lez_in(dir.path())
                    .env(&name, value)
                    .args(view)
                    .output()
                    .expect("run lez");
                assert_eq!(output.status.code(), Some(3), "{name}={value:?}");
                assert!(output.stdout.is_empty(), "{name}={value:?}");
                assert_eq!(
                    String::from_utf8_lossy(&output.stderr),
                    format!(
                        "lez: Value {value:?} not valid for environment variable {name}: {reason}\n"
                    ),
                );
            }
        }
    }
}
