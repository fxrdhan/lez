// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Broken and unreachable symlink targets (#141): the `mi` colour for the
//! missing path, absolute paths for broken links in the working directory,
//! and targets behind a directory the user cannot search.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, exit_and_stdout, lez_in};

fn long_name_column(dir: &Path, args: &[&str], colours: &str) -> (Option<i32>, String) {
    exit_and_stdout(
        lez_in(dir)
            .args(NAME_COLUMN_ONLY)
            .args(args)
            .env("LEZ_COLORS", colours),
    )
}

#[test]
fn mi_colours_the_missing_path() {
    let tmp = TempTestDir::new("broken_mi");
    tmp.create_symlink("missing", "link");

    let (code, stdout) = long_name_column(tmp.path(), &["-d", "--color=always", "link"], "mi=33");
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "\x1b[36mlink\x1b[0m \x1b[31m->\x1b[0m \x1b[4;33mmissing\x1b[0m\n"
    );
}

#[test]
fn mi_replaces_the_orphan_style_instead_of_layering_on_it() {
    let tmp = TempTestDir::new("broken_mi_or");
    tmp.create_symlink("missing", "link");

    let (code, stdout) = long_name_column(
        tmp.path(),
        &["-d", "--color=always", "link"],
        "or=1;31:mi=33",
    );
    assert_eq!(code, Some(0));
    // The arrow keeps `or`; the missing path is `mi` plus the broken-path
    // underline, without the bold that `or` carries.
    assert_eq!(
        stdout,
        "\x1b[36mlink\x1b[0m \x1b[1;31m->\x1b[0m \x1b[4;33mmissing\x1b[0m\n"
    );
}

#[test]
fn mi_is_read_from_ls_colors_too() {
    let tmp = TempTestDir::new("broken_mi_ls");
    tmp.create_symlink("missing", "link");

    let (code, stdout) = exit_and_stdout(
        lez_in(tmp.path())
            .args(NAME_COLUMN_ONLY)
            .args(["-d", "--color=always", "link"])
            .env("LS_COLORS", "mi=35"),
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "\x1b[36mlink\x1b[0m \x1b[31m->\x1b[0m \x1b[4;35mmissing\x1b[0m\n"
    );
}

#[test]
fn missing_path_falls_back_to_the_orphan_colour() {
    let tmp = TempTestDir::new("broken_or_only");
    tmp.create_symlink("missing", "link");

    let (code, stdout) = long_name_column(tmp.path(), &["-d", "--color=always", "link"], "or=32");
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        "\x1b[36mlink\x1b[0m \x1b[32m->\x1b[0m \x1b[4;32mmissing\x1b[0m\n"
    );
}

#[test]
fn broken_link_in_working_directory_gets_an_absolute_path() {
    let tmp = TempTestDir::new("broken_cwd_abs");
    tmp.create_symlink("missing", "link");
    let expected = fs::canonicalize(tmp.path()).unwrap().join("link");

    let (code, stdout) =
        exit_and_stdout(lez_in(tmp.path()).args(["-1", "--absolute=follow", "link"]));
    assert_eq!(code, Some(0));
    assert_eq!(stdout, format!("{}\n", expected.display()));
}

#[test]
fn broken_link_in_working_directory_gets_a_hyperlink() {
    let tmp = TempTestDir::new("broken_cwd_link");
    tmp.create_symlink("missing", "link");
    let expected = fs::canonicalize(tmp.path()).unwrap().join("link");

    let (code, stdout) =
        exit_and_stdout(lez_in(tmp.path()).args(["-1", "--hyperlink=always", "link"]));
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout,
        format!(
            "\x1b]8;;file://{}\x1b\\link\x1b]8;;\x1b\\\n",
            expected.display()
        )
    );
}

/// A target behind a directory the user cannot search is shown the way `ls`
/// shows it: the link and its target text, painted as broken, exiting 0.
/// Only reading the link itself failing is an error.
#[test]
fn unsearchable_target_is_listed_like_a_broken_link() {
    if unsafe { libc::geteuid() } == 0 {
        return; // root ignores the permission bits this test relies on
    }
    let tmp = TempTestDir::new("broken_eacces");
    let locked = tmp.create_dir("locked");
    tmp.create_file("locked/secret.txt", b"x");
    tmp.create_symlink("locked/secret.txt", "link");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let plain = long_name_column(tmp.path(), &["-d", "--color=never", "link"], "");
    let missing = long_name_column(
        tmp.path(),
        &["-d", "--color=never", "link", "does-not-exist"],
        "",
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(plain, (Some(0), "link -> locked/secret.txt\n".to_owned()));
    // A missing argument still decides the exit code.
    assert_eq!(missing.0, Some(2));
    assert_eq!(missing.1, "link -> locked/secret.txt\n");
}
