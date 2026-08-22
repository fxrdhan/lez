// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "lsr_mime_test_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_mime_types_cli_flag_png_icon() {
    let temp = TempTestDir::new("png_icon");
    let png_no_ext = temp.path.join("image_without_extension");
    // Standard PNG magic bytes
    fs::write(
        &png_no_ext,
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    // 1. Without --mime-types: should not detect image icon
    let output_without = Command::new(lsr_bin)
        .args(["--icons=always", png_no_ext.to_str().unwrap()])
        .output()
        .expect("run lsr without --mime-types");
    assert!(output_without.status.success());
    let stdout_without = String::from_utf8_lossy(&output_without.stdout);
    // Image icon is \u{f1c5} () or \u{f03e}
    assert!(
        !stdout_without.contains('\u{f1c5}'),
        "Without --mime-types, image icon should NOT be shown: {stdout_without}"
    );

    // 2. With --mime-types: should detect image/png icon (\u{f1c5})
    let output_with = Command::new(lsr_bin)
        .args([
            "--icons=always",
            "--mime-types",
            png_no_ext.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with --mime-types");
    assert!(output_with.status.success());
    let stdout_with = String::from_utf8_lossy(&output_with.stdout);
    assert!(
        stdout_with.contains('\u{f1c5}'),
        "With --mime-types, image icon MUST be shown: {stdout_with}"
    );
}

#[test]
fn test_mime_types_lsr_env_var() {
    let temp = TempTestDir::new("lsr_env");
    let png_file = temp.path.join("png_sample");
    fs::write(
        &png_file,
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .env("LSR_MIME_TYPES", "1")
        .args(["--icons=always", png_file.to_str().unwrap()])
        .output()
        .expect("run lsr with LSR_MIME_TYPES=1");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\u{f1c5}'),
        "With LSR_MIME_TYPES=1, image icon MUST be shown: {stdout}"
    );
}

#[test]
fn test_mime_types_eza_env_var() {
    let temp = TempTestDir::new("eza_env");
    let png_file = temp.path.join("png_sample");
    fs::write(
        &png_file,
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .env_remove("LSR_MIME_TYPES")
        .env("EZA_MIME_TYPES", "1")
        .args(["--icons=always", png_file.to_str().unwrap()])
        .output()
        .expect("run lsr with EZA_MIME_TYPES=1");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\u{f1c5}'),
        "With EZA_MIME_TYPES=1, image icon MUST be shown: {stdout}"
    );
}

#[test]
fn test_mime_types_directory_not_sniffed() {
    let temp = TempTestDir::new("dir_not_sniffed");
    let subdir = temp.path.join("subfolder");
    fs::create_dir_all(&subdir).unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .args([
            "--icons=always",
            "--mime-types",
            "-d",
            subdir.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with --mime-types on directory");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Folder icon is \u{e5fe} () or \u{f115} ()
    assert!(
        stdout.contains('\u{e5fe}') || stdout.contains('\u{f115}'),
        "Directory must retain folder icon: {stdout}"
    );
}

#[test]
fn test_mime_types_gzip_archive() {
    let temp = TempTestDir::new("gzip_archive");
    let gz_no_ext = temp.path.join("compressed_stream");
    // GZIP header: \x1f\x8b\x08
    fs::write(
        &gz_no_ext,
        b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03\x03\x00\x00\x00\x00\x00\x00\x00\x00\x00",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .args([
            "--icons=always",
            "--mime-types",
            gz_no_ext.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with --mime-types on gzip");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\u{f410}') || stdout.contains('\u{f1c6}'),
        "Gzip archive without extension must show COMPRESSED icon: {stdout}"
    );
}

#[test]
fn test_mime_types_python_script() {
    let temp = TempTestDir::new("python_script");
    let py_no_ext = temp.path.join("script_runner");
    fs::write(
        &py_no_ext,
        b"#!/usr/bin/env python3\nimport sys\nprint('hello')\n",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .args([
            "--icons=always",
            "--mime-types",
            py_no_ext.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with --mime-types on python script");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Python icon is \u{e73c} () or \u{e606} ()
    assert!(
        stdout.contains('\u{e73c}') || stdout.contains('\u{e606}'),
        "Python script without extension must show PYTHON icon: {stdout}"
    );
}

#[test]
fn test_mime_types_c_source() {
    let temp = TempTestDir::new("c_source");
    let c_no_ext = temp.path.join("c_program");
    fs::write(
        &c_no_ext,
        b"#include <stdio.h>\n\nint main(void) {\n    printf(\"Hello, world!\\n\");\n    return 0;\n}\n",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .args(["--icons=always", "--mime-types", c_no_ext.to_str().unwrap()])
        .output()
        .expect("run lsr with --mime-types on c source");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // C icon is \u{e61e} () or \u{e649}
    assert!(
        stdout.contains('\u{e61e}') || stdout.contains('\u{e649}'),
        "C source without extension must show C icon: {stdout}"
    );
}

#[test]
fn test_mime_types_gif_wildcard_fallback() {
    let temp = TempTestDir::new("gif_wildcard");
    let gif_no_ext = temp.path.join("sample_gif");
    fs::write(
        &gif_no_ext,
        b"GIF89a\x01\x00\x01\x00\x80\x00\x00\xff\xff\xff\x00\x00\x00!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .args([
            "--icons=always",
            "--mime-types",
            gif_no_ext.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with --mime-types on gif");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Image wildcard icon is \u{f1c5} ()
    assert!(
        stdout.contains('\u{f1c5}'),
        "GIF image must match image/* wildcard icon: {stdout}"
    );
}

#[test]
fn test_mime_types_theme_yaml_override() {
    let temp = TempTestDir::new("theme_override");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // Create theme.yml with custom mimetypes override for image/png
    let theme_content = r#"
mimetypes:
  image/png:
    filename:
      foreground: Magenta
    icon:
      glyph: "🖼️"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let png_file = temp.path.join("photo_file");
    fs::write(
        &png_file,
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4",
    )
    .unwrap();

    let lsr_bin = env!("CARGO_BIN_EXE_lsr");

    let output = Command::new(lsr_bin)
        .env("LSR_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--mime-types",
            "--color=always",
            png_file.to_str().unwrap(),
        ])
        .output()
        .expect("run lsr with theme.yml mimetypes override");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Custom glyph 🖼️ should be present
    assert!(
        stdout.contains('🖼'),
        "Custom theme.yml glyph 🖼️ must be rendered for image/png: {stdout}"
    );
}
