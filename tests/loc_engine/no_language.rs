// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--loc` adds a language column beside the counts, in each of its modes;
//! `--no-language`, or `language = false` under `[loc]` or `[display]` in
//! the config file, leaves it out and keeps the counts.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// 3 lines of Rust and 1 of Python, so the shares are 75% and 25%.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("no_language");
    dir.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    dir.create_file("script.py", b"print('test')\n");
    dir
}

fn columns(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .args([
                "-lh",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
                "main.rs",
                "script.py",
            ])
            .args(args),
    )
}

#[test]
fn no_language_drops_the_column_in_every_mode() {
    let dir = fixture();
    for (mode, with, without) in [
        (
            "--loc",
            "Language Code Code % Name\nRust        3  75.0% main.rs\nPython      1  25.0% script.py\n",
            "Code Code % Name\n   3  75.0% main.rs\n   1  25.0% script.py\n",
        ),
        (
            "--loc=both",
            "Language Code Code % Name\nRust        3  75.0% main.rs\nPython      1  25.0% script.py\n",
            "Code Code % Name\n   3  75.0% main.rs\n   1  25.0% script.py\n",
        ),
        (
            "--loc=lines",
            "Language Code Name\nRust        3 main.rs\nPython      1 script.py\n",
            "Code Name\n   3 main.rs\n   1 script.py\n",
        ),
        (
            "--loc=percent",
            "Language Code % Name\nRust      75.0% main.rs\nPython    25.0% script.py\n",
            "Code % Name\n 75.0% main.rs\n 25.0% script.py\n",
        ),
    ] {
        assert_eq!(columns(&dir, &[mode]), with, "{mode}");
        assert_eq!(columns(&dir, &[mode, "--no-language"]), without, "{mode}");
    }
}

#[test]
fn the_config_file_can_drop_the_column() {
    let dir = fixture();
    let config = TempTestDir::new("no_language_config");
    let without = "Code Code % Name\n   3  75.0% main.rs\n   1  25.0% script.py\n";
    for (name, contents) in [
        ("loc.toml", "[loc]\nlanguage = false\n"),
        ("display.toml", "[display]\nlanguage = false\n"),
    ] {
        let path = config.create_file(name, contents.as_bytes());
        let path = path.to_str().expect("UTF-8 path");
        assert_eq!(
            columns(&dir, &["--loc", "--config", path]),
            without,
            "{contents}"
        );
    }
}
