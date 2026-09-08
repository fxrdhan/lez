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
            "lez_test_summary_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[cfg(unix)]
    fn create_symlink<P: AsRef<Path>, Q: AsRef<Path>>(&self, original: P, link: Q) -> PathBuf {
        let link_path = self.path.join(link);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        std::os::unix::fs::symlink(original, &link_path).expect("Failed to create symlink");
        link_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

// ---------------------------------------------------------------------------
// 1. EMPTY DIRECTORY SUMMARY
// ---------------------------------------------------------------------------

#[test]
fn test_summary_empty_directory() {
    let temp = TempTestDir::new("empty_dir");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "0 directories, 0 files, 0 symlinks (0 total)"
    );
}

// ---------------------------------------------------------------------------
// 2. MIXED ENTRIES AND PLURALIZATION
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_mixed_plural() {
    let temp = TempTestDir::new("mixed_plural");
    temp.create_dir("dir1");
    temp.create_dir("dir2");
    temp.create_file("file1.txt", b"1");
    temp.create_file("file2.txt", b"2");
    temp.create_file("file3.txt", b"3");
    temp.create_symlink("file1.txt", "link1");
    temp.create_symlink("file2.txt", "link2");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "2 directories, 3 files, 2 symlinks (7 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_singular() {
    let temp = TempTestDir::new("singular");
    temp.create_dir("single_dir");
    temp.create_file("single_file.txt", b"hello");
    temp.create_symlink("single_file.txt", "single_link");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "1 directory, 1 file, 1 symlink (3 total)"
    );
}

// ---------------------------------------------------------------------------
// 3. FLAT VIEW MODES (Grid, Lines, Details, GridDetails)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_oneline_mode() {
    let temp = TempTestDir::new("oneline_mode");
    temp.create_dir("folder");
    temp.create_file("a.txt", b"a");
    temp.create_symlink("a.txt", "link_a");

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 4);
    assert_eq!(
        lines.last().unwrap().trim(),
        "1 directory, 1 file, 1 symlink (3 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_long_details_mode() {
    let temp = TempTestDir::new("long_mode");
    temp.create_dir("docs");
    temp.create_file("readme.md", b"hello");
    temp.create_symlink("docs", "docs_link");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.len() >= 4);
    assert_eq!(
        lines.last().unwrap().trim(),
        "1 directory, 1 file, 1 symlink (3 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_grid_details_mode() {
    let temp = TempTestDir::new("grid_details_mode");
    temp.create_dir("src");
    temp.create_file("main.rs", b"fn main() {}");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-G")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "1 directory, 1 file, 0 symlinks (2 total)"
    );
}

// ---------------------------------------------------------------------------
// 4. RECURSIVE TREE MODE
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_tree_mode_nested() {
    let temp = TempTestDir::new("tree_nested");
    temp.create_dir("parent_dir");
    temp.create_dir("parent_dir/child_dir");
    temp.create_file("parent_dir/child_dir/nested_file.txt", b"nested");
    temp.create_file("parent_dir/root_file.txt", b"root");
    temp.create_symlink("parent_dir/root_file.txt", "parent_dir/link_to_root");

    // Tree without long details (-T --summary)
    let output = Command::new(bin_path())
        .arg("-T")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    // Under temp.path (including root dir in tree):
    // 3 dirs: temp.path, parent_dir, parent_dir/child_dir
    // 2 files: root_file.txt, nested_file.txt
    // 1 symlink: link_to_root
    // total: 6
    assert_eq!(
        summary_line.trim(),
        "3 directories, 2 files, 1 symlink (6 total)"
    );

    // Tree with long details (-l -T --summary)
    let output_long = Command::new(bin_path())
        .arg("-l")
        .arg("-T")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    let summary_line_long = stdout_long.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line_long.trim(),
        "3 directories, 2 files, 1 symlink (6 total)"
    );
}

// ---------------------------------------------------------------------------
// 5. FILTERS (only-dirs, only-files, no-symlinks, ignore-glob)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_with_only_dirs() {
    let temp = TempTestDir::new("only_dirs");
    temp.create_dir("dirA");
    temp.create_dir("dirB");
    temp.create_file("fileA.txt", b"a");
    temp.create_symlink("fileA.txt", "linkA");

    let output = Command::new(bin_path())
        .arg("-D")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "2 directories, 0 files, 0 symlinks (2 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_with_only_files() {
    let temp = TempTestDir::new("only_files");
    temp.create_dir("dirA");
    temp.create_file("fileA.txt", b"a");
    temp.create_file("fileB.txt", b"b");
    temp.create_symlink("fileA.txt", "linkA");

    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "0 directories, 2 files, 0 symlinks (2 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_with_no_symlinks() {
    let temp = TempTestDir::new("no_symlinks");
    temp.create_dir("dir1");
    temp.create_file("file1.txt", b"a");
    temp.create_symlink("file1.txt", "link1");

    let output = Command::new(bin_path())
        .arg("--no-symlinks")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "1 directory, 1 file, 0 symlinks (2 total)"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_with_ignore_glob() {
    let temp = TempTestDir::new("ignore_glob");
    temp.create_dir("dir1");
    temp.create_file("file1.txt", b"a");
    temp.create_file("skip.log", b"log");

    let output = Command::new(bin_path())
        .arg("-I")
        .arg("*.log")
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "1 directory, 1 file, 0 symlinks (2 total)"
    );
}

// ---------------------------------------------------------------------------
// 6. ICONS RENDERING (Enabled vs Disabled)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_with_icons_always() {
    let temp = TempTestDir::new("icons_always");
    temp.create_dir("my_dir");
    temp.create_file("my_file.txt", b"content");
    temp.create_symlink("my_file.txt", "my_link");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--icons=always")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");

    // Icons: folder is \u{e5ff}, file is \u{f15b}, link is \u{f481}
    assert!(
        summary_line.contains('\u{e5ff}'),
        "Summary line must contain directory icon \\u{{e5ff}}: {summary_line}"
    );
    assert!(
        summary_line.contains('\u{f15b}'),
        "Summary line must contain file icon \\u{{f15b}}: {summary_line}"
    );
    assert!(
        summary_line.contains('\u{f481}'),
        "Summary line must contain symlink icon \\u{{f481}}: {summary_line}"
    );
    assert!(
        summary_line.contains("1 directory"),
        "Summary line must contain '1 directory': {summary_line}"
    );
    assert!(
        summary_line.contains("1 file"),
        "Summary line must contain '1 file': {summary_line}"
    );
    assert!(
        summary_line.contains("1 symlink"),
        "Summary line must contain '1 symlink': {summary_line}"
    );
    assert!(
        summary_line.contains("(3 total)"),
        "Summary line must contain '(3 total)': {summary_line}"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_with_icons_never() {
    let temp = TempTestDir::new("icons_never");
    temp.create_dir("my_dir");
    temp.create_file("my_file.txt", b"content");
    temp.create_symlink("my_file.txt", "my_link");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--icons=never")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");

    assert!(
        !summary_line.contains('\u{e5ff}'),
        "Summary line must NOT contain directory icon when icons=never: {summary_line}"
    );
    assert!(
        !summary_line.contains('\u{f15b}'),
        "Summary line must NOT contain file icon when icons=never: {summary_line}"
    );
    assert!(
        !summary_line.contains('\u{f481}'),
        "Summary line must NOT contain symlink icon when icons=never: {summary_line}"
    );
    assert_eq!(
        summary_line.trim(),
        "1 directory, 1 file, 1 symlink (3 total)"
    );
}

// ---------------------------------------------------------------------------
// 7. MULTI-DIRECTORY ARGUMENTS
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn test_summary_multi_directory_arguments() {
    let temp1 = TempTestDir::new("multi_dir1");
    temp1.create_dir("sub1");
    temp1.create_file("file1.txt", b"1");

    let temp2 = TempTestDir::new("multi_dir2");
    temp2.create_file("file2a.txt", b"2a");
    temp2.create_file("file2b.txt", b"2b");
    temp2.create_symlink("file2a.txt", "link2");

    let output = Command::new(bin_path())
        .arg("--summary")
        .arg("--color=never")
        .arg(&temp1.path)
        .arg(&temp2.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Both summaries should appear in the output
    assert!(
        stdout.contains("1 directory, 1 file, 0 symlinks (2 total)"),
        "Output must contain summary for dir 1: {stdout}"
    );
    assert!(
        stdout.contains("0 directories, 2 files, 1 symlink (3 total)"),
        "Output must contain summary for dir 2: {stdout}"
    );
}

#[test]
#[cfg(unix)]
fn test_summary_tree_mode_with_only_files() {
    let temp = TempTestDir::new("tree_only_files");
    temp.create_dir("parent_dir");
    temp.create_dir("parent_dir/child_dir");
    temp.create_file("parent_dir/child_dir/nested_file.txt", b"nested");
    temp.create_file("parent_dir/root_file.txt", b"root");
    temp.create_symlink("parent_dir/root_file.txt", "parent_dir/link_to_root");

    // -T -f --summary: container directories must not be in summary
    let output = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--summary",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let summary_line = stdout.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line.trim(),
        "0 directories, 2 files, 1 symlink (3 total)"
    );

    // -T -f --print-total: total must reflect only displayed entries (3)
    let output_total = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_total.status.success());
    let stdout_total = String::from_utf8_lossy(&output_total.stdout);
    let total_line = stdout_total.lines().last().expect("last line is total");
    assert_eq!(total_line.trim(), "total: 3");

    // Long tree: -l -T -f --summary
    let output_long = Command::new(bin_path())
        .args([
            "-l",
            "-T",
            "-f",
            "--summary",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    let summary_line_long = stdout_long.lines().last().expect("last line is summary");
    assert_eq!(
        summary_line_long.trim(),
        "0 directories, 2 files, 1 symlink (3 total)"
    );

    // Long tree: -l -T -f --print-total
    let output_long_total = Command::new(bin_path())
        .args([
            "-l",
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_long_total.status.success());
    let stdout_long_total = String::from_utf8_lossy(&output_long_total.stdout);
    let total_line_long = stdout_long_total
        .lines()
        .last()
        .expect("last line is total");
    assert_eq!(total_line_long.trim(), "total: 3");
}

#[test]
fn test_summary_tree_mode_with_only_files_empty_and_dirs_only() {
    let temp = TempTestDir::new("tree_dirs_only");
    temp.create_dir("d1");
    temp.create_dir("d1/d2");
    temp.create_dir("d1/d2/d3");

    // Directory with only subdirectories
    let output = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--summary",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "0 directories, 0 files, 0 symlinks (0 total)"
    );

    let output_total = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_total.status.success());
    let stdout_total = String::from_utf8_lossy(&output_total.stdout);
    assert_eq!(stdout_total.trim(), "total: 0");

    // Empty directory
    let temp_empty = TempTestDir::new("tree_empty");
    let output_empty = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--summary",
            "--color=never",
            temp_empty.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_empty.status.success());
    let stdout_empty = String::from_utf8_lossy(&output_empty.stdout);
    assert_eq!(
        stdout_empty.trim(),
        "0 directories, 0 files, 0 symlinks (0 total)"
    );

    let output_empty_total = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            temp_empty.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");

    assert!(output_empty_total.status.success());
    let stdout_empty_total = String::from_utf8_lossy(&output_empty_total.stdout);
    assert_eq!(stdout_empty_total.trim(), "total: 0");
}

#[test]
#[cfg(unix)]
fn test_summary_tree_mode_with_level_and_multi_dirs() {
    let temp = TempTestDir::new("tree_level_multi");
    let dir1 = temp.path.join("dir1");
    let dir2 = temp.path.join("dir2");
    std::fs::create_dir_all(dir1.join("sub_lvl2").join("sub_lvl3")).unwrap();
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir1.join("f1.txt"), b"1").unwrap();
    std::fs::write(dir1.join("sub_lvl2").join("f2.txt"), b"2").unwrap();
    std::fs::write(dir1.join("sub_lvl2").join("sub_lvl3").join("f3.txt"), b"3").unwrap();
    std::os::unix::fs::symlink(dir1.join("f1.txt"), dir1.join("link_to_file")).unwrap();
    std::fs::write(dir2.join("f4.txt"), b"4").unwrap();

    // -T -f -L 2 --summary dir1: stops at depth 2, f3.txt is excluded
    let output_l2 = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "-L",
            "2",
            "--summary",
            "--color=never",
            dir1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");
    assert!(output_l2.status.success());
    let stdout_l2 = String::from_utf8_lossy(&output_l2.stdout);
    let summary_l2 = stdout_l2.lines().last().expect("last line is summary");
    assert_eq!(
        summary_l2.trim(),
        "0 directories, 2 files, 1 symlink (3 total)"
    );

    // -T -f -L 2 --print-total dir1
    let output_l2_tot = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "-L",
            "2",
            "--print-total",
            "--color=never",
            dir1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");
    assert!(output_l2_tot.status.success());
    let stdout_l2_tot = String::from_utf8_lossy(&output_l2_tot.stdout);
    let total_l2 = stdout_l2_tot.lines().last().expect("last line is total");
    assert_eq!(total_l2.trim(), "total: 3");

    // -T -f -L 1 --summary dir1: only depth 1 items
    let output_l1 = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "-L",
            "1",
            "--summary",
            "--color=never",
            dir1.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");
    assert!(output_l1.status.success());
    let stdout_l1 = String::from_utf8_lossy(&output_l1.stdout);
    let summary_l1 = stdout_l1.lines().last().expect("last line is summary");
    assert_eq!(
        summary_l1.trim(),
        "0 directories, 1 file, 1 symlink (2 total)"
    );

    // -T -f --summary dir1 dir2: multi-directory accumulation
    let output_multi = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--summary",
            "--color=never",
            dir1.to_str().unwrap(),
            dir2.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");
    assert!(output_multi.status.success());
    let stdout_multi = String::from_utf8_lossy(&output_multi.stdout);
    let summary_multi = stdout_multi.lines().last().expect("last line is summary");
    assert_eq!(
        summary_multi.trim(),
        "0 directories, 4 files, 1 symlink (5 total)"
    );

    // -T -f --print-total dir1 dir2
    let output_multi_tot = Command::new(bin_path())
        .args([
            "-T",
            "-f",
            "--print-total",
            "--color=never",
            dir1.to_str().unwrap(),
            dir2.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lez");
    assert!(output_multi_tot.status.success());
    let stdout_multi_tot = String::from_utf8_lossy(&output_multi_tot.stdout);
    let total_multi = stdout_multi_tot.lines().last().expect("last line is total");
    assert_eq!(total_multi.trim(), "total: 5");
}
