// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--explain` lists each entry with the rule that chose the colour of its
//! name and the one that chose its icon (eza#1954). The rules are the
//! listing's own, so the first test holds every explanation to what a
//! listing prints for the same entry.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Entries for most of the rules: globs from both variables, built-in file
/// types, a directory, a file no rule names, and, on Unix, an executable
/// and links.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("explain");
    for file in [
        "main.rs",
        "notes.txt",
        "Makefile",
        "photo.JPG",
        "data.weird",
        "noext",
    ] {
        dir.create_file(file, b"");
    }
    dir.create_dir("sub");
    dir.create_file(
        ".config/theme.yml",
        b"filenames:\n  Makefile: {filename: {foreground: Red}, icon: {glyph: M}}\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let run = dir.create_file("run.sh", b"");
        std::fs::set_permissions(run, std::fs::Permissions::from_mode(0o755)).unwrap();
        dir.create_symlink("main.rs", "link.rs");
    }
    dir
}

const VARS: [(&str, &str); 2] = [("LS_COLORS", "di=32:*.txt=35"), ("LEZ_COLORS", "*.JPG=33")];

fn run(dir: &TempTestDir, args: &[&str], vars: &[(&str, &str)]) -> String {
    let mut cmd = lez_in(dir.path());
    cmd.args(args);
    for (var, value) in vars {
        cmd.env(var, value);
    }
    success_stdout(&mut cmd)
}

/// The colour codes and the name, from a line `lez -1 --color=always`
/// prints, such as `\x1b[1;34msub\x1b[0m`.
fn painted(line: &str) -> (String, String) {
    match line.strip_prefix("\x1b[") {
        Some(rest) => {
            let (codes, rest) = rest.split_once('m').unwrap();
            let name = rest.split('\x1b').next().unwrap();
            (codes.to_owned(), name.to_owned())
        }
        None => ("none".to_owned(), line.to_owned()),
    }
}

/// Each entry `--explain` lists: its name, the colour codes it explains,
/// the glyph, and the whole text.
fn explained(output: &str) -> Vec<(String, String, String, String)> {
    let lines: Vec<&str> = output.lines().collect();
    lines
        .chunks(3)
        .map(|entry| {
            let colour = entry[1].trim().strip_prefix("colour: ").unwrap();
            let icon = entry[2].trim().strip_prefix("icon:").unwrap().trim();
            (
                entry[0].to_owned(),
                colour.split(',').next().unwrap().to_owned(),
                icon.split(' ').next().unwrap().to_owned(),
                entry[1..].join("\n"),
            )
        })
        .collect()
}

#[test]
fn every_explanation_matches_what_the_listing_prints() {
    let dir = fixture();
    for theme in [false, true] {
        let mut vars = VARS.to_vec();
        let config = dir.path().join(".config");
        let config = config.to_str().unwrap();
        if theme {
            vars.push(("LEZ_CONFIG_DIR", config));
        }

        let explanation = explained(&run(&dir, &["--explain"], &vars));
        let listing = run(&dir, &["-1", "--color=always"], &vars);
        let icons = run(&dir, &["-1", "--icons=always"], &vars);
        assert_eq!(explanation.len(), listing.lines().count());

        for ((name, codes, glyph, text), (line, icon_line)) in
            explanation.iter().zip(listing.lines().zip(icons.lines()))
        {
            let (listed_codes, listed_name) = painted(line);
            assert_eq!(&listed_name, name, "theme: {theme}");
            assert_eq!(&listed_codes, codes, "{name}, theme: {theme}\n{text}");
            assert!(
                icon_line.starts_with(glyph.as_str()),
                "{name}, theme: {theme}: {icon_line:?} against\n{text}"
            );
        }
    }
}

#[test]
fn each_rule_is_named() {
    let dir = fixture();
    let output = run(&dir, &["--explain"], &VARS);
    let text = |name: &str| {
        explained(&output)
            .into_iter()
            .find(|entry| entry.0 == name)
            .unwrap()
            .3
    };

    assert!(text("notes.txt").contains("colour: 35, from the glob `*.txt` in LS_COLORS"));
    assert!(text("photo.JPG").contains("colour: 33, from the glob `*.JPG` in LEZ_COLORS"));
    assert!(text("main.rs").contains("from `sc`, the built-in file type for source code files"));
    assert!(text("main.rs").contains("from the built-in icon for the extension `rs`"));
    assert!(text("Makefile").contains("from the built-in icon for files named `Makefile`"));
    assert!(text("sub").contains("colour: 32, from `di`, a directory"));
    assert!(text("data.weird").contains("colour: none, from `fi`, a regular file"));
    #[cfg(unix)]
    {
        assert!(text("run.sh").contains("from `ex`, an executable file"));
        assert!(text("link.rs").contains("from `ln`, a symlink"));
    }
}

#[test]
fn theme_entries_and_the_globs_laid_over_them_are_named() {
    let dir = fixture();
    let config = dir.path().join(".config");
    let output = run(
        &dir,
        &["--explain", "Makefile"],
        &[
            ("LEZ_CONFIG_DIR", config.to_str().unwrap()),
            ("EZA_COLORS", "Makefile=36"),
        ],
    );
    assert_eq!(
        output,
        "Makefile\n    \
         colour: 36, from the glob `Makefile` in EZA_COLORS, laid over the theme file's `filenames` entry `Makefile`\n    \
         icon:   M (U+004D), from the theme file's `filenames` entry `Makefile`, coloured 36 as the name is\n"
    );
}

/// The colours are worked out as a terminal would get them, so the
/// explanation holds through a pipe; `--color=never` still turns them off.
#[test]
fn colours_are_explained_through_a_pipe_unless_turned_off() {
    let dir = fixture();
    assert!(run(&dir, &["--explain", "-d", "sub"], &VARS).contains("colour: 32, from `di`"));
    assert!(
        run(&dir, &["--explain", "--color=never", "-d", "sub"], &VARS)
            .contains("colour: none, from nothing: colours are off")
    );
}

#[test]
fn a_layout_beside_it_is_refused_in_strict_mode() {
    let dir = fixture();
    let out = lez_in(dir.path())
        .env("LEZ_STRICT", "1")
        .args(["--explain", "-l"])
        .output()
        .expect("run lez");
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("long is useless given option explain"));
}
