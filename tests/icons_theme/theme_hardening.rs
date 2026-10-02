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

/// A theme file is laid over the built-in theme, so a theme that sets
/// nothing, or only styles a name that is not there, changes nothing: down
/// to the bold total of `--summary`.
#[test]
fn a_theme_that_sets_nothing_changes_nothing() {
    let dir = TempTestDir::new("idle_theme");
    dir.create_file("test_sample.txt", b"");
    dir.create_dir("docs");
    let config = dir.path().join("config");
    std::fs::create_dir(&config).expect("create the config directory");
    let listing = || {
        streams(
            lez_in(dir.path())
                .env("LEZ_CONFIG_DIR", &config)
                .args([
                    "--color=always",
                    "-1",
                    "--summary",
                    "docs",
                    "test_sample.txt",
                ])
                .output()
                .expect("run lez"),
        )
    };

    let built_in = listing();
    assert_eq!(
        built_in,
        (
            "\x1b[32mtest_sample.txt\x1b[0m\n\
             \x1b[1;34m\x1b[0m\x1b[32m0\x1b[0m \x1b[1;34mdirectories\x1b[0m, \
             \x1b[32m1\x1b[0m file, \
             \x1b[36m\x1b[0m\x1b[32m0\x1b[0m \x1b[36msymlinks\x1b[0m \
             \x1b[1;90m(\x1b[0m\x1b[1m1 total\x1b[0m\x1b[1;90m)\x1b[0m\n\
             \n\
             docs:\n\
             \x1b[1;34m\x1b[0m\x1b[32m0\x1b[0m \x1b[1;34mdirectories\x1b[0m, \
             \x1b[32m0\x1b[0m files, \
             \x1b[36m\x1b[0m\x1b[32m0\x1b[0m \x1b[36msymlinks\x1b[0m \
             \x1b[1;90m(\x1b[0m\x1b[1m0 total\x1b[0m\x1b[1;90m)\x1b[0m\n"
                .to_owned(),
            String::new()
        )
    );
    for theme in [
        "{}\n",
        "filenames:\n  absent_name:\n    filename:\n      foreground: Red\n",
    ] {
        std::fs::write(config.join("theme.yml"), theme).expect("write the theme");
        assert_eq!(listing(), built_in, "{theme}");
    }
}
