// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "lez_theme_test_{label}_{}_{}",
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

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_no_config_flag_suppresses_theme_yml() {
    let config_dir = TempTestDir::new("noconfig_theme");
    let theme_path = config_dir.path.join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filenames:\n  test_sample.txt:\n    filename:\n      foreground: Red"
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("noconfig_work");
    let sample = work_dir.path.join("test_sample.txt");
    StdFile::create(&sample).expect("create sample file");

    // 1. Without --no-config: theme.yml is loaded and applies Red color (\x1b[31m)
    let out_themed = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", &config_dir.path)
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("EXA_COLORS")
        .arg("--color=always")
        .arg(&sample)
        .output()
        .expect("run lez with theme");
    assert!(out_themed.status.success());
    let stdout_themed = String::from_utf8_lossy(&out_themed.stdout);
    assert!(
        stdout_themed.contains("\x1b[31m"),
        "Expected Red ANSI color from theme.yml, got: {stdout_themed:?}"
    );

    // 2. With --no-config: theme.yml MUST NOT be loaded
    let out_noconfig = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", &config_dir.path)
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("EXA_COLORS")
        .arg("--no-config")
        .arg("--color=always")
        .arg(&sample)
        .output()
        .expect("run lez with --no-config");
    assert!(out_noconfig.status.success());
    let stdout_noconfig = String::from_utf8_lossy(&out_noconfig.stdout);
    assert!(
        !stdout_noconfig.contains("\x1b[31m"),
        "--no-config must suppress theme.yml loading, but found Red ANSI color: {stdout_noconfig:?}"
    );
}

#[test]
fn test_corrupt_theme_yml_falls_back_to_default_theme_with_warning() {
    let config_dir = TempTestDir::new("corrupt_theme");
    let theme_path = config_dir.path.join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(f, "[[[ this is definitely corrupted yaml : {{").expect("write corrupt yaml");

    let work_dir = TempTestDir::new("corrupt_theme_work");
    let sample = work_dir.path.join("sample_file.txt");
    StdFile::create(&sample).expect("create sample file");

    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", &config_dir.path)
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .arg("-l")
        .arg("--color=always")
        .arg(&sample)
        .output()
        .expect("run lez with corrupt theme");

    assert!(
        out.status.success(),
        "lez should not crash or fail when theme is corrupt"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Failed to parse theme file"),
        "Expected stderr warning about failed theme parsing, got: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    // When falling back to default theme with --color=always, ANSI styling must still be present
    assert!(
        stdout.contains("\x1b["),
        "Output should retain default theme ANSI styling instead of plain text, got: {stdout:?}"
    );
}
