// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--no-config` ignores the theme file; a theme file that does not parse
//! is reported, and the listing falls back to the built-in theme.

use std::path::Path;
use std::process::Output;

use crate::common::{TempTestDir, lez_in};

fn run(dir: &TempTestDir, config: &Path, extra: &[&str]) -> Output {
    lez_in(dir.path())
        .env("LEZ_CONFIG_DIR", config)
        .args(["--color=always", "-1"])
        .args(extra)
        .arg("test_sample.txt")
        .output()
        .expect("run lez")
}

/// Stdout and stderr, of a run that succeeded.
fn streams(output: Output) -> (String, String) {
    assert_eq!(output.status.code(), Some(0));
    (
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        String::from_utf8(output.stderr).expect("UTF-8 stderr"),
    )
}

const BUILT_IN: &str = "\x1b[32mtest_sample.txt\x1b[0m\n";

#[test]
fn no_config_ignores_the_theme_file() {
    let dir = TempTestDir::new("no_config");
    dir.create_file("test_sample.txt", b"");
    dir.create_file(
        "config/theme.yml",
        b"filenames:\n  test_sample.txt:\n    filename:\n      foreground: Red\n",
    );
    let config = dir.path().join("config");

    assert_eq!(
        streams(run(&dir, &config, &[])),
        ("\x1b[31mtest_sample.txt\x1b[0m\n".to_owned(), String::new())
    );
    assert_eq!(
        streams(run(&dir, &config, &["--no-config"])),
        (BUILT_IN.to_owned(), String::new())
    );
}

#[test]
fn a_theme_that_does_not_parse_is_reported_and_ignored() {
    let dir = TempTestDir::new("corrupt_theme");
    dir.create_file("test_sample.txt", b"");
    dir.create_file(
        "config/theme.yml",
        b"[[[ this is definitely corrupted yaml : {\n",
    );
    // Joined a part at a time, as lez joins it, so the separators match on
    // Windows.
    let theme = dir.path().join("config").join("theme.yml");

    assert_eq!(
        streams(run(&dir, &dir.path().join("config"), &[])),
        (
            BUILT_IN.to_owned(),
            format!(
                "lez: Failed to parse theme file {theme:?}: \
                 invalid type: sequence, expected struct UiStylesOverride\n"
            )
        )
    );
}
