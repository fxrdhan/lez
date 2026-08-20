// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use lsr::options::config::ThemeConfig;
use nu_ansi_term::Color;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("lsr_adv_m2_{prefix}_{pid}_{nanos}_{count}"));
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn subpath(&self, sub: &str) -> PathBuf {
        let p = self.path.join(sub);
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lsr"))
}

// ----------------------------------------------------------------------------
// 1. Live CLI Executions: LSR_CONFIG_DIR, EZA_CONFIG_DIR, XDG_CONFIG_HOME
// ----------------------------------------------------------------------------

#[test]
fn test_live_cli_with_lsr_config_dir_tilde_expansion() {
    let temp = TempTestDir::new("live_lsr_tilde");
    let home = temp.subpath("fake_home");
    let config_dir = home.join(".config").join("lsr");
    fs::create_dir_all(&config_dir).unwrap();

    // Red directory color in theme.yml
    let theme_content = "filekinds:\n  directory:\n    fg: red\n";
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("LSR_CONFIG_DIR", "~/.config/lsr")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success(), "lsr failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;31m") || stdout.contains("\x1b[31m"),
        "Stdout should contain red ANSI code for directory: {}",
        stdout
    );
}

#[test]
fn test_live_cli_with_eza_config_dir_tilde_expansion() {
    let temp = TempTestDir::new("live_eza_tilde");
    let home = temp.subpath("fake_home");
    let config_dir = home.join(".config").join("eza");
    fs::create_dir_all(&config_dir).unwrap();

    // Green directory color in theme.yml
    let theme_content = "filekinds:\n  directory:\n    fg: green\n";
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("EZA_CONFIG_DIR", "~/.config/eza")
        .env_remove("LSR_CONFIG_DIR")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success(), "lsr failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;32m") || stdout.contains("\x1b[32m"),
        "Stdout should contain green ANSI code for directory: {}",
        stdout
    );
}

#[test]
fn test_live_cli_with_xdg_config_home_tilde_expansion() {
    let temp = TempTestDir::new("live_xdg_tilde");
    let home = temp.subpath("fake_home");
    let config_dir = home.join("my_xdg").join("lsr");
    fs::create_dir_all(&config_dir).unwrap();

    // Yellow directory color in theme.yml
    let theme_content = "filekinds:\n  directory:\n    fg: yellow\n";
    fs::write(config_dir.join("theme.yml"), theme_content).unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", "~/my_xdg")
        .env_remove("LSR_CONFIG_DIR")
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success(), "lsr failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;33m") || stdout.contains("\x1b[33m"),
        "Stdout should contain yellow ANSI code for directory: {}",
        stdout
    );
}

// ----------------------------------------------------------------------------
// 2. Theme file format support: theme.yml vs theme.yaml
// ----------------------------------------------------------------------------

#[test]
fn test_live_cli_theme_yaml_support() {
    let temp = TempTestDir::new("live_yaml_ext");
    let home = temp.subpath("fake_home");
    let config_dir = home.join(".config").join("lsr");
    fs::create_dir_all(&config_dir).unwrap();

    // Cyan directory color in theme.yaml (note .yaml extension)
    let theme_content = "filekinds:\n  directory:\n    fg: cyan\n";
    fs::write(config_dir.join("theme.yaml"), theme_content).unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("LSR_CONFIG_DIR", "~/.config/lsr")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success(), "lsr failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;36m") || stdout.contains("\x1b[36m"),
        "Stdout should contain cyan ANSI code from theme.yaml: {}",
        stdout
    );
}

#[test]
fn test_live_cli_theme_yml_takes_priority_over_yaml() {
    let temp = TempTestDir::new("live_yml_over_yaml");
    let home = temp.subpath("fake_home");
    let config_dir = home.join(".config").join("lsr");
    fs::create_dir_all(&config_dir).unwrap();

    // theme.yml has RED, theme.yaml has CYAN
    fs::write(
        config_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: red\n",
    )
    .unwrap();
    fs::write(
        config_dir.join("theme.yaml"),
        "filekinds:\n  directory:\n    fg: cyan\n",
    )
    .unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("LSR_CONFIG_DIR", "~/.config/lsr")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success(), "lsr failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;31m") || stdout.contains("\x1b[31m"),
        "theme.yml (red) must take precedence over theme.yaml"
    );
    assert!(
        !stdout.contains("\x1b[1;36m") && !stdout.contains("\x1b[36m"),
        "theme.yaml (cyan) must NOT be applied when theme.yml is present"
    );
}

// ----------------------------------------------------------------------------
// 3. Env Var Precedence: LSR_CONFIG_DIR > EZA_CONFIG_DIR > XDG_CONFIG_HOME
// ----------------------------------------------------------------------------

#[test]
fn test_live_cli_env_precedence_lsr_over_eza() {
    let temp = TempTestDir::new("live_prec_lsr_eza");
    let home = temp.subpath("fake_home");

    let lsr_dir = home.join("lsr_conf");
    let eza_dir = home.join("eza_conf");
    fs::create_dir_all(&lsr_dir).unwrap();
    fs::create_dir_all(&eza_dir).unwrap();

    // LSR_CONFIG_DIR = Red, EZA_CONFIG_DIR = Green
    fs::write(
        lsr_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: red\n",
    )
    .unwrap();
    fs::write(
        eza_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: green\n",
    )
    .unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env("LSR_CONFIG_DIR", "~/lsr_conf")
        .env("EZA_CONFIG_DIR", "~/eza_conf")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;31m") || stdout.contains("\x1b[31m"),
        "LSR_CONFIG_DIR (red) should override EZA_CONFIG_DIR (green)"
    );
}

#[test]
fn test_live_cli_env_precedence_eza_over_xdg() {
    let temp = TempTestDir::new("live_prec_eza_xdg");
    let home = temp.subpath("fake_home");

    let eza_dir = home.join("eza_conf");
    let xdg_dir = home.join("xdg_conf").join("lsr");
    fs::create_dir_all(&eza_dir).unwrap();
    fs::create_dir_all(&xdg_dir).unwrap();

    // EZA_CONFIG_DIR = Green, XDG_CONFIG_HOME = Yellow
    fs::write(
        eza_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: green\n",
    )
    .unwrap();
    fs::write(
        xdg_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: yellow\n",
    )
    .unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg("--color=always")
        .arg(&target_dir)
        .env("HOME", &home)
        .env_remove("LSR_CONFIG_DIR")
        .env("EZA_CONFIG_DIR", "~/eza_conf")
        .env("XDG_CONFIG_HOME", "~/xdg_conf")
        .env_remove("LS_COLORS")
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1;32m") || stdout.contains("\x1b[32m"),
        "EZA_CONFIG_DIR (green) should override XDG_CONFIG_HOME (yellow)"
    );
}

// ----------------------------------------------------------------------------
// 4. Dollar HOME Variants in Live CLI
// ----------------------------------------------------------------------------

#[test]
fn test_live_cli_dollar_home_variants() {
    let temp = TempTestDir::new("live_dollar_home");
    let home = temp.subpath("fake_home");
    let config_dir = home.join("custom_config");
    fs::create_dir_all(&config_dir).unwrap();

    fs::write(
        config_dir.join("theme.yml"),
        "filekinds:\n  directory:\n    fg: magenta\n",
    )
    .unwrap();

    let target_dir = temp.subpath("target");
    fs::create_dir_all(target_dir.join("test_subdir")).unwrap();

    let lsr = bin_path();
    let variants = ["$HOME/custom_config", "${HOME}/custom_config"];
    for var in variants {
        let output = Command::new(&lsr)
            .arg("--color=always")
            .arg(&target_dir)
            .env("HOME", &home)
            .env("LSR_CONFIG_DIR", var)
            .env_remove("LS_COLORS")
            .env_remove("EZA_COLORS")
            .env_remove("EXA_COLORS")
            .output()
            .expect("Failed to execute lsr binary");

        assert!(
            output.status.success(),
            "Failed with env {var}: {:?}",
            output
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\x1b[1;35m") || stdout.contains("\x1b[35m"),
            "Expected magenta ANSI code for {var}: {stdout}"
        );
    }
}

// ----------------------------------------------------------------------------
// 5. Stress & Edge Cases: Corrupted YAML, 0-byte file, Dir as file, Non-existent
// ----------------------------------------------------------------------------

#[test]
fn test_theme_config_to_theme_api() {
    let temp = TempTestDir::new("theme_api");
    let theme_file = temp.subpath("theme.yml");
    fs::write(&theme_file, "filekinds:\n  directory:\n    fg: cyan\n").unwrap();

    let cfg = ThemeConfig::from_path(theme_file.clone());
    assert_eq!(cfg.location(), theme_file.as_path());
    let ui_opt = cfg.to_theme();
    assert!(ui_opt.is_some());
    let ui = ui_opt.unwrap();
    assert_eq!(
        ui.filekinds.unwrap().directory.unwrap().foreground,
        Some(Color::Cyan)
    );
}

#[test]
fn test_corrupted_theme_yaml_graceful_fallback() {
    let temp = TempTestDir::new("corrupted_theme");
    let cfg_dir = temp.subpath("bad_cfg");
    fs::create_dir_all(&cfg_dir).unwrap();
    // Intentionally corrupted / invalid YAML
    let bad_theme = cfg_dir.join("theme.yml");
    fs::write(&bad_theme, b":: invalid yaml [[[[ \n  bad_syntax: {{{\n").unwrap();

    let cfg = ThemeConfig::from_path(bad_theme);
    let ui_styles = cfg.to_theme();
    // to_theme returns Some(UiStyles::default()) on syntax error without panic
    assert!(
        ui_styles.is_some(),
        "Corrupted YAML should fall back safely to default UiStyles"
    );

    // Live CLI execution test with corrupted YAML
    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", &cfg_dir)
        .output()
        .expect("Failed to execute lsr binary");

    assert!(
        output.status.success(),
        "lsr must not crash on malformed theme YAML: {:?}",
        output
    );
}

#[test]
fn test_empty_theme_file_graceful_handling() {
    let temp = TempTestDir::new("empty_theme");
    let cfg_dir = temp.subpath("empty_cfg");
    fs::create_dir_all(&cfg_dir).unwrap();
    // 0-byte file
    let empty_file = cfg_dir.join("theme.yml");
    fs::write(&empty_file, b"").unwrap();

    let cfg = ThemeConfig::from_path(empty_file);
    let ui = cfg.to_theme();
    assert!(
        ui.is_some(),
        "Empty theme file safely returns default UiStyles"
    );

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", &cfg_dir)
        .output()
        .expect("Failed to execute lsr");

    assert!(
        output.status.success(),
        "lsr must succeed with empty theme file"
    );
}

#[test]
fn test_theme_as_directory_graceful_handling() {
    let temp = TempTestDir::new("dir_as_theme");
    let cfg_dir = temp.subpath("cfg");
    // Create theme.yml as a directory instead of a regular file
    let dir_theme = cfg_dir.join("theme.yml");
    fs::create_dir_all(&dir_theme).unwrap();

    let cfg = ThemeConfig::from_path(dir_theme);
    let ui = cfg.to_theme();
    assert!(
        ui.is_some(),
        "Directory as theme.yml safely returns default styles"
    );

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", &cfg_dir)
        .output()
        .expect("Failed to execute lsr");

    assert!(
        output.status.success(),
        "lsr must handle directory named theme.yml without crashing"
    );
}

#[test]
fn test_symlinked_config_directory_and_file() {
    #[cfg(unix)]
    {
        let temp = TempTestDir::new("symlink_cfg");
        let real_cfg = temp.subpath("real_cfg");
        fs::create_dir_all(&real_cfg).unwrap();
        fs::write(
            real_cfg.join("theme.yml"),
            "filekinds:\n  directory:\n    fg: cyan\n",
        )
        .unwrap();

        let symlink_cfg = temp.subpath("symlink_cfg");
        std::os::unix::fs::symlink(&real_cfg, &symlink_cfg).unwrap();

        let lsr = bin_path();
        let target = temp.subpath("target");
        fs::create_dir_all(target.join("subdir")).unwrap();

        let output = Command::new(&lsr)
            .arg("--color=always")
            .arg(&target)
            .env("LSR_CONFIG_DIR", &symlink_cfg)
            .env_remove("LS_COLORS")
            .env_remove("EZA_COLORS")
            .env_remove("EXA_COLORS")
            .output()
            .expect("Failed to execute lsr");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\x1b[1;36m") || stdout.contains("\x1b[36m"),
            "Cyan ANSI code for directory expected: {}",
            stdout
        );
    }
}

#[test]
fn test_non_existent_config_dir_fallback() {
    let temp = TempTestDir::new("nonexistent_cfg");

    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", "~/completely_bogus_dir_99999")
        .output()
        .expect("Failed to execute lsr");

    assert!(
        output.status.success(),
        "lsr must succeed even if config dir does not exist"
    );
}

#[test]
fn test_empty_env_vars_stability() {
    let temp = TempTestDir::new("empty_env");
    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", "")
        .env("EZA_CONFIG_DIR", "")
        .env("XDG_CONFIG_HOME", "")
        .output()
        .expect("Failed to execute lsr");

    assert!(
        output.status.success(),
        "lsr must succeed with empty config env variables"
    );
}

#[test]
fn test_relative_xdg_config_home_live_cli() {
    let temp = TempTestDir::new("rel_xdg_cli");
    let home = temp.subpath("home");
    let lsr = bin_path();
    let output = Command::new(&lsr)
        .arg(&temp.path)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", "relative_dir/without/root")
        .output()
        .expect("Failed to execute lsr");

    assert!(
        output.status.success(),
        "lsr must ignore relative XDG_CONFIG_HOME and exit with success"
    );
}
