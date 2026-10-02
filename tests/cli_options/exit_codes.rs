// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Explicit exit code verification suite:
//! - Exit 0: Success
//! - Exit 3: Options error / invalid flag combinations in strict mode (via LEZ_STRICT / EZA_STRICT)
//! - Exit 13 / 1: Permission denied / runtime I/O error
//! - Exit 1: Missing input paths / non-existent directory error

use std::fs::{self, File as StdFile};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_exit_code_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Restore permissions so cleanup succeeds
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_exit_code_0_on_success() {
    let temp = TempTestDir::new("success");
    fs::write(temp.path.join("file.txt"), b"test").unwrap();

    let output = crate::common::lez_cmd()
        .arg("-1")
        .arg(&temp.path)
        .output()
        .expect("run lez");

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 on success"
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_long_only_options() {
    let temp = TempTestDir::new("strict_opt_err");
    let temp_str = temp.path.to_str().unwrap();

    // In strict mode (LEZ_STRICT=1), passing long-only flags like --binary without -l triggers OptionsError (Exit 3)
    let output = crate::common::lez_cmd()
        .args(["--binary", temp_str])
        .env("LEZ_STRICT", "1")
        .output()
        .expect("run lez in strict mode with long-only option");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 (OPTIONS_ERROR) on strict option failure, got: {:?}",
        output.status.code()
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_conflicting_options() {
    let temp = TempTestDir::new("strict_conflict_err");
    let temp_str = temp.path.to_str().unwrap();

    // In strict mode (EZA_STRICT=1), passing -l with --across triggers OptionsError::Useless (Exit 3)
    let output = crate::common::lez_cmd()
        .args(["-l", "-x", temp_str])
        .env("EZA_STRICT", "1")
        .output()
        .expect("run lez with conflicting options in strict mode");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 on conflicting options in strict mode"
    );
}

#[test]
fn test_exit_code_3_on_invalid_cli_arguments() {
    let output = crate::common::lez_cmd()
        .arg("--completely-invalid-nonexistent-flag-xyz")
        .output()
        .expect("run lez with invalid option");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 (OPTIONS_ERROR) on invalid CLI arguments"
    );
}

#[test]
fn test_exit_code_on_missing_input_path() {
    let temp = TempTestDir::new("missing_path");
    let non_existent = temp.path.join("definitely_missing_subdir_12345");

    let output = crate::common::lez_cmd()
        .arg(&non_existent)
        .output()
        .expect("run lez on missing path");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (MISSING_INPUT_PATH) on missing path"
    );
}

#[test]
fn test_exit_code_on_code_mode_missing_input_path() {
    let temp = TempTestDir::new("code_missing_path");
    let non_existent = temp.path.join("definitely_missing_code_subdir_12345");

    let output = crate::common::lez_cmd()
        .arg("--code")
        .arg(&non_existent)
        .output()
        .expect("run lez --code on missing path");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (MISSING_INPUT_PATH) on missing path in --code mode"
    );
}

/// After `--` everything is a path, so a file whose name starts with a dash
/// can be listed; without it the name is read as bundled short flags, and
/// `-dash.txt` ends in `-s h.txt`, an unknown sort field.
#[test]
fn a_double_dash_ends_option_parsing() {
    let dir = crate::common::TempTestDir::new("double_dash");
    dir.create_file("-dash.txt", b"x");

    assert_eq!(
        crate::common::success_stdout(crate::common::lez_in(dir.path()).args([
            "-1",
            "--",
            "-dash.txt"
        ])),
        "-dash.txt\n"
    );

    let output = crate::common::lez_in(dir.path())
        .args(["-1", "-dash.txt"])
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with("error: invalid value 'h.txt' for '--sort <FIELD>'\n"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A whole number a digit count does not accept is out of range, in the
/// words clap uses for the flag; it used to be blamed on an invalid digit.
/// The ends of each range are accepted.
#[test]
fn a_variable_out_of_range_says_so() {
    let dir = crate::common::TempTestDir::new("range_variable");
    dir.create_file("file.rs", b"fn main() {}\n");
    let run = |name: &str, value: &str, view: &[&str]| {
        crate::common::lez_in(dir.path())
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
    let dir = crate::common::TempTestDir::new("bad_variable");
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
                let output = crate::common::lez_in(dir.path())
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
