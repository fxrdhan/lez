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
            "lez_theme_default_icons_test_{label}_{}_{}",
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
fn test_default_file_icon_override() {
    let temp = TempTestDir::new("default_file");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
extensions:
  .default_file:
    icon:
      glyph: "📄"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let unmapped_file = temp.path.join("data.unmapped_custom_ext_xyz");
    fs::write(&unmapped_file, b"content").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            unmapped_file.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with .default_file override");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('📄'),
        "Expected .default_file glyph '📄' in output, got: {stdout}"
    );
}

#[test]
fn test_default_file_unknown_icon_override() {
    let temp = TempTestDir::new("default_file_unknown");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
extensions:
  .default_file:
    icon:
      glyph: "📄"
  .default_file_unknown:
    icon:
      glyph: "❓"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let no_ext_file = temp.path.join("extensionless_binary");
    fs::write(&no_ext_file, b"content").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            no_ext_file.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with .default_file_unknown override");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('❓'),
        "Expected .default_file_unknown glyph '❓' in output, got: {stdout}"
    );
}

#[test]
fn test_extensionless_file_fallback_to_default_file() {
    let temp = TempTestDir::new("fallback_default_file");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // Only .default_file is defined; .default_file_unknown is omitted
    let theme_content = r#"
extensions:
  .default_file:
    icon:
      glyph: "📄"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let no_ext_file = temp.path.join("extensionless_document");
    fs::write(&no_ext_file, b"content").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            no_ext_file.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with fallback to .default_file");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('📄'),
        "Expected fallback to .default_file glyph '📄' in output, got: {stdout}"
    );
}

#[test]
fn test_default_directory_icon_override() {
    let temp = TempTestDir::new("default_directory");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
extensions:
  .default_directory:
    icon:
      glyph: "📁"
  .default_directory_empty:
    icon:
      glyph: "📂"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let non_empty_dir = temp.path.join("my_non_empty_folder");
    fs::create_dir_all(&non_empty_dir).unwrap();
    fs::write(non_empty_dir.join("item.txt"), b"item").unwrap();

    let empty_dir = temp.path.join("my_empty_folder");
    fs::create_dir_all(&empty_dir).unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");

    // 1. Non-empty directory should render .default_directory (📁)
    let output_non_empty = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            "-d",
            non_empty_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with .default_directory override");
    assert!(output_non_empty.status.success());
    let stdout_non_empty = String::from_utf8_lossy(&output_non_empty.stdout);
    assert!(
        stdout_non_empty.contains('📁'),
        "Expected .default_directory glyph '📁' for non-empty dir, got: {stdout_non_empty}"
    );

    // 2. Empty directory should render .default_directory_empty (📂)
    let output_empty = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            "-d",
            empty_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with .default_directory_empty override");
    assert!(output_empty.status.success());
    let stdout_empty = String::from_utf8_lossy(&output_empty.stdout);
    assert!(
        stdout_empty.contains('📂'),
        "Expected .default_directory_empty glyph '📂' for empty dir, got: {stdout_empty}"
    );
}

#[test]
fn test_empty_directory_fallback_to_default_directory() {
    let temp = TempTestDir::new("fallback_default_directory");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // Only .default_directory is defined; .default_directory_empty is omitted
    let theme_content = r#"
extensions:
  .default_directory:
    icon:
      glyph: "📁"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let empty_dir = temp.path.join("empty_folder");
    fs::create_dir_all(&empty_dir).unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            "-d",
            empty_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run lez with fallback to .default_directory");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('📁'),
        "Expected empty directory to fall back to .default_directory '📁', got: {stdout}"
    );
}

#[test]
fn test_specific_overrides_precedence_over_defaults() {
    let temp = TempTestDir::new("precedence");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
filenames:
  special_file.txt:
    icon:
      glyph: "⭐"

directorynames:
  special_dir:
    icon:
      glyph: "💎"

extensions:
  rs:
    icon:
      glyph: "🦀"
  .default_file:
    icon:
      glyph: "📄"
  .default_directory:
    icon:
      glyph: "📁"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let special_file = temp.path.join("special_file.txt");
    fs::write(&special_file, b"content").unwrap();

    let rust_file = temp.path.join("main.rs");
    fs::write(&rust_file, b"fn main() {}").unwrap();

    let special_dir = temp.path.join("special_dir");
    fs::create_dir_all(&special_dir).unwrap();
    fs::write(special_dir.join("sub.txt"), b"sub").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");

    // Specific filename matches ⭐ instead of 📄
    let out_file = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            special_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_file.stdout).contains('⭐'));

    // Specific extension matches 🦀 instead of 📄
    let out_rs = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            rust_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_rs.stdout).contains('🦀'));

    // Specific directoryname matches 💎 instead of 📁
    let out_dir = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            "-d",
            special_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_dir.stdout).contains('💎'));
}

#[test]
fn test_no_collision_with_real_default_file_extension() {
    let temp = TempTestDir::new("no_collision");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
extensions:
  default_file:
    icon:
      glyph: "🎯"
  .default_file:
    icon:
      glyph: "📄"
"#;
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    // A real file with literal extension '.default_file'
    let real_ext_file = temp.path.join("my_sample.default_file");
    fs::write(&real_ext_file, b"content").unwrap();

    // A file with some other unknown extension
    let other_ext_file = temp.path.join("other.unknown_ext_123");
    fs::write(&other_ext_file, b"content").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");

    // my_sample.default_file should match explicit extension `default_file` (🎯)
    let out_real = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            real_ext_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_real.stdout).contains('🎯'));

    // other.unknown_ext_123 should match fallback `.default_file` (📄)
    let out_other = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .args([
            "--icons=always",
            "--color=always",
            other_ext_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out_other.stdout).contains('📄'));
}
