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
fn test_color_scale_max_luminance_cli_execution() {
    let temp_dir = std::env::temp_dir().join("lez_test_color_scale_max_luminance");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let f1 = temp_dir.join("small.txt");
    let f2 = temp_dir.join("large.txt");
    fs::write(&f1, vec![0u8; 100]).unwrap();
    fs::write(&f2, vec![0u8; 50000]).unwrap();

    // Test with LEZ_MAX_LUMINANCE=80 and --color-scale=all
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=all")
        .arg(&temp_dir)
        .env("LEZ_MAX_LUMINANCE", "80")
        .env_remove("EZA_MAX_LUMINANCE")
        .env_remove("EXA_MAX_LUMINANCE")
        .output()
        .expect("Failed to execute lez with LEZ_MAX_LUMINANCE=80");

    assert!(output.status.success(), "Command failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("small.txt") && stdout.contains("large.txt"),
        "stdout missing files: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_color_scale_max_luminance_eza_fallback() {
    let temp_dir = std::env::temp_dir().join("lez_test_color_scale_max_eza_fallback");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let f1 = temp_dir.join("file1.txt");
    fs::write(&f1, vec![0u8; 500]).unwrap();

    // With EZA_MAX_LUMINANCE=60 and LEZ_MAX_LUMINANCE unset
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=size")
        .arg(&temp_dir)
        .env_remove("LEZ_MAX_LUMINANCE")
        .env("EZA_MAX_LUMINANCE", "60")
        .env_remove("EXA_MAX_LUMINANCE")
        .output()
        .expect("Failed to execute lez with EZA_MAX_LUMINANCE=60");

    assert!(output.status.success(), "Command failed: {:?}", output);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_color_scale_max_luminance_exa_fallback() {
    let temp_dir = std::env::temp_dir().join("lez_test_color_scale_max_exa_fallback");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let f1 = temp_dir.join("file1.txt");
    fs::write(&f1, vec![0u8; 500]).unwrap();

    // With EXA_MAX_LUMINANCE=70 and others unset
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=size")
        .arg(&temp_dir)
        .env_remove("LEZ_MAX_LUMINANCE")
        .env_remove("EZA_MAX_LUMINANCE")
        .env("EXA_MAX_LUMINANCE", "70")
        .output()
        .expect("Failed to execute lez with EXA_MAX_LUMINANCE=70");

    assert!(output.status.success(), "Command failed: {:?}", output);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_color_scale_max_luminance_precedence() {
    let temp_dir = std::env::temp_dir().join("lez_test_color_scale_max_precedence");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let f1 = temp_dir.join("file1.txt");
    fs::write(&f1, vec![0u8; 1000]).unwrap();

    // LEZ_MAX_LUMINANCE takes precedence over EZA_MAX_LUMINANCE and EXA_MAX_LUMINANCE
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=size")
        .arg(&temp_dir)
        .env("LEZ_MAX_LUMINANCE", "75")
        .env("EZA_MAX_LUMINANCE", "50")
        .env("EXA_MAX_LUMINANCE", "30")
        .output()
        .expect("Failed to execute lez with conflicting luminance env vars");

    assert!(output.status.success(), "Command failed: {:?}", output);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_color_scale_invalid_and_out_of_bounds_luminance() {
    let temp_dir = std::env::temp_dir().join("lez_test_color_scale_invalid_lum");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let f1 = temp_dir.join("file1.txt");
    fs::write(&f1, vec![0u8; 1000]).unwrap();

    // Invalid string non-numeric
    let output_invalid = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=size")
        .arg(&temp_dir)
        .env("LEZ_MAX_LUMINANCE", "not_a_number")
        .env("LEZ_MIN_LUMINANCE", "invalid")
        .output()
        .expect("Failed to execute lez with invalid luminance strings");

    assert!(
        output_invalid.status.success(),
        "Command must succeed using default fallbacks"
    );

    // Out of range values > 100 or < -100
    let output_oor = Command::new(bin_path())
        .arg("-l")
        .arg("--color=always")
        .arg("--color-scale=size")
        .arg(&temp_dir)
        .env("LEZ_MAX_LUMINANCE", "200")
        .env("LEZ_MIN_LUMINANCE", "-300")
        .output()
        .expect("Failed to execute lez with out-of-range luminance");

    assert!(
        output_oor.status.success(),
        "Command must succeed using default fallbacks for out-of-range luminance"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
