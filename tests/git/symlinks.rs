// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            eprintln!("git not available; skipping");
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_git_symlink_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp git repo root");

        let repo = Self { path };
        if !repo.git(&["init", "-q"]) {
            return None;
        }
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        Some(repo)
    }

    fn write_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel_path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    #[cfg(unix)]
    fn create_symlink(&self, target: &str, link_rel_path: &str) -> Option<PathBuf> {
        let link_path = self.path.join(link_rel_path);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if std::os::unix::fs::symlink(target, &link_path).is_ok() {
            Some(link_path)
        } else {
            None
        }
    }

    fn git(&self, args: &[&str]) -> bool {
        let output = Command::new("git")
            .args(
                [
                    "-c",
                    "user.name=Test User",
                    "-c",
                    "user.email=test@example.com",
                ]
                .iter()
                .chain(args.iter()),
            )
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to spawn git");
        if !output.status.success() {
            eprintln!(
                "git {:?} failed: {:?}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        output.status.success()
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run_lez(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

// ----------------------------------------------------------------------------
// 1. Modified symlink pointing to unchanged target file
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_modified_symlink_pointing_to_unchanged_file() {
    let Some(repo) = TempGitRepo::new("mod_symlink") else {
        return;
    };

    repo.write_file("target1.txt", b"target 1 content\n");
    repo.write_file("target2.txt", b"target 2 content\n");
    let link_path = repo.create_symlink("target1.txt", "link.txt").unwrap();

    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    // Modify the symlink to point to target2.txt without changing target1.txt
    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("target2.txt", "link.txt").unwrap();

    let output = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the line for link.txt and target1.txt
    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in output");
    let target1_line = stdout
        .lines()
        .find(|l| l.contains("target1.txt"))
        .expect("target1.txt in output");

    // link.txt should report modified in working tree (-M)
    assert!(
        link_line.contains("-M"),
        "Modified symlink must report -M status, got line: {link_line}"
    );

    // target1.txt should report clean (--)
    assert!(
        target1_line.contains("--"),
        "Unchanged target file must report -- status, got line: {target1_line}"
    );
}

// ----------------------------------------------------------------------------
// 2. Unmodified symlink pointing to modified target file
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_unmodified_symlink_pointing_to_modified_file() {
    let Some(repo) = TempGitRepo::new("unmod_symlink") else {
        return;
    };

    let target_path = repo.write_file("target.txt", b"initial content\n");
    repo.create_symlink("target.txt", "link.txt").unwrap();

    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    // Modify the target file, keep symlink untouched
    fs::write(&target_path, b"modified content\n").unwrap();

    let output = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let link_line = stdout
        .lines()
        .find(|l| l.contains("->") && l.contains("link.txt"))
        .expect("link.txt symlink in output");
    let target_line = stdout
        .lines()
        .find(|l| !l.contains("->") && l.contains("target.txt"))
        .expect("target.txt regular file in output");

    // link.txt should report clean (--)
    assert!(
        link_line.contains("--"),
        "Unmodified symlink must report -- status even when target is modified, got line: {link_line}"
    );

    // target.txt should report modified (-M)
    assert!(
        target_line.contains("-M"),
        "Modified target file must report -M status, got line: {target_line}"
    );
}

// ----------------------------------------------------------------------------
// 3. Broken symlink pointing to non-existent target reports status without panicking
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_broken_symlink_git_status_without_panicking() {
    let Some(repo) = TempGitRepo::new("broken_symlink") else {
        return;
    };

    let link_path = repo
        .create_symlink("non_existent_file.txt", "broken.txt")
        .unwrap();

    // Untracked broken symlink
    let output = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let link_line = stdout
        .lines()
        .find(|l| l.contains("broken.txt"))
        .expect("broken.txt in output");
    assert!(
        link_line.contains("-N"),
        "Untracked broken symlink must report -N, got line: {link_line}"
    );

    // Commit the broken symlink
    assert!(repo.git(&["add", "broken.txt"]));
    assert!(repo.git(&["commit", "-q", "-m", "add broken symlink"]));

    let output_committed = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output_committed.status.success());
    let stdout_committed = String::from_utf8_lossy(&output_committed.stdout);
    let link_line_committed = stdout_committed
        .lines()
        .find(|l| l.contains("broken.txt"))
        .expect("broken.txt in output");
    assert!(
        link_line_committed.contains("--"),
        "Committed broken symlink must report --, got line: {link_line_committed}"
    );

    // Change target to another non-existent file
    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("another_missing.txt", "broken.txt")
        .unwrap();

    let output_modified = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output_modified.status.success());
    let stdout_modified = String::from_utf8_lossy(&output_modified.stdout);
    let link_line_modified = stdout_modified
        .lines()
        .find(|l| l.contains("broken.txt"))
        .expect("broken.txt in output");
    assert!(
        link_line_modified.contains("-M"),
        "Modified broken symlink must report -M, got line: {link_line_modified}"
    );
}

// ----------------------------------------------------------------------------
// 4. Staged symlink additions, modifications, and deletions
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_staged_symlink_additions_modifications_and_deletions() {
    let Some(repo) = TempGitRepo::new("staged_symlink") else {
        return;
    };

    repo.write_file("target1.txt", b"content 1\n");
    repo.write_file("target2.txt", b"content 2\n");
    let link_path = repo.create_symlink("target1.txt", "link.txt").unwrap();

    // 4.1 Staged addition
    assert!(repo.git(&["add", "target1.txt", "target2.txt", "link.txt"]));

    let output_staged_add = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output_staged_add.status.success());
    let stdout_add = String::from_utf8_lossy(&output_staged_add.stdout);
    let link_line_add = stdout_add
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in output");
    assert!(
        link_line_add.contains("N-"),
        "Staged new symlink must report N-, got line: {link_line_add}"
    );

    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    // 4.2 Staged modification
    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("target2.txt", "link.txt").unwrap();
    assert!(repo.git(&["add", "link.txt"]));

    let output_staged_mod = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(output_staged_mod.status.success());
    let stdout_mod = String::from_utf8_lossy(&output_staged_mod.stdout);
    let link_line_mod = stdout_mod
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in output");
    assert!(
        link_line_mod.contains("M-"),
        "Staged modified symlink must report M-, got line: {link_line_mod}"
    );

    assert!(repo.git(&["commit", "-q", "-m", "update symlink"]));

    // 4.3 Staged deletion
    fs::remove_file(&link_path).unwrap();
    assert!(repo.git(&["add", "link.txt"]));

    // In JSON mode or git queries, verify deletion status
    let json_output = run_lez(&["--json", "-l", "--git", repo.path.to_str().unwrap()]);
    assert!(json_output.status.success());
}

// ----------------------------------------------------------------------------
// 5. Nested symlink inside subdirectory
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_nested_symlink_in_subdirectory() {
    let Some(repo) = TempGitRepo::new("nested_symlink") else {
        return;
    };

    let target_path = repo.write_file("data/source.txt", b"source data\n");
    let link_path = repo
        .create_symlink("data/source.txt", "nested/link.txt")
        .unwrap();

    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    // Modify the symlink target
    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("data/other.txt", "nested/link.txt")
        .unwrap();

    // Query inside nested directory directly
    let nested_dir = repo.path.join("nested");
    let output = run_lez(&["-l", "--git", "--color=never", nested_dir.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let link_line = stdout
        .lines()
        .find(|l| l.contains("link.txt"))
        .expect("link.txt in output");
    assert!(
        link_line.contains("-M"),
        "Nested modified symlink must report -M when querying subdirectory, got line: {link_line}"
    );

    // Target remains clean
    let data_dir = repo.path.join("data");
    let output_data = run_lez(&["-l", "--git", "--color=never", data_dir.to_str().unwrap()]);
    assert!(output_data.status.success());
    let stdout_data = String::from_utf8_lossy(&output_data.stdout);
    let target_line = stdout_data
        .lines()
        .find(|l| !l.contains("->") && l.contains("source.txt"))
        .expect("source.txt in output");
    assert!(
        target_line.contains("--"),
        "Target file in data directory must report --, got line: {target_line}"
    );

    let _ = target_path;
}

// ----------------------------------------------------------------------------
// 6. JSON output mode for symlink git statuses
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_json_mode_symlink_git_status() {
    let Some(repo) = TempGitRepo::new("json_symlink_git") else {
        return;
    };

    repo.write_file("real_file.txt", b"content\n");
    let link_path = repo.create_symlink("real_file.txt", "sym.txt").unwrap();

    assert!(repo.git(&["add", "."]));
    assert!(repo.git(&["commit", "-q", "-m", "init"]));

    // Modify symlink
    fs::remove_file(&link_path).unwrap();
    repo.create_symlink("other_file.txt", "sym.txt").unwrap();

    let output = run_lez(&["--json", "-l", "--git", repo.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Valid JSON");

    let sym_entry = if parsed.is_object() {
        parsed
            .get("sym.txt")
            .expect("sym.txt in JSON object")
            .clone()
    } else if let Some(arr) = parsed.as_array() {
        arr.iter()
            .find(|e| e.get("name").and_then(|n| n.as_str()) == Some("sym.txt"))
            .expect("sym.txt in JSON array")
            .clone()
    } else {
        panic!("Unexpected JSON root structure: {stdout}");
    };

    let git_field = sym_entry
        .get("Git")
        .or_else(|| sym_entry.get("git"))
        .expect("git field present");
    let git_str = git_field.as_str().unwrap_or("");
    assert_eq!(
        git_str, "-M",
        "JSON git status for modified symlink must be '-M', got: {git_str}"
    );
}
