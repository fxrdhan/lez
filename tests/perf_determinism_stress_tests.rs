// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial test suite verifying multi-threaded output determinism,
//! sorting stability, and byte-for-byte consistency across rapid successive
//! invocations of `lsr`.

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File as StdFile};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct DeterminismTestDir {
    path: PathBuf,
}

impl DeterminismTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lsr_det_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn populate_diverse_corpus(&self) {
        // 1. Natural numeric ordering files
        for i in [1, 2, 3, 5, 9, 10, 11, 20, 21, 100, 101, 200] {
            self.create_file(
                &format!("num_item_{i}.txt"),
                format!("content {i}").as_bytes(),
            );
        }

        // 2. Case variations
        for name in [
            "alpha.txt",
            "Alpha.txt",
            "ALPHA.txt",
            "beta.txt",
            "Beta.txt",
            "BETA.txt",
        ] {
            self.create_file(name, b"case test");
        }

        // 3. Unicode and non-ASCII names
        for name in [
            "café_au_lait.txt",
            "naïve_approach.txt",
            "日本語_テスト.txt",
            "한국어_파일.txt",
            "emoji_🎉_party.txt",
            "crab_🦀_rust.txt",
            "umlaut_über_münchen.txt",
        ] {
            self.create_file(name, b"unicode payload");
        }

        // 4. Dotfiles
        for dot in [
            ".hidden_one",
            ".hidden_two",
            ".config_sample.json",
            ".env.local",
        ] {
            self.create_file(dot, b"dotfile content");
        }

        // 5. Files with varying sizes
        self.create_file("size_zero.dat", b"");
        self.create_file("size_small.dat", &[b'A'; 64]);
        self.create_file("size_medium.dat", &[b'B'; 4096]);
        self.create_file("size_large.dat", &[b'C'; 65536]);

        // 6. Subdirectories and nested files
        let sub1 = self.path.join("sub_alpha");
        let sub2 = self.path.join("sub_beta");
        fs::create_dir_all(&sub1).unwrap();
        fs::create_dir_all(&sub2).unwrap();
        StdFile::create(sub1.join("nested_1.txt"))
            .unwrap()
            .write_all(b"sub1")
            .unwrap();
        StdFile::create(sub2.join("nested_2.txt"))
            .unwrap()
            .write_all(b"sub2")
            .unwrap();

        // 7. Symlinks on Unix platforms
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let _ = symlink(
                self.path.join("size_small.dat"),
                self.path.join("link_to_small.dat"),
            );
            let _ = symlink(&sub1, self.path.join("link_to_sub1"));
        }
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
}

impl Drop for DeterminismTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(dir: &Path, args: &[&str]) -> (String, u64) {
    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .env("LSR_COLORS", "reset")
        .output()
        .expect("Failed to execute lsr binary");

    assert!(
        output.status.success(),
        "lsr command failed with status {}: stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let mut hasher = DefaultHasher::new();
    stdout_str.hash(&mut hasher);
    let hash = hasher.finish();

    (stdout_str, hash)
}

fn assert_deterministic_across_iterations(
    dir: &Path,
    args: &[&str],
    iterations: usize,
    context: &str,
) {
    let (first_output, first_hash) = run_lsr(dir, args);
    assert!(
        !first_output.is_empty(),
        "First output for {context} was empty"
    );

    for i in 1..iterations {
        let (output, hash) = run_lsr(dir, args);
        assert_eq!(
            first_hash, hash,
            "Determinism mismatch on iteration {i} for {context} (args: {args:?})\nExpected:\n{first_output}\nGot:\n{output}"
        );
        assert_eq!(
            first_output, output,
            "Byte mismatch on iteration {i} for {context} (args: {args:?})"
        );
    }
}

#[test]
fn determinism_grid_default_view() {
    let fixture = DeterminismTestDir::new("grid");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-G", "--color=never"],
        30,
        "grid default view",
    );
}

#[test]
fn determinism_oneline_mode() {
    let fixture = DeterminismTestDir::new("oneline");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-1", "--color=never"],
        30,
        "oneline mode",
    );
}

#[test]
fn determinism_oneline_with_all_dotfiles() {
    let fixture = DeterminismTestDir::new("oneline_all");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-1", "-a", "--color=never"],
        30,
        "oneline with all dotfiles",
    );
}

#[test]
fn determinism_long_details_table() {
    let fixture = DeterminismTestDir::new("long");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-l", "--color=never", "--time-style=iso"],
        25,
        "long details view",
    );
}

#[test]
fn determinism_tree_view() {
    let fixture = DeterminismTestDir::new("tree");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-T", "--color=never"],
        25,
        "tree view",
    );
}

#[test]
fn determinism_json_output() {
    let fixture = DeterminismTestDir::new("json");
    fixture.populate_diverse_corpus();

    let (json_str, _) = run_lsr(&fixture.path, &["--json", "--color=never"]);
    // Verify valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");
    assert!(parsed.is_array());

    assert_deterministic_across_iterations(
        &fixture.path,
        &["--json", "--color=never"],
        20,
        "structured JSON output",
    );
}

#[test]
fn determinism_across_all_sort_fields() {
    let fixture = DeterminismTestDir::new("sorts");
    fixture.populate_diverse_corpus();

    let sort_fields = ["name", "Name", "lexicographic", "size", "ext", "path"];

    for field in sort_fields {
        assert_deterministic_across_iterations(
            &fixture.path,
            &["-1", &format!("--sort={field}"), "--color=never"],
            15,
            &format!("sort={field}"),
        );
    }
}

#[test]
fn determinism_reverse_sort() {
    let fixture = DeterminismTestDir::new("reverse_sort");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-1", "-r", "--sort=name", "--color=never"],
        20,
        "reverse sort name",
    );

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-1", "-r", "--sort=size", "--color=never"],
        20,
        "reverse sort size",
    );
}

#[test]
fn determinism_icons_and_classify_flags() {
    let fixture = DeterminismTestDir::new("icons_classify");
    fixture.populate_diverse_corpus();

    assert_deterministic_across_iterations(
        &fixture.path,
        &["-1", "--icons=always", "-F=always", "--color=never"],
        20,
        "icons and classify flags",
    );
}
