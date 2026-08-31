// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Tests for `--only-files` (`-f`) when positional arguments contain mixed
//! files and directories (simulating shell wildcard expansion `*`).

use std::fs::{self, File as StdFile};
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
            "lez_only_files_wildcard_{prefix}_{}_{}",
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
        StdFile::create(&file_path).unwrap();
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

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_only_files_with_mixed_positional_files_and_dirs() {
    let temp = TempTestDir::new("mixed_wildcard");
    let file1 = temp.create_file("alpha.txt");
    let file2 = temp.create_file("beta.txt");
    let dir1 = temp.create_dir("folder1");
    let _inner1 = temp.create_file("folder1/inner1.txt");
    let dir2 = temp.create_dir("folder2");
    let _inner2 = temp.create_file("folder2/inner2.txt");

    // Simulating shell wildcard: lez -f alpha.txt beta.txt folder1 folder2
    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--color=never")
        .arg(&file1)
        .arg(&file2)
        .arg(&dir1)
        .arg(&dir2)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Files must be present
    assert!(
        stdout.contains("alpha.txt"),
        "Positional file alpha.txt must be listed: {stdout}"
    );
    assert!(
        stdout.contains("beta.txt"),
        "Positional file beta.txt must be listed: {stdout}"
    );

    // Directories and their internal contents must NOT be opened or listed
    assert!(
        !stdout.contains("folder1:"),
        "folder1 must not be opened as a directory listing: {stdout}"
    );
    assert!(
        !stdout.contains("folder2:"),
        "folder2 must not be opened as a directory listing: {stdout}"
    );
    assert!(
        !stdout.contains("inner1.txt"),
        "inner1.txt must not be listed: {stdout}"
    );
    assert!(
        !stdout.contains("inner2.txt"),
        "inner2.txt must not be listed: {stdout}"
    );
}

#[test]
fn test_only_files_with_explicit_single_directory_argument() {
    let temp = TempTestDir::new("single_dir");
    let dir = temp.create_dir("sub");
    temp.create_file("sub/file1.txt");
    temp.create_file("sub/file2.txt");
    temp.create_dir("sub/nested_dir");

    // Explicit directory argument: lez -f sub
    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--color=never")
        .arg(&dir)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("file1.txt"),
        "Files inside directory must be listed: {stdout}"
    );
    assert!(
        stdout.contains("file2.txt"),
        "Files inside directory must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("nested_dir"),
        "Subdirectories must be filtered out by -f: {stdout}"
    );
}

#[test]
fn test_only_files_with_treat_dirs_as_files_flag() {
    let temp = TempTestDir::new("treat_dirs_as_files");
    let file = temp.create_file("regular.txt");
    let dir = temp.create_dir("somedir");

    // Simulating: lez -d -f regular.txt somedir
    let output = Command::new(bin_path())
        .arg("-d")
        .arg("-f")
        .arg("--color=never")
        .arg(&file)
        .arg(&dir)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("regular.txt"),
        "regular.txt must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("somedir"),
        "somedir must be filtered out when -f is used with -d: {stdout}"
    );
}

#[test]
fn test_only_files_json_mode_with_mixed_positional() {
    let temp = TempTestDir::new("json_mixed");
    let file = temp.create_file("item.txt");
    let dir = temp.create_dir("items_dir");
    temp.create_file("items_dir/hidden_item.txt");

    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--json")
        .arg(&file)
        .arg(&dir)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("item.txt"),
        "JSON output must contain item.txt: {stdout}"
    );
    assert!(
        !stdout.contains("items_dir"),
        "JSON output must not contain items_dir: {stdout}"
    );
    assert!(
        !stdout.contains("hidden_item.txt"),
        "JSON output must not contain items_dir contents: {stdout}"
    );
}

#[test]
fn test_only_files_with_nonexistent_file_and_directory() {
    let temp = TempTestDir::new("nonexistent");
    let dir = temp.create_dir("valid_dir");
    temp.create_file("valid_dir/content.txt");
    let nonexistent = temp.path.join("does_not_exist.txt");

    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--color=never")
        .arg(&nonexistent)
        .arg(&dir)
        .output()
        .expect("Failed to run lez");

    // Exit code 2 because of nonexistent path
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stderr.contains("does_not_exist.txt"),
        "Stderr must report nonexistent file: {stderr}"
    );
    assert!(
        stdout.contains("content.txt"),
        "Stdout must list files inside valid_dir: {stdout}"
    );
}

#[test]
fn test_only_files_with_multiple_explicit_directory_arguments() {
    let temp = TempTestDir::new("multi_dir");
    let dir1 = temp.create_dir("dir1");
    let dir2 = temp.create_dir("dir2");
    temp.create_file("dir1/file1.txt");
    temp.create_file("dir2/file2.txt");

    let output = Command::new(bin_path())
        .arg("-f")
        .arg("--color=never")
        .arg(&dir1)
        .arg(&dir2)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("file1.txt"),
        "file1.txt inside dir1 must be listed: {stdout}"
    );
    assert!(
        stdout.contains("file2.txt"),
        "file2.txt inside dir2 must be listed: {stdout}"
    );
}

#[test]
fn test_only_files_with_ignore_glob_on_mixed_positional() {
    let temp = TempTestDir::new("ignore_glob_mixed");
    let f_keep = temp.create_file("keep.txt");
    let f_ignore = temp.create_file("skip.tmp");
    let dir = temp.create_dir("subdir");
    temp.create_file("subdir/nested.txt");

    // Simulating wildcard: lez -f -I "*.tmp" keep.txt skip.tmp subdir
    let output = Command::new(bin_path())
        .arg("-f")
        .arg("-I")
        .arg("*.tmp")
        .arg("--color=never")
        .arg(&f_keep)
        .arg(&f_ignore)
        .arg(&dir)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("keep.txt"),
        "keep.txt must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("skip.tmp"),
        "skip.tmp must be filtered by -I: {stdout}"
    );
    assert!(
        !stdout.contains("subdir"),
        "subdir must be suppressed: {stdout}"
    );
    assert!(
        !stdout.contains("nested.txt"),
        "subdir contents must not be listed: {stdout}"
    );
}

#[test]
fn test_composed_only_files_and_no_symlinks_child_files() {
    let temp = TempTestDir::new("composed_only_files_no_symlinks");
    temp.create_file("regular.txt");
    temp.create_dir("subdir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink("subdir", temp.path.join("dir_link"));
        let _ = symlink("regular.txt", temp.path.join("file_link"));
    }

    let output = Command::new(bin_path())
        .arg("--only-files")
        .arg("--no-symlinks")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("regular.txt"),
        "regular.txt must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("subdir"),
        "subdir must be filtered out: {stdout}"
    );
    #[cfg(unix)]
    {
        assert!(
            !stdout.contains("dir_link"),
            "dir_link must be filtered out by --no-symlinks: {stdout}"
        );
        assert!(
            !stdout.contains("file_link"),
            "file_link must be filtered out by --no-symlinks: {stdout}"
        );
    }
}

#[test]
fn test_composed_only_files_and_no_symlinks_with_d_flag_positional() {
    let temp = TempTestDir::new("composed_positional_d_flag");
    let file = temp.create_file("file.txt");
    let dir = temp.create_dir("target_subdir");
    #[cfg(unix)]
    let link = {
        use std::os::unix::fs::symlink;
        let p = temp.path.join("symlink_to_file");
        let _ = symlink("file.txt", &p);
        p
    };

    let mut cmd = Command::new(bin_path());
    cmd.arg("-d")
        .arg("--only-files")
        .arg("--no-symlinks")
        .arg("--color=never")
        .arg(&file)
        .arg(&dir);

    #[cfg(unix)]
    cmd.arg(&link);

    let output = cmd.output().expect("Failed to run lez");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("file.txt"),
        "file.txt must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("target_subdir"),
        "target_subdir must be filtered out by --only-files: {stdout}"
    );
    #[cfg(unix)]
    {
        assert!(
            !stdout.contains("symlink_to_file"),
            "symlink_to_file must be filtered out by --no-symlinks: {stdout}"
        );
    }
}

#[test]
fn test_composed_only_dirs_and_no_symlinks() {
    let temp = TempTestDir::new("composed_only_dirs_no_symlinks");
    temp.create_file("regular.txt");
    temp.create_dir("target_subdir");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink("target_subdir", temp.path.join("link_to_subdir"));
    }

    let output = Command::new(bin_path())
        .arg("--only-dirs")
        .arg("--no-symlinks")
        .arg("--color=never")
        .arg(&temp.path)
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("target_subdir"),
        "target_subdir must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("regular.txt"),
        "regular.txt must be filtered out by --only-dirs: {stdout}"
    );
    #[cfg(unix)]
    {
        assert!(
            !stdout.contains("link_to_subdir"),
            "link_to_subdir must be filtered out by --no-symlinks: {stdout}"
        );
    }
}
