// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::process::Command;

use crate::common::{TempTestDir, bin_path};

#[test]
fn test_loc_no_language_cli_flag() {
    let tmp = TempTestDir::new("loc_no_lang_cli");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    tmp.create_file("script.py", b"print('test')\n");

    // 1. Default --loc with header: Language and Code columns are present
    let out_default = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_default.status.success());
    let stdout_default = String::from_utf8_lossy(&out_default.stdout);
    assert!(stdout_default.contains("Language"));
    assert!(stdout_default.contains("Rust"));
    assert!(stdout_default.contains("Python"));

    // 2. --loc with --no-language: Language column is suppressed, Code column remains
    let out_no_lang = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc")
        .arg("--no-language")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_lang.status.success());
    let stdout_no_lang = String::from_utf8_lossy(&out_no_lang.stdout);
    assert!(!stdout_no_lang.contains("Language"));
    assert!(!stdout_no_lang.contains("Rust"));
    assert!(!stdout_no_lang.contains("Python"));
    assert!(stdout_no_lang.contains("Code") || stdout_no_lang.contains("Lines"));
    assert!(stdout_no_lang.contains("main.rs"));
    assert!(stdout_no_lang.contains("script.py"));
}

#[test]
fn test_loc_modes_with_no_language() {
    let tmp = TempTestDir::new("loc_modes_no_lang");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");

    // --loc=lines --no-language
    let out_lines = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc=lines")
        .arg("--no-language")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_lines.status.success());
    let stdout_lines = String::from_utf8_lossy(&out_lines.stdout);
    assert!(!stdout_lines.contains("Language"));
    assert!(!stdout_lines.contains("Rust"));
    assert!(stdout_lines.contains("3")); // 3 lines of code

    // --loc=percent --no-language
    let out_percent = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc=percent")
        .arg("--no-language")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_percent.status.success());
    let stdout_percent = String::from_utf8_lossy(&out_percent.stdout);
    assert!(!stdout_percent.contains("Language"));
    assert!(!stdout_percent.contains("Rust"));
    assert!(stdout_percent.contains("100.0%"));
}

#[test]
fn test_loc_language_config_file() {
    let tmp = TempTestDir::new("loc_lang_config");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");

    // Config with [loc] language = false
    let config_loc = tmp.create_file("config_loc.toml", b"[loc]\nlanguage = false\n");
    let out_loc = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc")
        .arg("--config")
        .arg(config_loc)
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_loc.status.success());
    let stdout_loc = String::from_utf8_lossy(&out_loc.stdout);
    assert!(!stdout_loc.contains("Language"));
    assert!(!stdout_loc.contains("Rust"));

    // Config with [display] language = false
    let config_display = tmp.create_file("config_disp.toml", b"[display]\nlanguage = false\n");
    let out_disp = Command::new(bin_path())
        .arg("-lh")
        .arg("--loc")
        .arg("--config")
        .arg(config_display)
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_disp.status.success());
    let stdout_disp = String::from_utf8_lossy(&out_disp.stdout);
    assert!(!stdout_disp.contains("Language"));
    assert!(!stdout_disp.contains("Rust"));
}
