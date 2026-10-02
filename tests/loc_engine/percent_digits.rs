// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--percent-digits` (alias `--precision-percent`), `LEZ_PERCENT_DIGITS`
//! and `percent_digits` under `[loc]` in the config file set the decimals of
//! a code share, 1 by default, in `--code` and in the `--loc` column alike.
//! Out-of-range values are refused in `cli_options/exit_codes.rs`.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// 3 lines of Rust and 1 of Python: 75% and 25%.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    dir.create_file("script.py", b"print('test')\n");
    dir
}

fn shares(dir: &TempTestDir, envs: &[(&str, &str)], args: &[&str]) -> String {
    let mut cmd = lez_in(dir.path());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    success_stdout(
        cmd.args([
            "-l",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
            "--loc=percent",
            "--no-language",
            "main.rs",
            "script.py",
        ])
        .args(args),
    )
}

fn rows(rust: &str, python: &str) -> String {
    let width = rust.len().max(python.len());
    format!("{rust:>width$} main.rs\n{python:>width$} script.py\n")
}

#[test]
fn the_flag_sets_the_decimals_of_each_share() {
    let dir = fixture("percent_flag");
    for (args, expected) in [
        (&[][..], rows("75.0%", "25.0%")),
        (&["--percent-digits=0"], rows("75%", "25%")),
        (&["--percent-digits=3"], rows("75.000%", "25.000%")),
        (&["--precision-percent=2"], rows("75.00%", "25.00%")),
        (
            &["--percent-digits=8"],
            rows("75.00000000%", "25.00000000%"),
        ),
    ] {
        assert_eq!(shares(&dir, &[], args), expected, "{args:?}");
    }
}

/// The flag beats the variables, `LEZ_` beats `EZA_` beats `EXA_`, and any
/// of them beats the config file, which clamps what it is given to 8.
#[test]
fn the_flag_then_the_environment_then_the_config_file() {
    let dir = fixture("percent_precedence");
    let two = rows("75.00%", "25.00%");
    let three = rows("75.000%", "25.000%");
    for (envs, args, expected) in [
        (&[("LEZ_PERCENT_DIGITS", "2")][..], &[][..], &two),
        (&[("EZA_PERCENT_DIGITS", "2")], &[], &two),
        (&[("EXA_PERCENT_DIGITS", "2")], &[], &two),
        (
            &[("LEZ_PERCENT_DIGITS", "2"), ("EZA_PERCENT_DIGITS", "3")],
            &[],
            &two,
        ),
        (
            &[("EZA_PERCENT_DIGITS", "2"), ("EXA_PERCENT_DIGITS", "3")],
            &[],
            &two,
        ),
        (
            &[("LEZ_PERCENT_DIGITS", "2")],
            &["--percent-digits=3"],
            &three,
        ),
    ] {
        assert_eq!(shares(&dir, envs, args), *expected, "{envs:?} {args:?}");
    }

    // The config file in the working directory.
    std::fs::write(dir.path().join(".lez.toml"), "[loc]\npercent_digits = 3\n")
        .expect("write the config");
    assert_eq!(shares(&dir, &[], &[]), three);
    assert_eq!(shares(&dir, &[("LEZ_PERCENT_DIGITS", "2")], &[]), two);
    std::fs::write(dir.path().join(".lez.toml"), "[loc]\npercent_digits = 99\n")
        .expect("write the config");
    assert_eq!(shares(&dir, &[], &[]), rows("75.00000000%", "25.00000000%"));
}

/// `--code` takes the same setting for every share, the total's included,
/// and widens its column to fit.
#[test]
fn code_takes_the_same_decimals() {
    let dir = fixture("percent_code");
    let code = |digits: &str| {
        success_stdout(lez_in(dir.path()).args([
            "--code",
            &format!("--percent-digits={digits}"),
            "main.rs",
            "script.py",
        ]))
    };
    assert_eq!(
        code("0"),
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20Rust          1      3     3         0       0     75%  ████████████████\n\
             \x20Python        1      1     1         0       0     25%  █████▍\n\
             {}\n\
             \x20Total         2      4     4         0       0    100%\n",
            "─".repeat(73)
        )
    );
    assert_eq!(
        code("3"),
        format!(
            " Language  Files  Lines  Code  Comments  Blanks    Code %\n\
             \x20Rust          1      3     3         0       0   75.000%  ████████████████\n\
             \x20Python        1      1     1         0       0   25.000%  █████▍\n\
             {}\n\
             \x20Total         2      4     4         0       0  100.000%\n",
            "─".repeat(75)
        )
    );
}
