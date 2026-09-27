// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Symlinks that point at other symlinks (#138): each hop is resolved from
//! the directory of the link before it, `ln=target` takes the colour of the
//! end of the chain, and loops or over-long chains end as broken links
//! instead of recursing forever.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, exit_and_stdout, lez_in};

fn coloured(dir: &Path, args: &[&str], colours: &str) -> String {
    let (code, stdout) = exit_and_stdout(
        lez_in(dir)
            .args(args)
            .arg("--color=always")
            .env("LEZ_COLORS", colours),
    );
    assert_eq!(code, Some(0), "stdout: {stdout:?}");
    stdout
}

/// Creates `hop1 -> hop2 -> … -> hop{count} -> end` in `tmp`.
fn chain(tmp: &TempTestDir, count: usize, end: &str) {
    for n in 1..=count {
        let next = if n == count {
            end.to_owned()
        } else {
            format!("hop{}", n + 1)
        };
        tmp.create_symlink(&next, &format!("hop{n}"));
    }
}

#[test]
fn ln_target_takes_the_colour_of_the_end_of_the_chain() {
    let tmp = TempTestDir::new("chain_ln_target");
    tmp.create_file("doc.pdf", b"%PDF");
    chain(&tmp, 2, "doc.pdf");

    assert_eq!(
        coloured(tmp.path(), &["-1", "hop1"], "ln=target:*.pdf=35"),
        "\x1b[35mhop1\x1b[0m\n"
    );
    assert_eq!(
        coloured(tmp.path(), &["-1", "hop1"], "ln=target;3:*.pdf=35"),
        "\x1b[3;35mhop1\x1b[0m\n"
    );
}

#[test]
fn ln_target_resolves_chains_from_the_links_directory() {
    let tmp = TempTestDir::new("chain_ln_target_subdir");
    tmp.create_file("doc.pdf", b"%PDF");
    tmp.create_symlink("../doc.pdf", "sub/first");
    tmp.create_symlink("first", "sub/second");

    let stdout = coloured(tmp.path(), &["-1", "sub/second"], "ln=target:*.pdf=35");
    assert!(
        stdout.ends_with("\x1b[35msecond\x1b[0m\n"),
        "got {stdout:?}"
    );
}

#[test]
fn intermediate_link_after_the_arrow_is_painted_as_a_link() {
    let tmp = TempTestDir::new("chain_arrow_link");
    tmp.create_file("doc.pdf", b"%PDF");
    chain(&tmp, 2, "doc.pdf");

    let mut args = NAME_COLUMN_ONLY.to_vec();
    args.extend(["-d", "hop1"]);
    assert_eq!(
        coloured(tmp.path(), &args, "ln=36:*.pdf=35"),
        "\x1b[36mhop1\x1b[0m \x1b[1;90m->\x1b[0m \x1b[36mhop2\x1b[0m\n"
    );
    // With `ln=target`, the intermediate link borrows the end's colour too.
    assert_eq!(
        coloured(tmp.path(), &args, "ln=target:*.pdf=35"),
        "\x1b[35mhop1\x1b[0m \x1b[1;90m->\x1b[0m \x1b[35mhop2\x1b[0m\n"
    );
}

#[test]
fn looping_chains_are_painted_as_broken() {
    let tmp = TempTestDir::new("chain_loop");
    tmp.create_symlink("loop_b", "loop_a");
    tmp.create_symlink("loop_a", "loop_b");
    tmp.create_symlink("itself", "itself");

    assert_eq!(
        coloured(tmp.path(), &["-1", "loop_a", "itself"], "ln=target:or=31"),
        "\x1b[31mitself\x1b[0m\n\x1b[31mloop_a\x1b[0m\n"
    );
}

#[test]
fn chains_longer_than_the_hop_limit_are_broken() {
    let tmp = TempTestDir::new("chain_limit");
    tmp.create_file("doc.pdf", b"%PDF");
    chain(&tmp, 33, "doc.pdf");

    // hop2 starts 32 links from the file, hop1 starts 33.
    assert_eq!(
        coloured(tmp.path(), &["-1", "hop2"], "ln=target:or=31:*.pdf=35"),
        "\x1b[35mhop2\x1b[0m\n"
    );
    assert_eq!(
        coloured(tmp.path(), &["-1", "hop1"], "ln=target:or=31:*.pdf=35"),
        "\x1b[31mhop1\x1b[0m\n"
    );
}

#[test]
fn dereference_describes_the_end_of_a_chain_in_a_subdirectory() {
    let tmp = TempTestDir::new("chain_deref_subdir");
    let file = tmp.create_file("sub/final", b"x");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    tmp.create_symlink("final", "sub/second");
    tmp.create_symlink("second", "sub/first");

    let (code, stdout) = exit_and_stdout(lez_in(tmp.path()).args([
        "-ldX",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--color=never",
        "sub/first",
    ]));
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with(".rw-r-----"), "got {stdout:?}");
    assert!(stdout.ends_with(" sub/first\n"), "got {stdout:?}");
}

#[test]
fn dereferencing_a_loop_falls_back_to_the_link() {
    let tmp = TempTestDir::new("chain_deref_loop");
    tmp.create_symlink("loop_b", "loop_a");
    tmp.create_symlink("loop_a", "loop_b");

    let (code, stdout) = exit_and_stdout(lez_in(tmp.path()).args([
        "-ldX",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--color=never",
        "loop_a",
    ]));
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with('l'), "got {stdout:?}");
}

#[test]
fn only_dirs_follows_chains_to_directories() {
    let tmp = TempTestDir::new("chain_only_dirs");
    tmp.create_dir("dir");
    chain(&tmp, 3, "dir");
    tmp.create_symlink("loop_b", "loop_a");
    tmp.create_symlink("loop_a", "loop_b");

    let (code, stdout) =
        exit_and_stdout(lez_in(tmp.path()).args(["-1D", "--show-symlinks", "--color=never"]));
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "dir\nhop1\nhop2\nhop3\n");
}
