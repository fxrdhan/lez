// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--size-digits` (alias `--digits`), `LEZ_SIZE_DIGITS` and `size_digits`
//! in the config file set how many characters a size's number takes, the
//! decimal point included: the default 3 gives `2.3M`. How each size is
//! rounded is unit tested in `src/output/render/size.rs`; these runs check
//! the setting reaches the listing from each place, which place wins, and
//! what is refused.

use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

/// Three files whose sizes round differently: 2 345 678, 999 and
/// 2 451 000 bytes, made sparse so nothing is written.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for (name, size) in [("a.bin", 2_345_678), ("b.bin", 999), ("c.bin", 2_451_000)] {
        std::fs::File::create(dir.path().join(name))
            .and_then(|file| file.set_len(size))
            .expect("create a sparse file");
    }
    dir
}

/// The size column and the names, with `envs` set and `args` given.
fn sizes(dir: &TempTestDir, envs: &[(&str, &str)], args: &[&str]) -> String {
    let mut cmd = lez_in(dir.path());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    success_stdout(
        cmd.args(["-l", "--no-permissions", "--no-user", "--no-time"])
            .args(args),
    )
}

/// The rows for the three files with these sizes, right-aligned.
fn rows(a: &str, b: &str, c: &str) -> String {
    let width = [a, b, c].iter().map(|s| s.len()).max().unwrap_or(0);
    format!("{a:>width$} a.bin\n{b:>width$} b.bin\n{c:>width$} c.bin\n")
}

#[test]
fn the_flag_sets_the_width_of_each_size() {
    let dir = fixture("widths");
    for (args, expected) in [
        (&[][..], rows("2.3M", "999", "2.5M")),
        (&["--size-digits=1"], rows("2M", "999", "2M")),
        (&["--size-digits=2"], rows("2M", "999", "2M")),
        (&["--size-digits=4"], rows("2.35M", "999", "2.45M")),
        (&["--digits=5"], rows("2.346M", "999", "2.451M")),
        (&["--size-digits=8"], rows("2.345678M", "999", "2.451000M")),
        // The last one given wins, whichever spelling.
        (
            &["--size-digits=4", "--digits=5"],
            rows("2.346M", "999", "2.451M"),
        ),
        (&["-b", "--size-digits=4"], rows("2.24Mi", "999", "2.34Mi")),
        // Bytes are whole, so there is nothing to round.
        (
            &["-B", "--size-digits=1"],
            rows(&grouped(2_345_678), &grouped(999), &grouped(2_451_000)),
        ),
    ] {
        assert_eq!(sizes(&dir, &[], args), expected, "{args:?}");
    }
}

/// The flag beats the variables, `LEZ_` beats `EZA_` beats `EXA_`, and any
/// of them beats the config file, which clamps what it is given to 1..=8.
#[test]
fn the_flag_then_the_environment_then_the_config_file() {
    let dir = fixture("precedence");
    let config_holder = TempTestDir::new("size_config");
    let config = config_holder.path();
    let config_dir = config.to_str().expect("UTF-8 path");
    let four = rows("2.35M", "999", "2.45M");
    let five = rows("2.346M", "999", "2.451M");
    let six = rows("2.3457M", "999", "2.4510M");
    for (env, args, expected) in [
        (&[("LEZ_SIZE_DIGITS", "4")][..], &[][..], &four),
        (&[("EZA_SIZE_DIGITS", "4")], &[], &four),
        (&[("EXA_SIZE_DIGITS", "4")], &[], &four),
        (
            &[("LEZ_SIZE_DIGITS", "4"), ("EZA_SIZE_DIGITS", "5")],
            &[],
            &four,
        ),
        (
            &[("EZA_SIZE_DIGITS", "4"), ("EXA_SIZE_DIGITS", "5")],
            &[],
            &four,
        ),
        (&[("LEZ_SIZE_DIGITS", "4")], &["--size-digits=5"], &five),
    ] {
        assert_eq!(sizes(&dir, env, args), *expected, "{env:?} {args:?}");
    }

    for (setting, expected) in [
        ("6", six.clone()),
        ("0", rows("2M", "999", "2M")),
        ("99", rows("2.345678M", "999", "2.451000M")),
    ] {
        std::fs::write(
            config.join("config.toml"),
            format!("[display]\nsize_digits = {setting}\n"),
        )
        .expect("write the config");
        let from_config = [("LEZ_CONFIG_DIR", config_dir)];
        assert_eq!(sizes(&dir, &from_config, &[]), expected, "{setting}");
    }
    let from_config = [("LEZ_CONFIG_DIR", config_dir)];
    let both = [("LEZ_CONFIG_DIR", config_dir), ("LEZ_SIZE_DIGITS", "4")];
    std::fs::write(config.join("config.toml"), "[display]\nsize_digits = 6\n")
        .expect("write the config");
    assert_eq!(sizes(&dir, &from_config, &[]), six);
    assert_eq!(sizes(&dir, &both, &[]), four);
    assert_eq!(sizes(&dir, &from_config, &["--size-digits=5"]), five);
}

/// JSON carries the size as the listing prints it.
#[test]
fn json_carries_the_rounded_size() {
    let dir = fixture("json");
    let json: serde_json::Value = serde_json::from_str(&success_stdout(lez_in(dir.path()).args([
        "--json",
        "-l",
        "--no-permissions",
        "--no-user",
        "--no-time",
        "--size-digits=4",
    ])))
    .expect("JSON");
    assert_eq!(
        json,
        serde_json::json!({
            "a.bin": {"Size": "2.35M"},
            "b.bin": {"Size": "999"},
            "c.bin": {"Size": "2.45M"},
        })
    );
}

/// Outside 1..=8 the flag is refused by clap, before anything is listed.
#[test]
fn a_width_outside_the_range_is_refused() {
    let dir = fixture("refused");
    for (value, reason) in [
        ("0", "0 is not in 1..=8"),
        ("9", "9 is not in 1..=8"),
        ("x", "invalid digit found in string"),
    ] {
        let output = lez_in(dir.path())
            .args(["-l", &format!("--size-digits={value}")])
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(3), "{value}");
        assert!(output.stdout.is_empty(), "{value}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "error: invalid value '{value}' for '--size-digits <NUM>': {reason}\n\n\
                 For more information, try '--help'.\n"
            ),
            "{value}"
        );
    }
}
