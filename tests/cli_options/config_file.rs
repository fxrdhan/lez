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
            "lez_cfg_test_{label}_{}_{}",
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
fn test_explicit_config_file_cli_flag() {
    let temp = TempTestDir::new("explicit_cli");
    let test_file = temp.path.join("file_a.txt");
    fs::write(&test_file, b"content").unwrap();

    let config_path = temp.path.join("my_custom_config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true

[icons]
icons = "never"
"#,
    )
    .unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with --config");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via --config: {stdout}"
    );
}

#[test]
fn test_no_config_flag_ignores_config_file() {
    let temp = TempTestDir::new("no_config");
    let test_file = temp.path.join("file_a.txt");
    fs::write(&test_file, b"content").unwrap();

    let config_dir = temp.path.join("config_dir");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    // With LEZ_CONFIG_DIR pointing to config_dir, but with --no-config
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .arg("--no-config")
        .arg(&temp.path)
        .output()
        .expect("run lez with --no-config");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Permissions") && !stdout.contains("Size") && !stdout.contains("Name"),
        "Header should NOT be displayed when --no-config is passed: {stdout}"
    );
}

#[test]
fn test_global_config_dir_discovery() {
    let temp = TempTestDir::new("global_discovery");
    let test_file = temp.path.join("sample.txt");
    fs::write(&test_file, b"sample").unwrap();

    let config_dir = temp.path.join("lez_config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with LEZ_CONFIG_DIR");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via LEZ_CONFIG_DIR config.toml: {stdout}"
    );
}

#[test]
fn test_local_directory_lez_toml_overrides_global() {
    let temp = TempTestDir::new("local_override");
    let workdir = temp.path.join("project");
    fs::create_dir_all(&workdir).unwrap();
    fs::write(workdir.join("a.txt"), b"1").unwrap();
    fs::write(workdir.join("b.txt"), b"2").unwrap();

    // Global config: header = false
    let config_dir = temp.path.join("lez_global");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
[display]
header = false
"#,
    )
    .unwrap();

    // Local .lez.toml in workdir: header = true
    fs::write(
        workdir.join(".lez.toml"),
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .current_dir(&workdir)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .output()
        .expect("run lez with local .lez.toml");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Local .lez.toml should enable header overriding global config: {stdout}"
    );
}

#[test]
fn test_cli_argument_overrides_config_file() {
    let temp = TempTestDir::new("cli_precedence");
    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let file_a = temp.path.join("file_a.txt");
    fs::write(&file_a, b"test").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    // Config enables header, but CLI explicitly runs without long table (e.g. oneline)
    let output = Command::new(lez_bin)
        .arg("--config")
        .arg(&config_path)
        .arg("--oneline")
        .arg(&temp.path)
        .output()
        .expect("run lez with --oneline overriding header");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Permissions") && !stdout.contains("Size"),
        "CLI --oneline should take precedence over config header: {stdout}"
    );
}

#[test]
fn test_env_var_lez_config_file() {
    let temp = TempTestDir::new("env_config_file");
    let config_path = temp.path.join("special_config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let file_a = temp.path.join("file.txt");
    fs::write(&file_a, b"test").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .env("LEZ_CONFIG_FILE", &config_path)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with LEZ_CONFIG_FILE");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via LEZ_CONFIG_FILE: {stdout}"
    );
}

#[test]
fn test_malformed_config_file_handled_gracefully() {
    let temp = TempTestDir::new("malformed_config");
    let config_path = temp.path.join("broken_config.toml");
    fs::write(&config_path, b"invalid = toml [ broken syntax").unwrap();

    let file_a = temp.path.join("file.txt");
    fs::write(&file_a, b"test").unwrap();

    let lez_bin = env!("CARGO_BIN_EXE_lez");
    let output = Command::new(lez_bin)
        .arg("--config")
        .arg(&config_path)
        .arg(&temp.path)
        .output()
        .expect("run lez with malformed config");

    assert!(
        output.status.success(),
        "lez should exit 0 even if config syntax is invalid"
    );
}
