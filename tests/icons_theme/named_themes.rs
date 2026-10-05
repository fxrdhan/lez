// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A theme can be picked by name from the `themes` folder of the
//! configuration directory, with `--theme`, `LEZ_THEME` or `EZA_THEME`, or
//! `name` under `[theme]` in the configuration file, in that order, so
//! switching themes no longer means rewriting `theme.yml` (eza#1945).

use crate::common::{TempTestDir, lez_in, success_stdout};

fn directory_in(colour: &str) -> String {
    format!("filekinds:\n  directory: {{foreground: {colour}}}\n")
}

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("named_themes");
    dir.create_dir("sub");
    dir.create_file(".config/theme.yml", directory_in("Red").as_bytes());
    dir.create_file(".config/themes/night.yml", directory_in("Green").as_bytes());
    dir.create_file(".config/themes/day.yaml", directory_in("Yellow").as_bytes());
    dir
}

fn sub(dir: &TempTestDir, args: &[&str], vars: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = lez_in(dir.path());
    cmd.env("LEZ_CONFIG_DIR", dir.path().join(".config"))
        .args(["-1d", "--color=always", "sub"])
        .args(args);
    for (var, value) in vars {
        cmd.env(var, value);
    }
    cmd.output().expect("run lez")
}

fn coloured(code: u8) -> String {
    format!("\x1b[1;{code}msub\x1b[0m\n")
}

#[test]
fn a_named_theme_replaces_theme_yml() {
    let dir = fixture();
    let listed = |args: &[&str], vars: &[(&str, &str)]| {
        String::from_utf8(sub(&dir, args, vars).stdout).unwrap()
    };
    assert_eq!(listed(&[], &[]), coloured(31));
    assert_eq!(listed(&[], &[("LEZ_THEME", "night")]), coloured(32));
    assert_eq!(listed(&[], &[("EZA_THEME", "day")]), coloured(33));
    assert_eq!(
        listed(&["--theme=day"], &[("LEZ_THEME", "night")]),
        coloured(33)
    );
}

#[test]
fn the_configuration_file_can_name_the_theme() {
    let dir = fixture();
    dir.create_file(".config/config.toml", b"[theme]\nname = \"night\"\n");
    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_CONFIG_DIR", dir.path().join(".config"))
                .args(["-1d", "--color=always", "sub"])
        ),
        coloured(32)
    );
}

#[test]
fn a_theme_that_is_not_there_is_an_error() {
    let dir = fixture();
    let out = sub(&dir, &["--theme=dusk"], &[]);
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("Theme \"dusk\" not found") && stderr.contains("dusk.yml"),
        "{stderr}"
    );
}
