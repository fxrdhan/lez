// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::path::PathBuf;
use std::process::Command;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "lez_opt_test_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_cli_order_precedence_between_almost_all_and_double_all() {
    let temp = TempTestDir::new("all_almost_all_order");
    let sample = temp.path.join("regular.txt");
    let hidden = temp.path.join(".hidden.txt");
    StdFile::create(&sample).expect("create regular");
    StdFile::create(&hidden).expect("create hidden");

    // 1. -a -a without -A shows "." and ".."
    let out_double_all = Command::new(bin_path())
        .arg("-a")
        .arg("-a")
        .arg(&temp.path)
        .output()
        .expect("run lez -a -a");
    assert!(out_double_all.status.success());
    let stdout_double_all = String::from_utf8_lossy(&out_double_all.stdout);
    let entries_double: Vec<&str> = stdout_double_all.split_whitespace().collect();
    assert!(
        entries_double.contains(&"."),
        "lez -a -a must show '.', got: {stdout_double_all:?}"
    );
    assert!(
        entries_double.contains(&".."),
        "lez -a -a must show '..', got: {stdout_double_all:?}"
    );

    // 2. -A alone hides "." and ".."
    let out_almost_all = Command::new(bin_path())
        .arg("-A")
        .arg(&temp.path)
        .output()
        .expect("run lez -A");
    assert!(out_almost_all.status.success());
    let stdout_almost = String::from_utf8_lossy(&out_almost_all.stdout);
    let entries_almost: Vec<&str> = stdout_almost.split_whitespace().collect();
    assert!(
        !entries_almost.contains(&"."),
        "lez -A must not show '.', got: {stdout_almost:?}"
    );
    assert!(
        !entries_almost.contains(&".."),
        "lez -A must not show '..', got: {stdout_almost:?}"
    );
    assert!(
        entries_almost.iter().any(|s| s.contains(".hidden.txt")),
        "lez -A must show .hidden.txt"
    );

    // 3. -A followed by -a -a: -a -a is later on command line, so -a -a wins and "." / ".." are shown
    let out_almost_then_double = Command::new(bin_path())
        .arg("-A")
        .arg("-a")
        .arg("-a")
        .arg(&temp.path)
        .output()
        .expect("run lez -A -a -a");
    assert!(out_almost_then_double.status.success());
    let stdout_almost_then_double = String::from_utf8_lossy(&out_almost_then_double.stdout);
    let entries_atd: Vec<&str> = stdout_almost_then_double.split_whitespace().collect();
    assert!(
        entries_atd.contains(&"."),
        "lez -A -a -a must show '.' because -a -a was passed after -A, got: {stdout_almost_then_double:?}"
    );
    assert!(
        entries_atd.contains(&".."),
        "lez -A -a -a must show '..' because -a -a was passed after -A, got: {stdout_almost_then_double:?}"
    );

    // 4. -a -a followed by -A: -A is later on command line, so -A wins and "." / ".." are suppressed
    let out_double_then_almost = Command::new(bin_path())
        .arg("-a")
        .arg("-a")
        .arg("-A")
        .arg(&temp.path)
        .output()
        .expect("run lez -a -a -A");
    assert!(out_double_then_almost.status.success());
    let stdout_double_then_almost = String::from_utf8_lossy(&out_double_then_almost.stdout);
    let entries_dta: Vec<&str> = stdout_double_then_almost.split_whitespace().collect();
    assert!(
        !entries_dta.contains(&"."),
        "lez -a -a -A must suppress '.' because -A was passed after -a -a, got: {stdout_double_then_almost:?}"
    );
    assert!(
        !entries_dta.contains(&".."),
        "lez -a -a -A must suppress '..' because -A was passed after -a -a, got: {stdout_double_then_almost:?}"
    );
}

#[test]
fn test_strict_mode_rejects_modern_long_flags_without_long() {
    let temp = TempTestDir::new("strict_modern_flags");
    let sample = temp.path.join("file.txt");
    StdFile::create(&sample).expect("create file");

    for flag_arg in &[
        "--color-scale=size",
        "--color-scale-mode=gradient",
        "--no-symlink-targets",
    ] {
        let out = Command::new(bin_path())
            .env("LEZ_STRICT", "1")
            .env_remove("EZA_STRICT")
            .arg(flag_arg)
            .arg(&sample)
            .output()
            .expect("run lez in strict mode");

        assert_eq!(
            out.status.code(),
            Some(3),
            "LEZ_STRICT=1 must exit with code 3 (OPTIONS_ERROR) when passing '{flag_arg}' without -l"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("is useless without option long"),
            "Expected 'is useless without option long' in stderr for '{flag_arg}', got: {stderr}"
        );
    }

    // With -l, they must be accepted in strict mode
    for flag_arg in &[
        "--color-scale=size",
        "--color-scale-mode=gradient",
        "--no-symlink-targets",
    ] {
        let out = Command::new(bin_path())
            .env("LEZ_STRICT", "1")
            .env_remove("EZA_STRICT")
            .arg("-l")
            .arg(flag_arg)
            .arg(&sample)
            .output()
            .expect("run lez in strict mode with -l");

        assert!(
            out.status.success(),
            "LEZ_STRICT=1 must accept '{flag_arg}' when -l is present"
        );
    }
}

#[test]
fn test_strict_mode_rejects_follow_symlinks_without_recurse_or_tree() {
    let temp = TempTestDir::new("strict_follow_symlinks");
    let sample = temp.path.join("file.txt");
    StdFile::create(&sample).expect("create file");

    // Without -R or -T, --follow-symlinks in strict mode must return exit code 3
    let out = Command::new(bin_path())
        .env("LEZ_STRICT", "1")
        .env_remove("EZA_STRICT")
        .arg("--follow-symlinks")
        .arg(&sample)
        .output()
        .expect("run lez in strict mode with --follow-symlinks");

    assert_eq!(
        out.status.code(),
        Some(3),
        "LEZ_STRICT=1 must exit with code 3 when --follow-symlinks is passed without recurse or tree"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("is useless without options recurse or tree"),
        "Expected useless without options recurse or tree error, got: {stderr}"
    );

    // With -R (recurse), --follow-symlinks in strict mode must succeed
    let out_recurse = Command::new(bin_path())
        .env("LEZ_STRICT", "1")
        .env_remove("EZA_STRICT")
        .arg("-R")
        .arg("--follow-symlinks")
        .arg(&sample)
        .output()
        .expect("run lez in strict mode with -R --follow-symlinks");

    assert!(
        out_recurse.status.success(),
        "LEZ_STRICT=1 must accept --follow-symlinks when -R is active"
    );

    // With -T (tree), --follow-symlinks in strict mode must succeed
    let out_tree = Command::new(bin_path())
        .env("LEZ_STRICT", "1")
        .env_remove("EZA_STRICT")
        .arg("-T")
        .arg("--follow-symlinks")
        .arg(&sample)
        .output()
        .expect("run lez in strict mode with -T --follow-symlinks");

    assert!(
        out_tree.status.success(),
        "LEZ_STRICT=1 must accept --follow-symlinks when -T is active"
    );
}
