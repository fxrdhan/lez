// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Explicit exit code verification suite:
//! - Exit 0: Success
//! - Exit 3: Options error / invalid flag combinations in strict mode (via LEZ_STRICT / EZA_STRICT)
//! - Exit 13 / 1: Permission denied / runtime I/O error
//! - Exit 1: Missing input paths / non-existent directory error

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
            "lez_exit_code_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Restore permissions so cleanup succeeds
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_exit_code_0_on_success() {
    let temp = TempTestDir::new("success");
    fs::write(temp.path.join("file.txt"), b"test").unwrap();

    let output = Command::new(bin_path())
        .arg("-1")
        .arg(&temp.path)
        .output()
        .expect("run lez");

    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit code 0 on success"
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_long_only_options() {
    let temp = TempTestDir::new("strict_opt_err");
    let temp_str = temp.path.to_str().unwrap();

    // In strict mode (LEZ_STRICT=1), passing long-only flags like --binary without -l triggers OptionsError (Exit 3)
    let output = Command::new(bin_path())
        .args(["--binary", temp_str])
        .env("LEZ_STRICT", "1")
        .output()
        .expect("run lez in strict mode with long-only option");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 (OPTIONS_ERROR) on strict option failure, got: {:?}",
        output.status.code()
    );
}

#[test]
fn test_exit_code_3_on_strict_mode_conflicting_options() {
    let temp = TempTestDir::new("strict_conflict_err");
    let temp_str = temp.path.to_str().unwrap();

    // In strict mode (EZA_STRICT=1), passing -l with --across triggers OptionsError::Useless (Exit 3)
    let output = Command::new(bin_path())
        .args(["-l", "-x", temp_str])
        .env("EZA_STRICT", "1")
        .output()
        .expect("run lez with conflicting options in strict mode");

    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 on conflicting options in strict mode"
    );
}

#[test]
fn test_exit_code_on_missing_input_path() {
    let temp = TempTestDir::new("missing_path");
    let non_existent = temp.path.join("definitely_missing_subdir_12345");

    let output = Command::new(bin_path())
        .arg(&non_existent)
        .output()
        .expect("run lez on missing path");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (MISSING_INPUT_PATH) on missing path"
    );
}

#[test]
fn test_exit_code_on_code_mode_missing_input_path() {
    let temp = TempTestDir::new("code_missing_path");
    let non_existent = temp.path.join("definitely_missing_code_subdir_12345");

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&non_existent)
        .output()
        .expect("run lez --code on missing path");

    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 (MISSING_INPUT_PATH) on missing path in --code mode"
    );
}

#[cfg(unix)]
#[test]
fn test_exit_code_13_on_permission_denied_directory() {
    use std::os::unix::fs::PermissionsExt;

    // Skip if running as root in container where chmod 000 doesn't block read
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let temp = TempTestDir::new("unreadable_dir");
    let unreadable = temp.path.join("locked_dir");
    fs::create_dir_all(&unreadable).unwrap();
    fs::write(unreadable.join("secret.txt"), b"secret").unwrap();

    // Remove all read & execute permissions (chmod 000)
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();

    let output = Command::new(bin_path())
        .arg("-l")
        .arg(&unreadable)
        .output()
        .expect("run lez on unreadable dir");

    let code = output.status.code();
    assert!(
        code == Some(13) || code == Some(1),
        "Expected exit code 13 (PERMISSION_DENIED) or 1 (RUNTIME_ERROR), got: {:?}",
        code
    );

    // Restore permissions so fixture drop cleanup succeeds
    let _ = fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o755));
}
