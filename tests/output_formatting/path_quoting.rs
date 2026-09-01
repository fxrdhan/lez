// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use crate::common::{TempTestDir, bin_path};
use std::path::MAIN_SEPARATOR;
use std::process::Command;

#[test]
fn test_full_path_with_spaces_quoted_as_single_token() {
    let temp = TempTestDir::new("full_path_quoting");
    temp.create_dir("parent with space");
    temp.create_file("parent with space/child with space.txt", b"content");

    let rel_path = format!("parent with space{MAIN_SEPARATOR}child with space.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("'parent with space{MAIN_SEPARATOR}child with space.txt'"),
        "Full path should be quoted as a single cohesive token"
    );
}

#[test]
fn test_path_without_spaces_not_quoted() {
    let temp = TempTestDir::new("path_without_spaces");
    temp.create_dir("parent_dir");
    temp.create_file("parent_dir/child_file.txt", b"content");

    let rel_path = format!("parent_dir{MAIN_SEPARATOR}child_file.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("parent_dir{MAIN_SEPARATOR}child_file.txt"),
        "Path without spaces should not have quotes"
    );
}

#[test]
fn test_path_with_space_in_parent_only_quoted() {
    let temp = TempTestDir::new("parent_space_only");
    temp.create_dir("parent space");
    temp.create_file("parent space/child.txt", b"content");

    let rel_path = format!("parent space{MAIN_SEPARATOR}child.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("'parent space{MAIN_SEPARATOR}child.txt'"),
        "Path with space in parent component should be quoted as single token"
    );
}

#[test]
fn test_path_with_space_in_child_only_quoted() {
    let temp = TempTestDir::new("child_space_only");
    temp.create_dir("parent");
    temp.create_file("parent/child space.txt", b"content");

    let rel_path = format!("parent{MAIN_SEPARATOR}child space.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("'parent{MAIN_SEPARATOR}child space.txt'"),
        "Path with space in child component should be quoted as single token"
    );
}

#[test]
fn test_quote_style_override_qu() {
    let temp = TempTestDir::new("quote_style_override");
    temp.create_dir("dir a");
    temp.create_file("dir a/file b.txt", b"content");

    let rel_path = format!("dir a{MAIN_SEPARATOR}file b.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .env("LEZ_COLORS", "qu=35;1")
        .arg("-1")
        .arg("--color=always")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Opening quote with bold magenta (1;35m)
    assert!(
        stdout.contains("\x1b[1;35m'\x1b[0m") || stdout.contains("\x1b[1;35m'"),
        "Quote should be styled with qu custom color code: {stdout}"
    );
}

#[test]
fn test_no_quotes_flag_suppresses_quotes_on_paths() {
    let temp = TempTestDir::new("no_quotes_suppress");
    temp.create_dir("parent with space");
    temp.create_file("parent with space/child with space.txt", b"content");

    let rel_path = format!("parent with space{MAIN_SEPARATOR}child with space.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg("--no-quotes")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("parent with space{MAIN_SEPARATOR}child with space.txt"),
        "--no-quotes should suppress quotes even on paths with spaces"
    );
}

#[test]
fn test_quotes_always_quotes_paths_even_without_spaces() {
    let temp = TempTestDir::new("quotes_always");
    temp.create_dir("parent");
    temp.create_file("parent/child.txt", b"content");

    let rel_path = format!("parent{MAIN_SEPARATOR}child.txt");
    let output = Command::new(bin_path())
        .current_dir(temp.path())
        .arg("-1")
        .arg("--color=never")
        .arg("--quotes=always")
        .arg(&rel_path)
        .output()
        .expect("failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        stdout.trim(),
        format!("'parent{MAIN_SEPARATOR}child.txt'"),
        "--quotes=always should quote path even without spaces"
    );
}
