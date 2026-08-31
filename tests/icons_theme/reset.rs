// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_theme_reset_lez_colors_plain() {
    let temp_dir = std::env::temp_dir().join("lez_test_theme_reset_plain");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("sample.txt");
    StdFile::create(&test_file).unwrap();

    // With LEZ_COLORS="reset", no ANSI escapes should color the metadata or filename
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg(&test_file)
        .env("LEZ_COLORS", "reset")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez with LEZ_COLORS=reset");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // In plain mode, there should be no color sequences (like \x1b[31m, \x1b[32m, \x1b[1;34m, etc.)
    // Note: \x1b[0m reset or no escape sequences at all
    assert!(
        !stdout.contains("\x1b[3"),
        "stdout should not contain standard foreground color escapes with LEZ_COLORS=reset, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[1;3"),
        "stdout should not contain bold color escapes with LEZ_COLORS=reset, got: {stdout:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_theme_reset_eza_colors_plain() {
    let temp_dir = std::env::temp_dir().join("lez_test_theme_reset_eza_plain");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("sample.txt");
    StdFile::create(&test_file).unwrap();

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg(&test_file)
        .env_remove("LEZ_COLORS")
        .env("EZA_COLORS", "reset")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez with EZA_COLORS=reset");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b[3"),
        "stdout should not contain standard foreground color escapes with EZA_COLORS=reset, got: {stdout:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_theme_reset_with_override_date() {
    let temp_dir = std::env::temp_dir().join("lez_test_theme_reset_override");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("sample.txt");
    StdFile::create(&test_file).unwrap();

    // With LEZ_COLORS="reset:da=32", date should be green (\x1b[32m), but permissions should be plain
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg(&test_file)
        .env("LEZ_COLORS", "reset:da=32")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez with LEZ_COLORS=reset:da=32");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("\x1b[32m") || stdout.contains("\x1b[0;32m"),
        "stdout should contain green date escape code, got: {stdout:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_theme_reset_with_ls_colors() {
    let temp_dir = std::env::temp_dir().join("lez_test_theme_reset_ls_colors");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("sample.txt");
    StdFile::create(&test_file).unwrap();

    // With LS_COLORS="fi=31" (red regular file) and LEZ_COLORS="reset"
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg(&test_file)
        .env("LS_COLORS", "fi=31")
        .env("LEZ_COLORS", "reset")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez with LS_COLORS=fi=31 and LEZ_COLORS=reset");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Regular file name should be red
    assert!(
        stdout.contains("\x1b[31m") || stdout.contains("\x1b[0;31m"),
        "stdout should contain red file style from LS_COLORS, got: {stdout:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_theme_reset_lez_colors_precedence_over_eza_colors() {
    let temp_dir = std::env::temp_dir().join("lez_test_theme_precedence");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("sample.txt");
    StdFile::create(&test_file).unwrap();

    // LEZ_COLORS sets date green (32), EZA_COLORS sets date red (31)
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg(&test_file)
        .env("LEZ_COLORS", "reset:da=32")
        .env("EZA_COLORS", "reset:da=31")
        .env_remove("EXA_COLORS")
        .env_remove("LS_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez with conflicting LEZ_COLORS and EZA_COLORS");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("\x1b[32m") || stdout.contains("\x1b[0;32m"),
        "stdout should contain green date escape from LEZ_COLORS precedence, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[31m") && !stdout.contains("\x1b[0;31m"),
        "stdout should not contain red date escape from EZA_COLORS, got: {stdout:?}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
