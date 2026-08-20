// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_theme_emoji_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    fn create_dir(&self, rel_path: &str) -> PathBuf {
        let dir_path = self.path.join(rel_path);
        fs::create_dir_all(&dir_path).unwrap();
        dir_path
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
    path.join(if cfg!(windows) { "lsr.exe" } else { "lsr" })
}

#[test]
fn test_theme_with_multicodepoint_emojis() {
    let temp = TempTestDir::new("emoji_theme");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
filenames:
  data.bin:
    icon:
      glyph: "💾"
  Pictures:
    icon:
      glyph: "🖼️"
  developer:
    icon:
      glyph: "👨‍💻"
  flag:
    icon:
      glyph: "🇺🇸"
  wave:
    icon:
      glyph: "👋🏻"

directorynames:
  Docs:
    icon:
      glyph: "📁"

extensions:
  rs:
    icon:
      glyph: "🦀"
  py:
    icon:
      glyph: "🐍"
"#;
    let theme_file = config_dir.join("theme.yml");
    let mut f = StdFile::create(&theme_file).unwrap();
    f.write_all(theme_content.as_bytes()).unwrap();

    temp.create_file("data.bin", b"data");
    temp.create_file("Pictures", b"pic");
    temp.create_file("developer", b"dev");
    temp.create_file("flag", b"flag");
    temp.create_file("wave", b"wave");
    temp.create_dir("Docs");
    temp.create_file("main.rs", b"fn main() {}");

    let output = Command::new(bin_path())
        .arg("--color=always")
        .arg("--icons=always")
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", &config_dir)
        .output()
        .expect("Failed to execute lsr with custom emoji theme");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("💾"));
    assert!(stdout.contains("🖼️"));
    assert!(stdout.contains("👨‍💻"));
    assert!(stdout.contains("🇺🇸"));
    assert!(stdout.contains("👋🏻"));
    assert!(stdout.contains("📁"));
    assert!(stdout.contains("🦀"));
}

#[test]
fn test_theme_deserialization_nested_emojis() {
    let temp = TempTestDir::new("deser_theme");
    let config_dir = temp.path.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let theme_content = r#"
filenames:
  family:
    icon:
      glyph: "👨‍👩‍👧‍👦"
"#;
    let theme_file = config_dir.join("theme.yml");
    let mut f = StdFile::create(&theme_file).unwrap();
    f.write_all(theme_content.as_bytes()).unwrap();

    temp.create_file("family", b"members");

    let output = Command::new(bin_path())
        .arg("--color=always")
        .arg("--icons=always")
        .arg(&temp.path)
        .env("LSR_CONFIG_DIR", &config_dir)
        .output()
        .expect("Failed to execute lsr with family emoji");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("👨‍👩‍👧‍👦"));
}
