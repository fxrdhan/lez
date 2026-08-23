// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Integration tests for path-aware ignore glob pattern matching (#1446).
//!
//! Verifies that:
//! - Patterns containing directory separators (`/` or `\\`) match against relative file paths.
//! - Patterns without directory separators match against leaf filenames everywhere.
//! - Recursive globs (`**/node_modules/*`, `target/*`) work as expected.
//! - Case-insensitive ignore globs (`--ignore-glob-ci`) work with path-aware patterns.

use std::fs;
use std::path::PathBuf;
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
            "lsr_ignore_glob_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &str) {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr_in(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lsr"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

#[test]
fn test_path_aware_ignore_glob_subdir() {
    let temp = TempTestDir::new("subdir");
    temp.create_file("root.rs", "// root file");
    temp.create_file("Cargo.toml", "[package]");
    temp.create_file("src/main.rs", "fn main() {}");
    temp.create_file("src/lib.rs", "pub fn lib() {}");
    temp.create_file("src/fs/filter.rs", "// nested filter");
    temp.create_file("tests/integration.rs", "// integration test");

    // -I "src/*.rs" should ignore src/main.rs and src/lib.rs
    // but keep root.rs, src/fs/filter.rs, tests/integration.rs, Cargo.toml
    let output = run_lsr_in(&temp.path, &["-T", "-I", "src/*.rs", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("root.rs"), "Expected root.rs in:\n{stdout}");
    assert!(
        stdout.contains("Cargo.toml"),
        "Expected Cargo.toml in:\n{stdout}"
    );
    assert!(
        stdout.contains("filter.rs"),
        "Expected nested filter.rs in:\n{stdout}"
    );
    assert!(
        stdout.contains("integration.rs"),
        "Expected integration.rs in:\n{stdout}"
    );
    assert!(
        !stdout.contains("main.rs"),
        "Expected src/main.rs to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("lib.rs"),
        "Expected src/lib.rs to be ignored in:\n{stdout}"
    );
}

#[test]
fn test_path_aware_ignore_glob_recursive_wildcard() {
    let temp = TempTestDir::new("rec_node");
    temp.create_file("app.js", "console.log('app');");
    temp.create_file("node_modules/pkg/index.js", "module.exports = {};");
    temp.create_file(
        "packages/web/node_modules/lib/index.js",
        "module.exports = {};",
    );
    temp.create_file("packages/web/src/index.js", "export default {};");

    // -I "**/node_modules/*" keeps node_modules folder nodes in tree view
    // but omits their child contents
    let output = run_lsr_in(
        &temp.path,
        &["-T", "-I", "**/node_modules/*", "--color=never"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("app.js"), "Expected app.js in:\n{stdout}");
    assert!(
        stdout.contains("packages"),
        "Expected packages in:\n{stdout}"
    );
    assert!(
        stdout.contains("node_modules"),
        "Expected node_modules dir node in:\n{stdout}"
    );
    assert!(
        !stdout.contains("pkg"),
        "Expected node_modules/pkg to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("lib"),
        "Expected packages/web/node_modules/lib to be ignored in:\n{stdout}"
    );
}

#[test]
fn test_path_aware_ignore_glob_target_wildcard() {
    let temp = TempTestDir::new("target_wild");
    temp.create_file("src/main.rs", "fn main() {}");
    temp.create_file("target/debug/app", "binary");
    temp.create_file("target/release/app", "binary");
    temp.create_file("target/build.log", "log");

    let output = run_lsr_in(&temp.path, &["-T", "-I", "target/*", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src"), "Expected src in:\n{stdout}");
    assert!(stdout.contains("main.rs"), "Expected main.rs in:\n{stdout}");
    assert!(
        stdout.contains("target"),
        "Expected target dir in:\n{stdout}"
    );
    assert!(
        !stdout.contains("debug"),
        "Expected target/debug to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("release"),
        "Expected target/release to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("build.log"),
        "Expected target/build.log to be ignored in:\n{stdout}"
    );
}

#[test]
fn test_flat_filename_glob_matches_everywhere() {
    let temp = TempTestDir::new("flat_glob");
    temp.create_file("temp.tmp", "temp");
    temp.create_file("dir_a/sub.tmp", "temp");
    temp.create_file("dir_a/keep.txt", "keep");
    temp.create_file("dir_b/deep/nested.tmp", "temp");
    temp.create_file("dir_b/deep/keep.txt", "keep");

    let output = run_lsr_in(&temp.path, &["-T", "-I", "*.tmp", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("keep.txt"),
        "Expected keep.txt in:\n{stdout}"
    );
    assert!(
        !stdout.contains("temp.tmp"),
        "Expected temp.tmp ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("sub.tmp"),
        "Expected sub.tmp ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("nested.tmp"),
        "Expected nested.tmp ignored in:\n{stdout}"
    );
}

#[test]
fn test_flat_hidden_file_glob() {
    let temp = TempTestDir::new("hidden_glob");
    temp.create_file(".git/config", "repo");
    temp.create_file(".gitignore", "*.tmp");
    temp.create_file("normal.txt", "normal");
    temp.create_file("dir/.config", "secret");
    temp.create_file("dir/file.txt", "content");

    let output = run_lsr_in(&temp.path, &["-a", "-T", "-I", ".*", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("normal.txt"),
        "Expected normal.txt in:\n{stdout}"
    );
    assert!(
        stdout.contains("file.txt"),
        "Expected file.txt in:\n{stdout}"
    );
    assert!(
        !stdout.contains(".gitignore"),
        "Expected .gitignore to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains(".config"),
        "Expected .config to be ignored in:\n{stdout}"
    );
}

#[test]
fn test_case_insensitive_path_aware_ignore_glob() {
    let temp = TempTestDir::new("ci_path");
    temp.create_file("SRC/MAIN.RS", "fn main() {}");
    temp.create_file("src/sub/mod.rs", "pub mod sub;");
    temp.create_file("tests/test.rs", "// test");

    let output = run_lsr_in(
        &temp.path,
        &["-T", "--ignore-glob-ci=SRC/*.RS", "--color=never"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("mod.rs"),
        "Expected src/sub/mod.rs in:\n{stdout}"
    );
    assert!(
        stdout.contains("test.rs"),
        "Expected tests/test.rs in:\n{stdout}"
    );
    assert!(
        !stdout.contains("MAIN.RS"),
        "Expected SRC/MAIN.RS to be ignored in:\n{stdout}"
    );
}

#[test]
fn test_multiple_ignore_patterns_combined() {
    let temp = TempTestDir::new("multi_pats");
    temp.create_file("src/main.rs", "fn main() {}");
    temp.create_file("src/fs/filter.rs", "// filter");
    temp.create_file("target/debug/app", "binary");
    temp.create_file("junk.tmp", "junk");
    temp.create_file("keep.txt", "keep");

    let output = run_lsr_in(
        &temp.path,
        &["-T", "-I", "src/*.rs|target/*|*.tmp", "--color=never"],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("keep.txt"),
        "Expected keep.txt in:\n{stdout}"
    );
    assert!(
        stdout.contains("filter.rs"),
        "Expected filter.rs in:\n{stdout}"
    );
    assert!(
        !stdout.contains("main.rs"),
        "Expected src/main.rs ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("debug"),
        "Expected target/debug ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("junk.tmp"),
        "Expected junk.tmp ignored in:\n{stdout}"
    );
}

#[test]
fn test_leading_slash_normalization() {
    let temp = TempTestDir::new("leading_slash");
    temp.create_file("src/main.rs", "fn main() {}");
    temp.create_file("src/lib.rs", "pub fn lib() {}");
    temp.create_file("root.rs", "// root");

    let output = run_lsr_in(&temp.path, &["-T", "-I", "/src/*.rs", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("root.rs"), "Expected root.rs in:\n{stdout}");
    assert!(
        !stdout.contains("main.rs"),
        "Expected src/main.rs ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("lib.rs"),
        "Expected src/lib.rs ignored in:\n{stdout}"
    );
}

#[test]
fn test_trailing_slash_directory_ignore() {
    let temp = TempTestDir::new("trailing_slash");
    temp.create_file("node_modules/package.json", "{}");
    temp.create_file("node_modules/index.js", "module.exports = {};");
    temp.create_file("src/index.js", "export default {};");

    let output = run_lsr_in(&temp.path, &["-T", "-I", "node_modules/", "--color=never"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src"), "Expected src in:\n{stdout}");
    assert!(
        stdout.contains("index.js"),
        "Expected src/index.js in:\n{stdout}"
    );
    assert!(
        !stdout.contains("node_modules"),
        "Expected node_modules to be ignored in:\n{stdout}"
    );
    assert!(
        !stdout.contains("package.json"),
        "Expected package.json to be ignored in:\n{stdout}"
    );
}
