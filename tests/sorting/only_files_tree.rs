// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--only-files` (`-f`) combined with recursion.
//!
//! Tree mode must keep descending into directories while hiding the
//! directories themselves, with tree edges left intact; other modes keep
//! filtering directories out entirely.

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
            "lez_only_files_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::File::create(&file_path).unwrap();
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

fn run_lez(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lez"))
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("top.txt");
    dir.create_file("sub/mid.txt");
    dir.create_file("sub/deeper/leaf.txt");
    dir.create_dir("empty_dir");
    dir
}

/// Renders `args` against the fixture and replaces the fixture's own path
/// with `<ROOT>`, so the whole block can be compared literally.
fn tree_of(fixture: &TempTestDir, args: &[&str]) -> String {
    let root = fixture.path.to_str().unwrap();
    let mut argv: Vec<&str> = args.to_vec();
    argv.push("--color=never");
    argv.push(root);

    let output = run_lez(&argv);
    assert!(output.status.success(), "lez {argv:?} should succeed");

    String::from_utf8_lossy(&output.stdout)
        .replace(root, "<ROOT>")
        .trim_end()
        .to_string()
}

/// Directories are hidden but still descended into, so the files keep the
/// indentation of the level they actually live at.
///
/// Asserted as a whole block on purpose. Checking only that each name appears
/// and that some edge character is present passes on mangled output: the
/// prefixes of the hidden directory rows used to be concatenated onto the
/// surviving rows, printing "├── ├── └── leaf.txt", which satisfies every
/// containment check while being structurally meaningless.
#[test]
fn tree_with_only_files_indents_files_under_hidden_directories() {
    let fixture = fixture("tree");

    assert_eq!(
        tree_of(&fixture, &["-T", "-f"]),
        concat!(
            "        \u{2514}\u{2500}\u{2500} leaf.txt\n",
            "    \u{2514}\u{2500}\u{2500} mid.txt\n",
            "\u{2514}\u{2500}\u{2500} top.txt",
        )
    );
}

/// No row is emitted for a level whose directory was hidden, so no connector
/// may be drawn for it either — those columns have to be blank.
#[test]
fn tree_with_only_files_draws_no_connector_for_hidden_levels() {
    let fixture = fixture("tree_edges");
    let rendered = tree_of(&fixture, &["-T", "-f"]);

    for line in rendered.lines() {
        let prefix: String = line
            .chars()
            .take_while(|c| *c == ' ' || "\u{2502}\u{251c}\u{2514}\u{2500}".contains(*c))
            .collect();
        assert_eq!(
            prefix.matches('\u{2514}').count() + prefix.matches('\u{251c}').count(),
            1,
            "each row carries exactly one connector, its own: {rendered}"
        );
    }
}

/// Without `--only-files` the tree is untouched, which is what pins that the
/// blank fill above did not change ordinary rendering.
#[test]
fn tree_without_only_files_still_shows_directories() {
    let fixture = fixture("tree_plain");

    assert_eq!(
        tree_of(&fixture, &["-T"]),
        concat!(
            "<ROOT>\n",
            "\u{251c}\u{2500}\u{2500} empty_dir\n",
            "\u{251c}\u{2500}\u{2500} sub\n",
            "\u{2502}   \u{251c}\u{2500}\u{2500} deeper\n",
            "\u{2502}   \u{2502}   \u{2514}\u{2500}\u{2500} leaf.txt\n",
            "\u{2502}   \u{2514}\u{2500}\u{2500} mid.txt\n",
            "\u{2514}\u{2500}\u{2500} top.txt",
        )
    );
}

#[test]
fn recursive_lines_mode_hides_directory_entries() {
    let fixture = fixture("lines");

    let output = run_lez(&["-R", "-f", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("mid.txt"));
    assert!(stdout.contains("leaf.txt"));
    for line in stdout.lines() {
        let trimmed = line.trim();
        assert!(
            trimmed != "sub" && trimmed != "deeper" && trimmed != "empty_dir",
            "non-tree recursion must not list directory entries: {stdout}"
        );
    }
}
