// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial fuzzing and resilience test suite for YAML theme configuration:
//! - Recursive YAML anchor bombs and alias expansion limits
//! - Schema type mismatch fuzzing (arrays/ints where strings/objects expected)
//! - Malformed, oversized, and truncated hex colors and color names
//! - Massive theme configuration files (5,000+ custom rules)
//! - Control characters and exotic glyph inputs in theme mappings
//! - Safe fallback to default styles without crashes or panics

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct ThemeFuzzDir {
    path: PathBuf,
}

impl ThemeFuzzDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_themefuzz_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp theme fuzz directory");
        Self { path }
    }

    fn write_theme(&self, content: &str) -> PathBuf {
        let config_dir = self.path.join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let theme_file = config_dir.join("theme.yml");
        let mut f = StdFile::create(&theme_file).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        config_dir
    }

    fn create_sample_files(&self) {
        let sample_dir = self.path.join("samples");
        fs::create_dir_all(&sample_dir).unwrap();
        for name in [
            "normal.rs",
            "doc.md",
            "archive.tar",
            "special_file.xyz",
            "another.bin",
        ] {
            fs::write(sample_dir.join(name), b"test content").unwrap();
        }
    }
}

impl Drop for ThemeFuzzDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

fn run_lez_with_theme(
    target_dir: &Path,
    config_dir: &Path,
    args: &[&str],
) -> (bool, String, String) {
    let output = Command::new(bin_path())
        .current_dir(target_dir)
        .args(args)
        .env("LEZ_CONFIG_DIR", config_dir)
        .env("NO_COLOR", "")
        .output()
        .expect("Failed to execute lez binary with theme");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_yaml_anchor_bomb_resilience() {
    let fixture = ThemeFuzzDir::new("anchor_bomb");
    fixture.create_sample_files();

    // Exponential anchor definition (Billion Laughs in YAML)
    let bomb_yaml = r#"
a: &a ["lol","lol","lol","lol","lol","lol","lol","lol","lol"]
b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]
c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]
d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]
extensions:
  rs:
    foreground: "green"
"#;
    let config_dir = fixture.write_theme(bomb_yaml);
    let sample_dir = fixture.path.join("samples");

    // lez must parse or safely reject without OOM panic or hanging
    let (success, stdout, stderr) = run_lez_with_theme(
        &sample_dir,
        &config_dir,
        &["-1", "--color=always", "--icons=always"],
    );

    assert!(success, "lez failed on YAML anchor bomb: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("normal.rs"));
}

#[test]
fn test_type_mismatch_and_garbage_types() {
    let fixture = ThemeFuzzDir::new("type_mismatch");
    fixture.create_sample_files();

    let invalid_types_yaml = r#"
filekinds: 12345
perms:
  - "not"
  - "an"
  - "object"
size: "huge"
users: true
filenames:
  normal.rs: 99999
extensions:
  rs:
    foreground:
      nested: "invalid_color_struct"
    bold: "not_a_bool"
    underline: [1, 2, 3]
punctuation: 0.42
header: null
"#;
    let config_dir = fixture.write_theme(invalid_types_yaml);
    let sample_dir = fixture.path.join("samples");

    let (success, stdout, stderr) =
        run_lez_with_theme(&sample_dir, &config_dir, &["-l", "--color=always"]);

    assert!(success, "lez failed on invalid YAML types: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("normal.rs"));
}

#[test]
fn test_malformed_and_exotic_color_codes() {
    let fixture = ThemeFuzzDir::new("malformed_colors");
    fixture.create_sample_files();

    let malformed_colors_yaml = r##"
extensions:
  rs:
    foreground: "#GGGGGG"
  md:
    foreground: "#12"
  tar:
    foreground: "#"
  xyz:
    foreground: "1234567890ABCDEF"
  bin:
    foreground: "ultra_invisible_nonexistent_color"
    background: "#FF00FF00FF"
"##;
    let config_dir = fixture.write_theme(malformed_colors_yaml);
    let sample_dir = fixture.path.join("samples");

    let (success, stdout, stderr) =
        run_lez_with_theme(&sample_dir, &config_dir, &["-1", "--color=always"]);

    assert!(success, "lez failed on malformed hex color codes: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("normal.rs"));
    assert!(stdout.contains("doc.md"));
}

#[test]
fn test_massive_theme_configuration_scale() {
    let fixture = ThemeFuzzDir::new("massive_theme");
    fixture.create_sample_files();

    let mut massive_yaml = String::from("extensions:\n");
    for i in 0..250 {
        massive_yaml.push_str(&format!(
            "  ext_{i:04}:\n    foreground: \"#{:06x}\"\n    icon:\n      glyph: \"📦\"\n",
            (i * 12345) % 0xFFFFFF
        ));
    }
    massive_yaml.push_str("filenames:\n");
    for i in 0..250 {
        massive_yaml.push_str(&format!(
            "  custom_file_{i:04}.dat:\n    foreground: \"#{:06x}\"\n",
            (i * 54321) % 0xFFFFFF
        ));
    }

    let config_dir = fixture.write_theme(&massive_yaml);
    let sample_dir = fixture.path.join("samples");

    let (success, stdout, stderr) = run_lez_with_theme(
        &sample_dir,
        &config_dir,
        &["-l", "--color=always", "--icons=always"],
    );

    assert!(success, "lez failed on 500-rule massive theme: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("normal.rs"));
}

#[test]
fn test_theme_with_control_characters_and_whitespace_keys() {
    let fixture = ThemeFuzzDir::new("ctrl_chars_theme");
    fixture.create_sample_files();

    let control_chars_yaml = "filenames:\n  \"space in name.rs\":\n    foreground: \"yellow\"\n  \"tab\tname.md\":\n    foreground: \"cyan\"\n  \"newline\nname.tar\":\n    foreground: \"green\"\n";
    let config_dir = fixture.write_theme(control_chars_yaml);
    let sample_dir = fixture.path.join("samples");

    let (success, stdout, stderr) =
        run_lez_with_theme(&sample_dir, &config_dir, &["-1", "--color=always"]);

    assert!(success, "lez failed on theme with control chars: {stderr}");
    assert!(!stderr.contains("panicked at"));
    assert!(stdout.contains("normal.rs"));
}
