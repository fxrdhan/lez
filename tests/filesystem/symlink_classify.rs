// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--classify` on symlinks (#140). In the long view the indicator describes
//! the file named after the arrow, as in GNU `ls -lF` and eza: an
//! intermediate link gets `@`, a target that already ends in a slash gets no
//! second one, and a trailing slash written into the link is kept.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, exit_and_stdout, lez_in};

fn long_names(dir: &Path, args: &[&str]) -> String {
    let (code, stdout) = exit_and_stdout(
        lez_in(dir)
            .args(NAME_COLUMN_ONLY)
            .args(["-d", "--color=never"])
            .args(args),
    );
    assert_eq!(code, Some(0), "stdout: {stdout:?}");
    stdout
}

fn executable(tmp: &TempTestDir, name: &str) {
    let file = tmp.create_file(name, b"#!/bin/sh\n");
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn indicator_goes_on_the_target_not_the_link_name() {
    let tmp = TempTestDir::new("classify_target");
    tmp.create_dir("dir");
    executable(&tmp, "run");
    tmp.create_file("plain", b"x");
    tmp.create_symlink("dir", "to_dir");
    tmp.create_symlink("run", "to_run");
    tmp.create_symlink("plain", "to_plain");
    tmp.create_symlink("missing", "broken");

    assert_eq!(
        long_names(
            tmp.path(),
            &[
                "--classify=always",
                "to_dir",
                "to_run",
                "to_plain",
                "broken"
            ]
        ),
        "broken -> missing\nto_dir -> dir/\nto_plain -> plain\nto_run -> run*\n"
    );
}

#[test]
fn intermediate_link_is_classified_as_a_link() {
    let tmp = TempTestDir::new("classify_hop");
    executable(&tmp, "run");
    tmp.create_dir("dir");
    tmp.create_symlink("run", "to_run");
    tmp.create_symlink("to_run", "via_run");
    tmp.create_symlink("dir", "to_dir");
    tmp.create_symlink("to_dir", "via_dir");

    assert_eq!(
        long_names(tmp.path(), &["--classify=always", "via_run", "via_dir"]),
        "via_dir -> to_dir@\nvia_run -> to_run@\n"
    );
}

#[test]
fn link_to_root_gets_a_single_slash() {
    let tmp = TempTestDir::new("classify_root");
    tmp.create_symlink("/", "root");

    assert_eq!(
        long_names(tmp.path(), &["--classify=always", "root"]),
        "root -> /\n"
    );
    assert_eq!(long_names(tmp.path(), &["root"]), "root -> /\n");
}

#[test]
fn trailing_slash_in_the_link_is_kept_and_not_doubled() {
    let tmp = TempTestDir::new("classify_slash");
    tmp.create_dir("a/b");
    tmp.create_symlink("a/", "short");
    tmp.create_symlink("a/b/", "nested");

    assert_eq!(
        long_names(tmp.path(), &["nested", "short"]),
        "nested -> a/b/\nshort -> a/\n"
    );
    assert_eq!(
        long_names(tmp.path(), &["--classify=always", "nested", "short"]),
        "nested -> a/b/\nshort -> a/\n"
    );
}

#[test]
fn automatic_classify_only_applies_on_a_terminal() {
    let tmp = TempTestDir::new("classify_auto");
    tmp.create_dir("dir");
    executable(&tmp, "run");
    tmp.create_symlink("dir", "to_dir");

    // Output is piped here, so neither `-F` nor `--classify=auto` adds one.
    assert_eq!(long_names(tmp.path(), &["-F", "to_dir"]), "to_dir -> dir\n");
    let (code, stdout) =
        exit_and_stdout(lez_in(tmp.path()).args(["-1", "--classify=auto", "--color=never", "run"]));
    assert_eq!((code, stdout.as_str()), (Some(0), "run\n"));
}

#[test]
fn dereference_classifies_links_by_the_end_of_the_chain() {
    let tmp = TempTestDir::new("classify_deref");
    tmp.create_dir("dir");
    executable(&tmp, "run");
    tmp.create_symlink("dir", "to_dir");
    tmp.create_symlink("run", "to_run");
    tmp.create_symlink("to_run", "via_run");
    tmp.create_symlink("missing", "broken");

    let (code, stdout) = exit_and_stdout(lez_in(tmp.path()).args([
        "-1dX",
        "--classify=always",
        "--color=never",
        "to_dir",
        "via_run",
        "broken",
    ]));
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "broken@\nto_dir/\nvia_run*\n");
}
