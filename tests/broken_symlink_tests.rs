// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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
            "lsr_broken_symlink_{prefix}_{}_{}",
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
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

// ----------------------------------------------------------------------------
// 1. Empty-target symlinks with --group-directories-first
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_with_group_directories_first() {
    let fixture = TempTestDir::new("grp_dirs_first");
    fixture.create_dir("alpha_dir");
    fixture.create_dir("omega_dir");
    fixture.create_file("beta_file.txt", b"beta");
    fixture.create_file("psi_file.txt", b"psi");

    if fixture.create_symlink("", "empty_symlink").is_none() {
        // OS / sandbox does not support empty symlink creation
        return;
    }

    let output = run_lsr(&[
        "--group-directories-first",
        "-1",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    // Directories must appear before regular files and broken empty symlink
    let alpha_pos = lines
        .iter()
        .position(|&l| l.starts_with("alpha_dir"))
        .unwrap();
    let omega_pos = lines
        .iter()
        .position(|&l| l.starts_with("omega_dir"))
        .unwrap();
    let beta_pos = lines
        .iter()
        .position(|&l| l.starts_with("beta_file.txt"))
        .unwrap();
    let empty_pos = lines
        .iter()
        .position(|&l| l.starts_with("empty_symlink"))
        .unwrap();

    assert!(alpha_pos < beta_pos, "alpha_dir must precede beta_file.txt");
    assert!(omega_pos < beta_pos, "omega_dir must precede beta_file.txt");
    assert!(
        alpha_pos < empty_pos,
        "alpha_dir must precede empty_symlink (empty symlink is not a dir)"
    );
    assert!(
        omega_pos < empty_pos,
        "omega_dir must precede empty_symlink (empty symlink is not a dir)"
    );
}

// ----------------------------------------------------------------------------
// 2. Empty-target symlinks with --icons (must not display directory icon)
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_with_icons_does_not_display_folder_icon() {
    let fixture = TempTestDir::new("icons");
    fixture.create_dir("real_folder");

    if fixture.create_symlink("", "empty_symlink").is_none() {
        return;
    }

    let output = run_lsr(&[
        "--icons=always",
        "-1",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut found_dir_line = false;
    let mut found_empty_symlink_line = false;

    for line in stdout.lines() {
        if line.contains("real_folder") {
            found_dir_line = true;
            // Should contain folder icon:  (\u{e5ff}) or  (\u{f115})
            assert!(
                line.contains('\u{e5ff}') || line.contains('\u{f115}'),
                "real_folder line must contain directory folder icon, got: {line}"
            );
        } else if line.contains("empty_symlink") {
            found_empty_symlink_line = true;
            // Broken symlink must NOT have folder icon
            assert!(
                !line.contains('\u{e5ff}') && !line.contains('\u{f115}'),
                "empty_symlink line must NOT contain directory folder icon, got: {line}"
            );
        }
    }

    assert!(found_dir_line, "Must find real_folder in output");
    assert!(
        found_empty_symlink_line,
        "Must find empty_symlink in output"
    );
}

// ----------------------------------------------------------------------------
// 3. Empty-target symlinks with -F / --classify (must output @, not /)
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_classify_indicator() {
    let fixture = TempTestDir::new("classify");
    fixture.create_dir("real_dir");

    if fixture.create_symlink("", "empty_link").is_none() {
        return;
    }

    // In grid mode with --classify=always:
    let output = run_lsr(&[
        "-G",
        "--classify=always",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("real_dir/"),
        "real_dir should be classified with /, got: {stdout}"
    );
    assert!(
        stdout.contains("empty_link@"),
        "empty_link should be classified with @, got: {stdout}"
    );
    assert!(
        !stdout.contains("empty_link/"),
        "empty_link must NOT be classified with /"
    );
}

// ----------------------------------------------------------------------------
// 4. Empty-target symlinks with --only-dirs (-D) and --only-files (-f)
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_filtering_dirs_and_files() {
    let fixture = TempTestDir::new("filtering");
    fixture.create_dir("actual_dir");
    fixture.create_file("actual_file.txt", b"content");

    if fixture.create_symlink("", "empty_link").is_none() {
        return;
    }
    if fixture
        .create_symlink("actual_dir", "dir_symlink")
        .is_none()
    {
        return;
    }

    // Test 1: --only-dirs (-D) alone
    let out_dirs_alone = run_lsr(&["-D", "-1", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(out_dirs_alone.status.success());
    let stdout_dirs_alone = String::from_utf8_lossy(&out_dirs_alone.stdout);
    assert!(stdout_dirs_alone.contains("actual_dir"));
    assert!(!stdout_dirs_alone.contains("empty_link"));
    assert!(!stdout_dirs_alone.contains("actual_file.txt"));

    // Test 2: --only-dirs with --show-symlinks (-D --show-symlinks)
    // Points-to-directory symlink (dir_symlink) is included, broken empty symlink is excluded
    let out_dirs_show = run_lsr(&[
        "-D",
        "--show-symlinks",
        "-1",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(out_dirs_show.status.success());
    let stdout_dirs_show = String::from_utf8_lossy(&out_dirs_show.stdout);
    assert!(stdout_dirs_show.contains("actual_dir"));
    assert!(stdout_dirs_show.contains("dir_symlink"));
    assert!(
        !stdout_dirs_show.contains("empty_link"),
        "empty_link must be excluded from --only-dirs --show-symlinks because it does not point to a dir"
    );

    // Test 3: --only-files with --show-symlinks (-f --show-symlinks)
    // Regular file and broken empty symlink are included, dir_symlink is excluded
    let out_files_show = run_lsr(&[
        "-f",
        "--show-symlinks",
        "-1",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(out_files_show.status.success());
    let stdout_files_show = String::from_utf8_lossy(&out_files_show.stdout);
    assert!(stdout_files_show.contains("actual_file.txt"));
    assert!(
        stdout_files_show.contains("empty_link"),
        "empty_link must be included in --only-files --show-symlinks"
    );
    assert!(
        !stdout_files_show.contains("actual_dir"),
        "actual_dir must be excluded from --only-files"
    );
    assert!(
        !stdout_files_show.contains("dir_symlink"),
        "dir_symlink must be excluded from --only-files --show-symlinks because it points to a directory"
    );
}

// ----------------------------------------------------------------------------
// 5. Empty-target symlink passed directly as positional argument
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_as_positional_cli_argument() {
    let fixture = TempTestDir::new("positional");

    let link_path = match fixture.create_symlink("", "empty_pos_link") {
        Some(p) => p,
        None => return,
    };

    let output = run_lsr(&["--color=never", link_path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("empty_pos_link"),
        "Positional argument for empty symlink should be rendered as a single file: {stdout}"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Not a directory") && !stderr.contains("No such file or directory"),
        "Positional empty symlink must not fail directory traversal, stderr: {stderr}"
    );
}

// ----------------------------------------------------------------------------
// 6. Empty-target symlink with -X / --dereference
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_with_dereference_flag() {
    let fixture = TempTestDir::new("dereference");

    if fixture.create_symlink("", "empty_deref_link").is_none() {
        return;
    }

    let output = run_lsr(&["-l", "-X", "--color=never", fixture.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("empty_deref_link"),
        "-l -X should list empty_deref_link without crashing: {stdout}"
    );
}

// ----------------------------------------------------------------------------
// 7. Empty-target symlink in long view (-l) arrow rendering
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_long_view_arrow() {
    let fixture = TempTestDir::new("long_view");

    if fixture.create_symlink("", "empty_link").is_none() {
        return;
    }

    let output = run_lsr(&["-l", "--color=never", fixture.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("empty_link"),
        "-l output should contain empty_link: {stdout}"
    );
    assert!(
        stdout.contains("->"),
        "-l output should contain symlink arrow: {stdout}"
    );
}

// ----------------------------------------------------------------------------
// 8. Broken symlink with deleted target directory and tree view
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_deleted_dir_symlink_in_tree_mode() {
    let fixture = TempTestDir::new("deleted_tree");
    let target = fixture.create_dir("ephemeral_dir");
    fixture.create_file("ephemeral_dir/inner.txt", b"inner");

    fixture.create_symlink("ephemeral_dir", "dangling_link");

    // Remove the target directory
    let _ = fs::remove_dir_all(&target);

    let output = run_lsr(&["-T", "--color=never", fixture.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("dangling_link"),
        "Tree mode should list dangling symlink without error"
    );
}

// ----------------------------------------------------------------------------
// 9. Empty-target symlink with JSON mode
// ----------------------------------------------------------------------------
#[cfg(unix)]
#[test]
fn test_empty_symlink_in_json_mode() {
    let fixture = TempTestDir::new("json_symlink");

    if fixture.create_symlink("", "empty_json_link").is_none() {
        return;
    }

    let output = run_lsr(&["--json", fixture.path.to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Valid JSON must be produced");
    assert!(parsed.is_array(), "JSON output root is an array");
}
