// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Adversarial tests covering edge case regressions and robustness remediations.

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_remed_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn lez_bin() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_spacing_multiplication_overflow_does_not_panic() {
    let output = Command::new(lez_bin())
        .arg("-l")
        .arg("--spacing")
        .arg("5000000000000000000")
        .arg("Cargo.toml")
        .output()
        .expect("Failed to execute lez binary");

    assert_ne!(
        output.status.code(),
        Some(101),
        "Process should not panic with exit 101 on large spacing"
    );
    assert!(output.status.success());
}

#[test]
fn test_symlink_cycle_recursion_pruned() {
    let dir = TestDir::new("symlink_cycle");
    let sub = dir.path.join("sub");
    fs::create_dir_all(&sub).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&dir.path, sub.join("loop_to_root")).unwrap();

        // JSON recursion must prune cycles cleanly and succeed
        let out_json = Command::new(lez_bin())
            .arg("--json")
            .arg("-R")
            .arg("--follow-symlinks")
            .arg(&dir.path)
            .output()
            .expect("Failed to run lez --json -R");
        assert_eq!(
            out_json.status.code(),
            Some(0),
            "lez --json -R --follow-symlinks should prune cycles cleanly"
        );

        // Standard -R recursion must not panic or hang, cleanly exiting with ELOOP error code
        let out_r = Command::new(lez_bin())
            .arg("-R")
            .arg("--follow-symlinks")
            .arg(&dir.path)
            .output()
            .expect("Failed to run lez -R");
        assert_eq!(
            out_r.status.code(),
            Some(1),
            "lez -R --follow-symlinks should isolate ELOOP with runtime error exit code 1"
        );
    }
}

#[test]
fn test_non_utf8_stdin_stream_error_handling() {
    let mut child = Command::new(lez_bin())
        .arg("--stdin")
        .env("LEZ_STDIN_SEPARATOR", "\\0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn lez --stdin");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"Cargo.toml\0\xff\xfe\0");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to read from stdin"),
        "Stdin reader should gracefully exit with error message on invalid UTF-8: {stderr}"
    );
}

#[test]
fn test_explicit_invalid_config_error_warning() {
    // 1. Nonexistent explicit config file must warn to stderr
    let out_nonexistent = Command::new(lez_bin())
        .arg("--config")
        .arg("/nonexistent/custom_config.toml")
        .arg("Cargo.toml")
        .output()
        .expect("Failed to run lez");
    assert!(out_nonexistent.status.success());
    let stderr1 = String::from_utf8_lossy(&out_nonexistent.stderr);
    assert!(
        stderr1.contains("Failed to read config file"),
        "Should warn on missing config file: {stderr1}"
    );

    // 2. Syntax-corrupted config file must warn to stderr
    let dir = TestDir::new("bad_config");
    let bad_config = dir.path.join("bad.toml");
    fs::write(&bad_config, b"[[[ syntax error").unwrap();

    let out_corrupted = Command::new(lez_bin())
        .arg("--config")
        .arg(&bad_config)
        .arg("Cargo.toml")
        .output()
        .expect("Failed to run lez");
    let stderr2 = String::from_utf8_lossy(&out_corrupted.stderr);
    assert!(
        stderr2.contains("Failed to parse config file"),
        "Should warn on corrupted config file: {stderr2}"
    );

    // 3. Valid YAML config file must succeed without parse errors
    let dir_yaml = TestDir::new("yaml_config");
    let yaml_config = dir_yaml.path.join("config.yaml");
    fs::write(&yaml_config, b"display:\n  header: true\n").unwrap();

    let out_yaml = Command::new(lez_bin())
        .arg("--config")
        .arg(&yaml_config)
        .arg("-l")
        .arg("Cargo.toml")
        .output()
        .expect("Failed to run lez");
    assert!(out_yaml.status.success());
    let stderr_yaml = String::from_utf8_lossy(&out_yaml.stderr);
    assert!(
        !stderr_yaml.contains("Failed to parse config file"),
        "Valid YAML config must not produce parse errors: {stderr_yaml}"
    );
}

#[test]
fn test_config_tree_mode_with_double_all_rejected() {
    let dir = TestDir::new("tree_cfg");
    let cfg_path = dir.path.join("tree.toml");
    fs::write(&cfg_path, b"[display]\nmode = \"tree\"\n").unwrap();

    let output = Command::new(lez_bin())
        .arg("--config")
        .arg(&cfg_path)
        .arg("-a")
        .arg("-a")
        .arg("Cargo.toml")
        .output()
        .expect("Failed to run lez");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Config mode = 'tree' with -a -a should fail with OptionsError::TreeAllAll (exit 3)"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Option --tree is useless given --all --all"),
        "Unexpected stderr: {stderr}"
    );

    // When CLI explicitly overrides mode with oneline or grid, TreeAllAll must not trigger
    let out_override = Command::new(lez_bin())
        .arg("--config")
        .arg(&cfg_path)
        .arg("-1")
        .arg("-a")
        .arg("-a")
        .arg("Cargo.toml")
        .output()
        .expect("Failed to run lez");
    assert!(out_override.status.success());
}

#[test]
fn test_strict_mode_rejects_modern_long_flags_without_long() {
    for flag in [
        "--git-repos",
        "--octal-permissions",
        "--total-size",
        "--flags",
        "--context",
        "--smart-group",
        "--extended",
        "--no-permissions",
    ] {
        let out = Command::new(lez_bin())
            .env("LEZ_STRICT", "1")
            .args([flag, "Cargo.toml"])
            .output()
            .expect("Failed to run lez in strict mode");
        assert_eq!(
            out.status.code(),
            Some(3),
            "LEZ_STRICT=1 must reject modern long-only flag '{flag}' without -l"
        );
    }

    let out_valid = Command::new(lez_bin())
        .env("LEZ_STRICT", "1")
        .args(["-l", "--octal-permissions", "Cargo.toml"])
        .output()
        .expect("Failed to run lez in strict mode with -l");
    assert!(
        out_valid.status.success(),
        "LEZ_STRICT=1 must accept long-only flag when -l is present"
    );
}

#[test]
#[cfg(feature = "git")]
fn test_ignore_submodule_contents_prunes_contents() {
    let dir = TestDir::new("submodule_ignore");
    let child_dir = dir.path.join("child");
    let parent_dir = dir.path.join("parent");
    fs::create_dir_all(&child_dir).unwrap();
    fs::create_dir_all(&parent_dir).unwrap();

    let run_git = |cwd: &Path, args: &[&str]| {
        Command::new("git")
            .current_dir(cwd)
            .args(args)
            .status()
            .is_ok_and(|s| s.success())
    };

    if !run_git(&child_dir, &["init", "-q", "-b", "main"])
        || !run_git(&child_dir, &["config", "user.name", "Tester"])
        || !run_git(&child_dir, &["config", "user.email", "tester@example.com"])
    {
        return;
    }
    fs::write(child_dir.join("child.txt"), b"child\n").unwrap();
    if !run_git(&child_dir, &["add", "."])
        || !run_git(&child_dir, &["commit", "-q", "-m", "child_init"])
    {
        return;
    }

    if !run_git(&parent_dir, &["init", "-q", "-b", "main"])
        || !run_git(&parent_dir, &["config", "user.name", "Tester"])
        || !run_git(&parent_dir, &["config", "user.email", "tester@example.com"])
    {
        return;
    }
    fs::write(parent_dir.join("parent.txt"), b"parent\n").unwrap();
    if !run_git(&parent_dir, &["add", "."])
        || !run_git(&parent_dir, &["commit", "-q", "-m", "parent_init"])
    {
        return;
    }

    if !run_git(
        &parent_dir,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "-q",
            child_dir.to_str().unwrap(),
            "sub",
        ],
    ) || !run_git(&parent_dir, &["commit", "-q", "-m", "sub_added"])
    {
        return;
    }

    let output = Command::new(lez_bin())
        .current_dir(&parent_dir)
        .args(["-T", "--ignore-submodule-contents"])
        .output()
        .expect("Failed to run lez -T --ignore-submodule-contents");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sub"),
        "Submodule folder itself must be listed: {stdout}"
    );
    assert!(
        !stdout.contains("child.txt"),
        "Submodule contents must be pruned with --ignore-submodule-contents: {stdout}"
    );
}

#[test]
#[cfg(feature = "git")]
fn test_bare_git_worktree_status_modified() {
    let dir = TestDir::new("bare_git");
    let bare_dir = dir.path.join("bare.git");
    let work_dir = dir.path.join("work");
    fs::create_dir_all(&bare_dir).unwrap();
    fs::create_dir_all(&work_dir).unwrap();

    // Initialize bare repo
    let init_bare = Command::new("git")
        .args(["init", "--bare", "-q"])
        .current_dir(&bare_dir)
        .status();
    if !init_bare.is_ok_and(|s| s.success()) {
        return;
    }

    // Configure worktree
    let test_file = work_dir.join("test.txt");
    fs::write(&test_file, b"hello\n").unwrap();

    let git_cmd = |args: &[&str]| {
        Command::new("git")
            .current_dir(&work_dir)
            .env("GIT_DIR", &bare_dir)
            .env("GIT_WORK_TREE", &work_dir)
            .args(args)
            .status()
            .is_ok_and(|s| s.success())
    };

    if !git_cmd(&["config", "user.name", "Tester"])
        || !git_cmd(&["config", "user.email", "tester@example.com"])
        || !git_cmd(&["add", "test.txt"])
        || !git_cmd(&["commit", "-q", "-m", "init"])
    {
        return;
    }

    // Modify test.txt
    fs::write(&test_file, b"hello\nmodified\n").unwrap();

    let output = Command::new(lez_bin())
        .args(["-l", "--git"])
        .current_dir(&work_dir)
        .env("GIT_DIR", &bare_dir)
        .env("GIT_WORK_TREE", &work_dir)
        .output()
        .expect("Failed to run lez -l --git");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("-M") || stdout.contains("[ M]"),
        "Expected git status column to show modified status for bare worktree file: {stdout}"
    );
}

#[test]
#[cfg(feature = "git")]
fn test_loc_percentage_not_distorted_by_git_ignored_files() {
    let dir = TestDir::new("loc_git_ignored");
    let git_cmd = |args: &[&str]| {
        Command::new("git")
            .current_dir(&dir.path)
            .args(args)
            .status()
            .is_ok_and(|s| s.success())
    };

    if !git_cmd(&["init", "-q", "-b", "main"])
        || !git_cmd(&["config", "user.name", "Tester"])
        || !git_cmd(&["config", "user.email", "tester@example.com"])
    {
        return;
    }

    fs::write(dir.path.join("tracked.rs"), b"fn main() {}\n").unwrap();
    let mut ignored_content = String::new();
    for i in 0..10 {
        ignored_content.push_str(&format!("fn f{i}() {{}}\n"));
    }
    fs::write(dir.path.join("ignored.rs"), ignored_content).unwrap();
    fs::write(dir.path.join(".gitignore"), b"ignored.rs\n").unwrap();

    if !git_cmd(&["add", ".gitignore", "tracked.rs"]) || !git_cmd(&["commit", "-q", "-m", "init"]) {
        return;
    }

    let output = Command::new(lez_bin())
        .args(["-l", "--loc=percent"])
        .current_dir(&dir.path)
        .output()
        .expect("Failed to run lez -l --loc=percent");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("1000.0%"),
        "LOC percentage should not be distorted to 1000.0%: {stdout}"
    );

    let output_grid = Command::new(lez_bin())
        .args(["-l", "--grid", "--loc=percent"])
        .current_dir(&dir.path)
        .output()
        .expect("Failed to run lez -l --grid --loc=percent");

    assert!(output_grid.status.success());
    let stdout_grid = String::from_utf8_lossy(&output_grid.stdout);
    assert!(
        !stdout_grid.contains("1000.0%"),
        "GridDetails LOC percentage should not be distorted to 1000.0%: {stdout_grid}"
    );
}

#[test]
fn test_loc_code_deduplicate_repeated_paths() {
    let output = Command::new(lez_bin())
        .args(["--code", "Cargo.toml", "Cargo.toml"])
        .output()
        .expect("Failed to run lez --code");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Total         1"),
        "lez --code should deduplicate repeated paths to 1 file: {stdout}"
    );
}

#[test]
fn test_time_style_escaped_percent_z_does_not_panic() {
    let out1 = Command::new(lez_bin())
        .args(["-l", "--time-style=+%%Z", "Cargo.toml"])
        .output()
        .expect("Failed to run lez");
    assert_ne!(out1.status.code(), Some(101), "Should not panic on +%%Z");
    assert!(out1.status.success());

    let out2 = Command::new(lez_bin())
        .args(["--json", "-l", "--time-style=+%Y-%%Z", "Cargo.toml"])
        .output()
        .expect("Failed to run lez");
    assert_ne!(
        out2.status.code(),
        Some(101),
        "Should not panic on +%Y-%%Z in json mode"
    );
    assert!(out2.status.success());
}

#[test]
fn test_ansi_c1_control_characters_escaped_in_output() {
    let dir = TestDir::new("c1_controls");
    let test_file = dir.path.join("test\u{0085}nel\u{009b}31mcsi.txt");
    StdFile::create(&test_file).unwrap();

    let output = Command::new(lez_bin())
        .args(["--color=never", dir.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez");

    assert!(output.status.success());
    let stdout_bytes = output.stdout;
    // Verify stdout does not contain raw C1 control bytes (U+0085: 0xC2 0x85; U+009B: 0xC2 0x9B)
    let has_raw_nel = stdout_bytes.windows(2).any(|w| w == [0xc2, 0x85]);
    let has_raw_csi = stdout_bytes.windows(2).any(|w| w == [0xc2, 0x9b]);

    assert!(
        !has_raw_nel && !has_raw_csi,
        "C1 control characters must be escaped, raw bytes found in stdout"
    );
}

#[test]
#[cfg(unix)]
fn test_json_directory_error_telemetry_reports_permission_code_13() {
    if unsafe { libc::geteuid() } == 0 {
        // Root ignores 000 permissions
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = TestDir::new("bug13_telemetry");
    let locked = dir.path.join("locked_dir");
    fs::create_dir_all(&locked).unwrap();
    fs::write(locked.join("file.txt"), b"data").unwrap();

    let orig_perms = fs::metadata(&locked).unwrap().permissions();
    let mut no_perms = orig_perms.clone();
    no_perms.set_mode(0o000);
    fs::set_permissions(&locked, no_perms).unwrap();

    let output = Command::new(lez_bin())
        .arg("--json")
        .arg(&locked)
        .output()
        .expect("Failed to run lez --json");

    // Restore permissions so cleanup succeeds
    let mut restore_perms = orig_perms;
    restore_perms.set_mode(0o755);
    let _ = fs::set_permissions(&locked, restore_perms);

    assert_eq!(
        output.status.code(),
        Some(13),
        "Must exit with code 13 (PERMISSION_DENIED) on permission error"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Permission denied: ") && stderr.contains("code: 13"),
        "Permission error must report code: 13 to stderr: {stderr}"
    );

    // Non-permission error (e.g. nonexistent path) must NOT emit "code: 13" or false "Permission denied"
    let non_existent = dir.path.join("non_existent_dir");
    let out_nonexist = Command::new(lez_bin())
        .arg("--json")
        .arg(&non_existent)
        .output()
        .expect("Failed to run lez --json");

    assert_ne!(
        out_nonexist.status.code(),
        Some(13),
        "Nonexistent directory must not exit with code 13"
    );
    let stderr_nonexist = String::from_utf8_lossy(&out_nonexist.stderr);
    assert!(
        !stderr_nonexist.contains("code: 13") && !stderr_nonexist.contains("Permission denied"),
        "Non-permission error must not emit false permission denied code 13: {stderr_nonexist}"
    );
}

#[test]
#[cfg(unix)]
fn test_json_mount_point_permissions_uppercase_indicator() {
    // 1. Root directory '/' is always a mount point on Unix
    let out_root = Command::new(lez_bin())
        .args(["--json", "-l", "-d", "/"])
        .output()
        .expect("Failed to run lez --json -l -d /");

    assert!(out_root.status.success());
    let stdout_root = String::from_utf8_lossy(&out_root.stdout);
    let val_root: serde_json::Value =
        serde_json::from_str(&stdout_root).expect("Must be valid JSON");
    let root_obj = val_root
        .get("/")
        .and_then(|v| v.as_object())
        .expect("Must have '/' entry");
    let perms_root = root_obj
        .get("Permissions")
        .and_then(|v| v.as_str())
        .expect("Must have Permissions string");

    assert!(
        perms_root.starts_with('D'),
        "JSON permissions for mount point directory '/' must start with uppercase 'D', got: {perms_root}"
    );

    // 2. A regular temporary directory is not a mount point and must start with lowercase 'd'
    let dir = TestDir::new("bug14_non_mount");
    let out_regular = Command::new(lez_bin())
        .args(["--json", "-l", "-d", dir.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez --json -l -d <dir>");

    assert!(out_regular.status.success());
    let stdout_regular = String::from_utf8_lossy(&out_regular.stdout);
    let val_reg: serde_json::Value =
        serde_json::from_str(&stdout_regular).expect("Must be valid JSON");
    let name = dir.path.file_name().unwrap().to_str().unwrap();
    let reg_obj = val_reg
        .get(name)
        .or_else(|| val_reg.get(dir.path.to_str().unwrap()))
        .or_else(|| val_reg.as_object().and_then(|m| m.values().next()))
        .and_then(|v| v.as_object())
        .expect("Must have entry for temp dir");
    let perms_reg = reg_obj
        .get("Permissions")
        .and_then(|v| v.as_str())
        .expect("Must have Permissions string");

    assert!(
        perms_reg.starts_with('d'),
        "JSON permissions for regular directory must start with lowercase 'd', got: {perms_reg}"
    );
}
