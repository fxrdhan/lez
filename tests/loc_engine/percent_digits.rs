// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::process::Command;

use crate::common::{TempTestDir, bin_path};

#[test]
fn test_code_percent_digits_cli_flag() {
    let tmp = TempTestDir::new("pct_digits_cli");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    tmp.create_file("script.py", b"print('test')\n");

    // Default (1 decimal place)
    let out_default = Command::new(bin_path())
        .arg("--code")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_default.status.success());
    let stdout_default = String::from_utf8_lossy(&out_default.stdout);
    assert!(stdout_default.contains("75.0%"));
    assert!(stdout_default.contains("25.0%"));
    assert!(stdout_default.contains("100.0%"));

    // 0 decimal places (integer percent)
    let out_zero = Command::new(bin_path())
        .arg("--code")
        .arg("--percent-digits=0")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_zero.status.success());
    let stdout_zero = String::from_utf8_lossy(&out_zero.stdout);
    assert!(stdout_zero.contains("75%"));
    assert!(stdout_zero.contains("25%"));
    assert!(stdout_zero.contains("100%"));

    // 3 decimal places
    let out_three = Command::new(bin_path())
        .arg("--code")
        .arg("--percent-digits=3")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_three.status.success());
    let stdout_three = String::from_utf8_lossy(&out_three.stdout);
    assert!(stdout_three.contains("75.000%"));
    assert!(stdout_three.contains("25.000%"));
    assert!(stdout_three.contains("100.000%"));

    // Alias --precision-percent=2
    let out_alias = Command::new(bin_path())
        .arg("--code")
        .arg("--precision-percent=2")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_alias.status.success());
    let stdout_alias = String::from_utf8_lossy(&out_alias.stdout);
    assert!(stdout_alias.contains("75.00%"));
    assert!(stdout_alias.contains("25.00%"));
    assert!(stdout_alias.contains("100.00%"));
}

#[test]
fn test_code_percent_digits_env_var() {
    let tmp = TempTestDir::new("pct_digits_env");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");

    let out = Command::new(bin_path())
        .env("LEZ_PERCENT_DIGITS", "2")
        .arg("--code")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("100.00%"));
}

#[test]
fn test_code_percent_digits_config_file() {
    let tmp = TempTestDir::new("pct_digits_cfg");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    tmp.create_file(".lez.toml", b"[loc]\npercent_digits = 3\n");

    let out = Command::new(bin_path())
        .arg("--code")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("100.000%"));
}

#[test]
fn test_loc_column_percent_digits_in_long_view() {
    let tmp = TempTestDir::new("pct_digits_long");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");

    let out = Command::new(bin_path())
        .arg("-l")
        .arg("--loc=percent")
        .arg("--percent-digits=2")
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("100.00%"));
}
