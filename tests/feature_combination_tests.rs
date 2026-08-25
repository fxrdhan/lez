// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Curated cross-feature end-to-end scenarios for the batch-two port set:
//! boundary cases, flag combinations, and realistic workloads that go beyond
//! the per-feature suites. Git-dependent cases skip silently when the `git`
//! binary is unavailable.

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
            "lez_combo_{prefix}_{}_{}",
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

fn write_file_at(base: &Path, rel: &str, content: &[u8]) -> PathBuf {
    let path = base.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = StdFile::create(&path).unwrap();
    file.write_all(content).unwrap();
    path
}

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_combo_git_{prefix}_{}_{}",
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
        output.status.success()
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct TempRepoFixture {
    path: PathBuf,
}

impl TempRepoFixture {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_combo_master_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create master fixture root");

        let repo = Self { path };
        if !repo.git(&["init", "-q"]) {
            return None;
        }
        repo.git(&["config", "user.name", "Master Tester"]);
        repo.git(&["config", "user.email", "master@tester.com"]);
        Some(repo)
    }

    fn write_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    #[cfg(unix)]
    fn create_symlink(&self, target: &str, link_rel: &str) -> Option<PathBuf> {
        let link_p = self.path.join(link_rel);
        if let Some(parent) = link_p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if std::os::unix::fs::symlink(target, &link_p).is_ok() {
            Some(link_p)
        } else {
            None
        }
    }

    fn git(&self, args: &[&str]) -> bool {
        let output = Command::new("git")
            .args(
                [
                    "-c",
                    "user.name=Master Tester",
                    "-c",
                    "user.email=master@tester.com",
                ]
                .iter()
                .chain(args.iter()),
            )
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to spawn git");
        output.status.success()
    }
}

impl Drop for TempRepoFixture {
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

// ====================================================================================
// Boundary cases
// ====================================================================================

#[test]
fn dotgit_directory_in_non_git_parent_is_plain() {
    let temp = TempTestDir::new("f2_orphan_dotgit");
    temp.create_dir(".git");
    let output = run_lez(&[
        "-la",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn worktree_with_detached_head() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_detached");
    let main_path = temp.create_dir("main");
    let wt_path = temp.path.join("wt");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "add", "--detach", wt_path.to_str().unwrap()])
        .current_dir(&main_path)
        .output();

    let output = run_lez(&[
        "-l",
        "--git-repos",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn nested_worktrees_resolve_branches() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("f3_deep_wt");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("deep/nested/wt_folder");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["worktree", "add", "-b", "deep-b", wt_path.to_str().unwrap()])
        .current_dir(&main_path)
        .output();

    let output = run_lez(&[
        "-l",
        "--git-repos",
        "--color=never",
        wt_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn sort_none_preserves_argument_order() {
    let temp = TempTestDir::new("f4_none");
    let fz = temp.create_file("z.txt", b"z");
    let fa = temp.create_file("a.txt", b"a");
    let fm = temp.create_file("m.txt", b"m");

    let output = run_lez(&[
        "-1d",
        "--sort=none",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
        fm.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("z.txt"));
    assert!(lines[1].contains("a.txt"));
    assert!(lines[2].contains("m.txt"));
}

#[test]
fn nonexistent_positional_arguments_report_error() {
    let temp = TempTestDir::new("f4_missing");
    let f1 = temp.create_file("valid1.txt", b"1");
    let f2 = temp.create_file("valid2.txt", b"2");
    let missing = temp.path.join("nonexistent.txt");

    let output = run_lez(&[
        "-1d",
        "--color=never",
        f2.to_str().unwrap(),
        missing.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    // Code 2 for missing file
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid1.txt"));
    assert!(stdout.contains("valid2.txt"));
}

#[test]
fn positional_argument_case_sensitivity() {
    let temp = TempTestDir::new("f4_case");
    let fb = temp.create_file("file_B.txt", b"B");
    let fa = temp.create_file("file_a.txt", b"a");
    let fa_cap = temp.create_file("file_A.txt", b"A");

    let output = run_lez(&[
        "-1d",
        "--color=never",
        fb.to_str().unwrap(),
        fa.to_str().unwrap(),
        fa_cap.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn sort_path_with_special_characters() {
    let temp = TempTestDir::new("f5_special");
    let f1 = temp.create_file("dir-1/file_a.txt", b"1");
    let f2 = temp.create_file("dir_2/file-b.txt", b"2");
    let f3 = temp.create_file("dir 3/file c.txt", b"3");

    let output = run_lez(&[
        "-1d",
        "--sort=path",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn sort_path_in_json_mode() {
    let temp = TempTestDir::new("f5_json");
    temp.create_file("b/1.txt", b"1");
    temp.create_file("a/2.txt", b"2");

    let output = run_lez(&["--json", "--sort=path", temp.path.to_str().unwrap()]);
    assert!(output.status.success());
}

#[test]
fn path_glob_matches_spaces_and_unicode() {
    let temp = TempTestDir::new("f6_unicode");
    temp.create_file("my docs/file.pdf", b"pdf");
    temp.create_file("my docs/file.txt", b"txt");

    let output = run_lez(&[
        "-1",
        "--tree",
        "-I",
        "my docs/*.pdf",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("file.pdf"));
    assert!(stdout.contains("file.txt"));
}

#[test]
fn nested_gitignore_applies_within_explicit_dir() {
    let Some(repo) = TempGitRepo::new("f7_nested_ignore") else {
        return;
    };
    repo.write_file(".gitignore", b"build/\n");
    repo.write_file("build/.gitignore", b"cache.dat\n");
    repo.write_file("build/output.bin", b"bin\n");
    repo.write_file("build/cache.dat", b"cache\n");

    let build_dir = repo.path.join("build");
    let output = run_lez(&[
        "-1",
        "--git-ignore",
        "--color=never",
        build_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("output.bin"));
}

#[test]
fn explicit_gitignored_dir_in_json_mode() {
    let Some(repo) = TempGitRepo::new("f7_json_explicit") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/build.log", b"log\n");

    let target_dir = repo.path.join("target");
    let output = run_lez(&["--json", "--git-ignore", target_dir.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("build.log"));
}

#[test]
fn git_glyphs_in_json_mode() {
    let Some(repo) = TempGitRepo::new("f8_json_glyphs") else {
        return;
    };
    repo.write_file("f.txt", b"1\n");
    let output = run_lez(&[
        "--json",
        "-l",
        "--git",
        "--git-glyphs",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

// ====================================================================================
// Cross-feature flag combinations
// ====================================================================================

#[test]
fn combined_path_sort_and_ignore_glob() {
    let temp = TempTestDir::new("t3_sortpath_glob");
    let f1 = temp.create_file("src/a.rs", b"a");
    let _f2 = temp.create_file("src/skip.tmp", b"skip");
    let f3 = temp.create_file("src/z.rs", b"z");

    let output = run_lez(&[
        "-1d",
        "--sort=path",
        "-I",
        "src/*.tmp",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("skip.tmp"));
    assert!(
        stdout.contains("src/a.rs") || stdout.contains(r"src\a.rs"),
        "path-sorted entry must be listed: {stdout}"
    );
}

#[test]
fn glyphs_column_besides_repos_branch() {
    let Some(repo) = TempGitRepo::new("t3_glyphs_repos") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");
    let output = run_lez(&[
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".git"));
}

#[test]
fn no_git_overrides_gitignore_for_positionals() {
    let Some(repo) = TempGitRepo::new("t3_nogit_ignore_pos") else {
        return;
    };
    repo.write_file(".gitignore", b"ignored_z.txt\nignored_a.txt\n");
    let fz = repo.write_file("ignored_z.txt", b"z\n");
    let fa = repo.write_file("ignored_a.txt", b"a\n");

    let output = run_lez(&[
        "-1d",
        "--git-ignore",
        "--no-git",
        "--color=never",
        fz.to_str().unwrap(),
        fa.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("ignored_a.txt"));
    assert!(lines[1].contains("ignored_z.txt"));
}

#[cfg(unix)]
#[test]
fn symlink_status_inside_worktree() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_sym_wt");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("wt");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-sym-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    fs::write(wt_path.join("target.txt"), b"target\n").unwrap();
    let _ = std::os::unix::fs::symlink("target.txt", wt_path.join("link.txt"));

    let output = run_lez(&["-l", "--git", "--color=never", wt_path.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link.txt"));
}

#[test]
fn worktree_root_hides_dotgit_subrepo() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_dotgit");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("wt");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("file.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "file.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-excl-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lez(&[
        "-la",
        "--git-repos",
        "--color=never",
        wt_path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn ignore_glob_with_explicit_gitignored_dir() {
    let Some(repo) = TempGitRepo::new("t3_glob_gitignored") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\n");
    repo.write_file("target/build.log", b"log\n");
    repo.write_file("target/keep.bin", b"bin\n");

    let target_dir = repo.path.join("target");
    let output = run_lez(&[
        "-1",
        "--git-ignore",
        "-I",
        "*.log",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("build.log"));
    assert!(stdout.contains("keep.bin"));
}

#[test]
fn relative_path_sort_reversed() {
    let temp = TempTestDir::new("t3_relpath_rev");
    let f1 = temp.create_file("dir_z/file.txt", b"z");
    let f2 = temp.create_file("dir_a/file.txt", b"a");

    let output = run_lez(&[
        "-1d",
        "--sort=relative-path",
        "-r",
        "--color=never",
        f2.to_str().unwrap(),
        f1.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    // Accept both path separators: Windows renders entries with `\`.
    assert!(
        lines[0].contains("dir_z/file.txt") || lines[0].contains(r"dir_z\file.txt"),
        "reversed relative-path sort must list dir_z first: {stdout}"
    );
    assert!(
        lines[1].contains("dir_a/file.txt") || lines[1].contains(r"dir_a\file.txt"),
        "reversed relative-path sort must list dir_a second: {stdout}"
    );
}

#[test]
fn repos_column_suppressed_by_no_git() {
    let Some(repo) = TempGitRepo::new("t3_repos_nogit") else {
        return;
    };
    repo.write_file("file.txt", b"file\n");
    let output = run_lez(&[
        "-l",
        "--git-repos",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn worktree_branch_glyphs_render() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_glyphs");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("wt");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-glyph-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lez(&[
        "-l",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn explicit_dir_filtered_by_path_glob() {
    let Some(repo) = TempGitRepo::new("t3_explicit_pathglob") else {
        return;
    };
    repo.write_file(".gitignore", b"dist/\n");
    repo.write_file("dist/logs/app.log", b"log\n");
    repo.write_file("dist/bundle.js", b"js\n");

    let dist_dir = repo.path.join("dist");
    let output = run_lez(&[
        "-1",
        "--tree",
        "--git-ignore",
        "-I",
        "dist/logs/*",
        "--color=never",
        dist_dir.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bundle.js"));
}

#[test]
fn worktree_sorted_by_path() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t3_wt_sortpath");
    let main_path = temp.create_dir("main");
    let wt_path = temp.create_dir("wt");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&main_path)
        .output();
    fs::write(main_path.join("f.txt"), b"1\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "f.txt"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "1"])
        .current_dir(&main_path)
        .output();
    let _ = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            "wt-sp-b",
            wt_path.to_str().unwrap(),
        ])
        .current_dir(&main_path)
        .output();

    let output = run_lez(&[
        "-1",
        "--tree",
        "--sort=path",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

// ====================================================================================
// Realistic workload simulations
// ====================================================================================

#[test]
fn monorepo_multitool_workspace_listing() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_monorepo");
    let repo_root = temp.create_dir("monorepo");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Mono User"])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "mono@example.com"])
        .current_dir(&repo_root)
        .output();

    write_file_at(
        &repo_root,
        ".gitignore",
        b"packages/*/dist/\npackages/*/node_modules/\n*.log\n",
    );
    write_file_at(
        &repo_root,
        "packages/core/src/index.ts",
        b"export const core = 1;\n",
    );
    write_file_at(&repo_root, "packages/core/dist/index.js", b"bundle\n");
    write_file_at(&repo_root, "packages/cli/src/main.ts", b"console.log(1);\n");
    write_file_at(&repo_root, "packages/docs/README.md", b"# Docs\n");

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_root)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init monorepo"])
        .current_dir(&repo_root)
        .output();

    // 1. Root listing with --git-ignore
    let output_root = run_lez(&[
        "-1",
        "--tree",
        "--git-ignore",
        "--color=never",
        repo_root.to_str().unwrap(),
    ]);
    assert!(output_root.status.success());
    let stdout_root = String::from_utf8_lossy(&output_root.stdout);
    assert!(!stdout_root.contains("dist"));
    assert!(stdout_root.contains("core"));

    // 2. Explicit inspection of ignored dist directory
    let dist_dir = repo_root.join("packages/core/dist");
    let output_dist = run_lez(&[
        "-1",
        "--git-ignore",
        "--color=never",
        dist_dir.to_str().unwrap(),
    ]);
    assert!(output_dist.status.success());
    let stdout_dist = String::from_utf8_lossy(&output_dist.stdout);
    assert!(stdout_dist.contains("index.js"));
}

#[test]
fn nested_submodules_and_worktrees() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_submod_wt");
    let main_repo = temp.create_dir("main_project");
    let sub_repo = temp.create_dir("main_project/libs/submod");
    let wt_dir = temp.create_dir("worktrees/wt_feat");

    // Init main repo
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Dev"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "dev@test.com"])
        .current_dir(&main_repo)
        .output();
    fs::write(main_repo.join("root.txt"), b"root\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "root.txt"])
        .current_dir(&main_repo)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init main"])
        .current_dir(&main_repo)
        .output();

    // Init sub repo
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Sub"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "sub@test.com"])
        .current_dir(&sub_repo)
        .output();
    fs::write(sub_repo.join("sub.txt"), b"sub\n").unwrap();
    let _ = Command::new("git")
        .args(["add", "sub.txt"])
        .current_dir(&sub_repo)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init sub"])
        .current_dir(&sub_repo)
        .output();

    // Worktree
    let _ = Command::new("git")
        .args(["worktree", "add", "-b", "feat-wt", wt_dir.to_str().unwrap()])
        .current_dir(&main_repo)
        .output();

    let output = run_lez(&[
        "-la",
        "--git-repos",
        "--git-glyphs",
        "--color=never",
        temp.path.to_str().unwrap(),
    ]);
    assert!(output.status.success());
}

#[test]
fn multi_repo_developer_hub() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_multi_repo");
    let hub = temp.create_dir("dev_hub");

    for name in &["repo_alpha", "repo_beta", "repo_gamma"] {
        let r = hub.join(name);
        fs::create_dir_all(&r).unwrap();
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.name", "Dev"])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["config", "user.email", "dev@dev.com"])
            .current_dir(&r)
            .output();
        write_file_at(&r, "f.txt", b"1\n");
        let _ = Command::new("git")
            .args(["add", "."])
            .current_dir(&r)
            .output();
        let _ = Command::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(&r)
            .output();
    }

    let output = run_lez(&["-l", "--git-repos", "--color=never", hub.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("repo_alpha"));
    assert!(stdout.contains("repo_beta"));
    assert!(stdout.contains("repo_gamma"));
}

#[test]
fn mixed_args_globs_and_no_git() {
    let temp = TempTestDir::new("t4_complex_cli");
    let f1 = temp.create_file("Cargo.toml", b"toml");
    let f2 = temp.create_file("src/main.rs", b"main");
    let f3 = temp.create_file("dist/temp.dat", b"temp");

    let output = run_lez(&[
        "-1d",
        "--sort=name",
        "--no-git",
        "-I",
        "*.dat",
        "--color=never",
        f3.to_str().unwrap(),
        f1.to_str().unwrap(),
        f2.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn full_feature_roundtrip() {
    if !git_available() {
        return;
    }
    let temp = TempTestDir::new("t4_roundtrip");
    let root = temp.create_dir("full_project");

    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Tester"])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "tester@test.com"])
        .current_dir(&root)
        .output();

    write_file_at(&root, ".gitignore", b"target/\n*.tmp\n");
    write_file_at(&root, "src/lib.rs", b"pub fn run() {}\n");
    write_file_at(&root, "target/build.bin", b"binary\n");

    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(&root)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(&root)
        .output();

    // Modify a file, add untracked, check git-glyphs, git-repos, path-sort, git-ignore
    write_file_at(&root, "src/lib.rs", b"pub fn run() { println!(); }\n");
    write_file_at(&root, "src/new.rs", b"pub fn new() {}\n");

    let output = run_lez(&[
        "-l",
        "--git",
        "--git-repos",
        "--git-glyphs",
        "--git-ignore",
        "--sort=path",
        "--color=never",
        root.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("src"));
    assert!(!stdout.contains("target"));
}

// ====================================================================================
// Full-pipeline scenarios
// ====================================================================================

#[cfg(unix)]
#[test]
fn symlink_git_full_lifecycle() {
    let Some(repo) = TempRepoFixture::new("lifecycle") else {
        return;
    };
    let target = repo.write_file("target_v1.txt", b"v1\n");
    let link = repo
        .create_symlink("target_v1.txt", "sym_link.txt")
        .unwrap();

    // 1. Untracked status
    let out_untracked = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_untracked.status.success());
    let s_untracked = String::from_utf8_lossy(&out_untracked.stdout);
    assert!(s_untracked.contains("sym_link.txt"));

    // 2. Staged
    assert!(repo.git(&["add", "."]));
    let out_staged = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_staged.status.success());

    // 3. Committed
    assert!(repo.git(&["commit", "-q", "-m", "init"]));
    let out_clean = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_clean.status.success());
    let s_clean = String::from_utf8_lossy(&out_clean.stdout);
    let link_line = s_clean
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line.contains("--"));

    // 4. Modify target only -> link remains clean (--)
    fs::write(&target, b"v2\n").unwrap();
    let out_tgt_mod = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_tgt_mod.status.success());
    let s_tgt_mod = String::from_utf8_lossy(&out_tgt_mod.stdout);
    let link_line2 = s_tgt_mod
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line2.contains("--"));

    // 5. Modify symlink only -> link becomes modified (-M)
    fs::remove_file(&link).unwrap();
    repo.write_file("target_v2.txt", b"v2 content\n");
    repo.create_symlink("target_v2.txt", "sym_link.txt")
        .unwrap();

    let out_link_mod = run_lez(&["-l", "--git", "--color=never", repo.path.to_str().unwrap()]);
    assert!(out_link_mod.status.success());
    let s_link_mod = String::from_utf8_lossy(&out_link_mod.stdout);
    let link_line3 = s_link_mod
        .lines()
        .find(|l| l.contains("sym_link.txt"))
        .unwrap();
    assert!(link_line3.contains("-M"));
}

#[test]
fn glob_and_gitignore_end_to_end() {
    let Some(repo) = TempRepoFixture::new("glob_scoping") else {
        return;
    };
    repo.write_file(".gitignore", b"target/\nbuild/\n*.bak\n");
    repo.write_file("src/main.rs", b"fn main() {}\n");
    repo.write_file("target/app.bin", b"bin\n");
    repo.write_file("build/temp.dat", b"temp\n");
    repo.write_file("root.bak", b"bak\n");

    // 1. Root listing hides target, build, root.bak
    let out_root = run_lez(&[
        "-1",
        "--git-ignore",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(out_root.status.success());
    let s_root = String::from_utf8_lossy(&out_root.stdout);
    assert!(!s_root.contains("target"));
    assert!(!s_root.contains("build"));
    assert!(!s_root.contains("root.bak"));

    // 2. Explicit target listing reveals target/app.bin
    let target_dir = repo.path.join("target");
    let out_target = run_lez(&[
        "-1",
        "--git-ignore",
        "--color=never",
        target_dir.to_str().unwrap(),
    ]);
    assert!(out_target.status.success());
    let s_target = String::from_utf8_lossy(&out_target.stdout);
    assert!(s_target.contains("app.bin"));

    // 3. --no-git overrides --git-ignore
    let out_nogit = run_lez(&[
        "-1",
        "--git-ignore",
        "--no-git",
        "--color=never",
        repo.path.to_str().unwrap(),
    ]);
    assert!(out_nogit.status.success());
    let s_nogit = String::from_utf8_lossy(&out_nogit.stdout);
    assert!(s_nogit.contains("target"));
    assert!(s_nogit.contains("build"));
    assert!(s_nogit.contains("root.bak"));
}
