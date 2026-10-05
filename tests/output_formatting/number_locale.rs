// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Sizes are written the locale's way: its separator between groups of
//! digits, and the one before a fraction. The C library says what those
//! are, read here through `locale -k`; a locale the machine does not have
//! is left out, and `C` it always has. macOS used to group digits the
//! English way under `C`, which groups none.

#![cfg(unix)]

use std::process::Command;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// `decimal_point` and `thousands_sep` in `locale`, or `None` where the
/// system does not have it. `LOCPATH` is passed on, so locales built with
/// `localedef` can be tried too.
///
/// `locale` itself is looked for on `PATH` first, then in `/usr/bin` and
/// `/bin`, the only places a cleared environment would search. NixOS and
/// the Nix build sandbox keep it in neither.
fn separators(locale: &str) -> Option<(String, String)> {
    let mut cmd = Command::new("locale");
    cmd.env_clear()
        .env("LC_ALL", locale)
        .args(["-k", "decimal_point", "thousands_sep"]);
    if let Some(path) = std::env::var_os("LOCPATH") {
        cmd.env("LOCPATH", path);
    }
    let search = match std::env::var_os("PATH") {
        Some(mut path) => {
            path.push(":/usr/bin:/bin");
            path
        }
        None => "/usr/bin:/bin".into(),
    };
    cmd.env("PATH", search);
    let output = cmd.output().expect("run locale");
    if !output.status.success() || !output.stderr.is_empty() {
        return None;
    }
    let text = String::from_utf8(output.stdout).expect("UTF-8 separators");
    let value = |key: &str| {
        text.lines()
            .find_map(|line| {
                line.strip_prefix(key)?
                    .strip_prefix("=\"")?
                    .strip_suffix('"')
            })
            .map(str::to_owned)
    };
    Some((value("decimal_point")?, value("thousands_sep")?))
}

#[test]
fn sizes_are_written_the_locales_way() {
    let dir = TempTestDir::new("number_locale");
    dir.create_file("big", &[0; 15_003]);
    dir.create_file("small", &[0; 1_500]);

    for locale in ["C", "POSIX", "de_DE.UTF-8", "fr_FR.UTF-8", "en_US.UTF-8"] {
        let Some((decimal, thousands)) = separators(locale) else {
            eprintln!("skipped {locale}: this system does not have it");
            continue;
        };
        let size = |args: &[&str]| {
            let mut cmd = lez_in(dir.path());
            cmd.env("LC_ALL", locale);
            if let Some(path) = std::env::var_os("LOCPATH") {
                cmd.env("LOCPATH", path);
            }
            success_stdout(
                cmd.args(["-l", "--no-permissions", "--no-user", "--no-time"])
                    .args(args),
            )
        };
        assert_eq!(
            size(&["-B", "big"]),
            format!("15{thousands}003 big\n"),
            "{locale}"
        );
        assert_eq!(
            size(&["small"]),
            format!("1{decimal}5k small\n"),
            "{locale}"
        );
    }
}
