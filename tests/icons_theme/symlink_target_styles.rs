// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `ln=target` in `LEZ_COLORS`, and `symlink: target` in a theme: a link
//! takes the colour of what it leads to, with its own attributes (italic,
//! bold, underline) on top. A link that leads nowhere keeps the `or`
//! colour, attributes and all left off.

#![cfg(unix)]

use std::path::Path;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Links of every kind, beside what they lead to: a directory, a `.rs`
/// file, an executable, a plain file, a chain of two links, a dangling link
/// and a loop.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_dir("my_target_dir");
    dir.create_file("main.rs", b"fn main() {}\n");
    let run = dir.create_file("run.sh", b"#!/bin/sh\n");
    std::fs::set_permissions(&run, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("chmod");
    dir.create_file("doc_data", b"text\n");
    for (target, link) in [
        ("my_target_dir", "link_to_dir"),
        ("main.rs", "link_to_rs"),
        ("run.sh", "link_to_exec"),
        ("doc_data", "link_to_doc"),
        ("link_b", "link_a"),
        ("my_target_dir", "link_b"),
        ("/nonexistent_path", "broken_orphan_link"),
        ("loop_link_2", "loop_link_1"),
        ("loop_link_1", "loop_link_2"),
    ] {
        dir.create_symlink(target, link);
        pin_link_mode(&dir.path().join(link));
    }
    dir
}

/// Gives a link the mode Linux gives every link, so the long view reads
/// `lrwxrwxrwx` everywhere; macOS otherwise takes it from the umask.
fn pin_link_mode(link: &Path) {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(link.as_os_str().as_bytes()).expect("no NUL");
        // SAFETY: a valid NUL-terminated path, relative to nothing.
        let result = unsafe {
            libc::fchmodat(
                libc::AT_FDCWD,
                path.as_ptr(),
                0o777,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        assert_eq!(result, 0, "fchmodat: {}", std::io::Error::last_os_error());
    }
    #[cfg(not(target_os = "macos"))]
    let _ = link;
}

/// Lists `args` in colour with `LEZ_COLORS` set to `colours` and, when
/// given, a theme file holding `theme`.
fn styled(dir: &TempTestDir, colours: &str, theme: Option<&str>, args: &[&str]) -> String {
    let config = dir.path().join(".config");
    let mut cmd = lez_in(dir.path());
    if let Some(theme) = theme {
        std::fs::create_dir_all(&config).expect("create the config directory");
        std::fs::write(config.join("theme.yml"), theme).expect("write the theme");
        cmd.env("LEZ_CONFIG_DIR", &config);
    }
    if !colours.is_empty() {
        cmd.env("LEZ_COLORS", colours);
    }
    success_stdout(cmd.arg("--color=always").args(args))
}

const BLUE_DIRECTORIES: &str =
    "filekinds:\n  directory:\n    foreground: Blue\n    is_bold: false\n";

#[test]
fn ln_target_borrows_the_targets_colour_and_adds_its_own_attributes() {
    let dir = fixture("lez_colors");
    for (colours, link, expected) in [
        ("di=34:ln=target;3", "link_to_dir", "\x1b[3;34m"),
        ("di=34:ln=target;1;4", "link_to_dir", "\x1b[1;4;34m"),
        ("*.rs=35:ln=target;3", "link_to_rs", "\x1b[3;35m"),
        ("ex=32:ln=target;4", "link_to_exec", "\x1b[4;32m"),
    ] {
        assert_eq!(
            styled(&dir, colours, None, &["-d", link]),
            format!("{expected}{link}\x1b[0m\n"),
            "{colours}"
        );
    }
}

/// The same from a theme: `foreground: target`, the bare string `target`,
/// or `target;3`; and a theme's attributes combined with `ln=target` from
/// the environment.
#[test]
fn a_theme_can_ask_for_the_targets_colour() {
    let dir = fixture("theme");
    let italic_blue = "\x1b[3;34mlink_to_dir\x1b[0m\n";
    for (theme, colours, expected) in [
        (
            "  symlink:\n    foreground: target\n    is_italic: true\n",
            "",
            italic_blue,
        ),
        ("  symlink: \"target;3\"\n", "", italic_blue),
        ("  symlink: target\n", "", "\x1b[34mlink_to_dir\x1b[0m\n"),
        (
            "  symlink:\n    is_italic: true\n",
            "ln=target",
            italic_blue,
        ),
    ] {
        let theme = format!("{BLUE_DIRECTORIES}{theme}");
        assert_eq!(
            styled(&dir, colours, Some(&theme), &["-d", "link_to_dir"]),
            expected,
            "{theme}"
        );
    }
}

/// Every hop of a chain takes the colour of where the chain ends.
#[test]
fn a_chain_takes_the_colour_of_its_end() {
    let dir = fixture("chain");
    assert_eq!(
        styled(
            &dir,
            "di=34:ln=target;3",
            None,
            &[
                "-ld",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
                "link_a"
            ]
        ),
        "\x1b[3;34mlink_a\x1b[0m \x1b[1;90m->\x1b[0m \x1b[3;34mlink_b\x1b[0m\n"
    );
}

/// In the long view the `l` has no target to borrow a colour from, so it
/// keeps only the link's attributes, whatever the link leads to; it used to
/// take the plain-file colour, and read as a file's `.`. The target after
/// the arrow is painted as itself, without them.
#[test]
fn the_long_view_styles_the_type_character_and_the_target() {
    let dir = fixture("long");
    let permissions = "\x1b[1;33mr\x1b[31mw\x1b[32mx\x1b[0m\
                       \x1b[33mr\x1b[31mw\x1b[32mx\
                       \x1b[33mr\x1b[31mw\x1b[32mx\x1b[0m";
    let long = |link: &str| {
        styled(
            &dir,
            "fi=33:di=34:ln=target;3",
            None,
            &["-ld", "--no-filesize", "--no-user", "--no-time", link],
        )
    };
    assert_eq!(
        long("link_to_doc"),
        format!(
            "\x1b[3ml\x1b[0m{permissions} \x1b[3;33mlink_to_doc\x1b[0m \x1b[1;90m->\x1b[0m \x1b[33mdoc_data\x1b[0m\n"
        )
    );
    assert_eq!(
        long("link_to_dir"),
        format!(
            "\x1b[3ml\x1b[0m{permissions} \x1b[3;34mlink_to_dir\x1b[0m \x1b[1;90m->\x1b[0m \x1b[34mmy_target_dir\x1b[0m\n"
        )
    );
}

/// A dangling link and a loop keep the `or` colour, without the italic
/// `ln=target;3` or the theme would add.
#[test]
fn a_link_that_leads_nowhere_keeps_the_orphan_colour() {
    let dir = fixture("orphans");
    let theme = "filekinds:\n  symlink:\n    foreground: target\n    is_italic: true\n";
    assert_eq!(
        styled(&dir, "or=31", Some(theme), &["-d", "broken_orphan_link"]),
        "\x1b[31mbroken_orphan_link\x1b[0m\n"
    );
    assert_eq!(
        styled(
            &dir,
            "or=31:ln=target;3",
            None,
            &[
                "-ld",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
                "loop_link_1"
            ]
        ),
        "\x1b[31mloop_link_1\x1b[0m \x1b[1;90m->\x1b[0m \x1b[31mloop_link_2\x1b[0m\n"
    );
}
