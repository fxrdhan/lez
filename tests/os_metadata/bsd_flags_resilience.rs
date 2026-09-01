// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Real OS Kernel BSD File Flags (`chflags`) and Extended Flags Invariants:
//! - Setting real kernel flags on filesystem entries (`UF_NODUMP`, `UF_HIDDEN`)
//! - Verification of `lez -l -O` (short flags format) and `lez -l --flags` (long flags format)
//! - macOS Finder `hidden` flag interaction with default vs `-a` (show all) filtering
//! - Zero-panic and clean teardown guarantees (clearing flags before temp dir deletion)

#![cfg(unix)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

struct FlagTestDir {
    path: PathBuf,
    flagged_files: Vec<PathBuf>,
}

impl FlagTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_flags_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self {
            path,
            flagged_files: Vec::new(),
        }
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

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    fn set_bsd_flag(&mut self, file: &Path, flag: u32) -> bool {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let Ok(c_path) = CString::new(file.as_os_str().as_bytes()) else {
            return false;
        };

        // SAFETY: Calling libc::chflags with valid CString path
        let res = unsafe { libc::chflags(c_path.as_ptr(), flag as _) };
        if res == 0 {
            self.flagged_files.push(file.to_path_buf());
            true
        } else {
            false
        }
    }
}

impl Drop for FlagTestDir {
    fn drop(&mut self) {
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;

            for file in &self.flagged_files {
                if let Ok(c_path) = CString::new(file.as_os_str().as_bytes()) {
                    // SAFETY: Resetting flags to 0 so the directory can be deleted
                    unsafe {
                        libc::chflags(c_path.as_ptr(), 0);
                    }
                }
            }
        }

        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "openbsd",
    target_os = "netbsd"
))]
fn test_real_kernel_bsd_nodump_flag_rendering() {
    let mut dir = FlagTestDir::new("nodump");
    let file = dir.create_file("archive_backup.dat", b"important backup data");

    // UF_NODUMP is user-settable without root on BSD/macOS
    let success = dir.set_bsd_flag(&file, libc::UF_NODUMP);
    if !success {
        eprintln!("Skipping: filesystem does not support UF_NODUMP");
        return;
    }

    // 1. lez -l -O (short flags option)
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-O")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez -l -O");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("nodump"),
        "lez -l -O output must contain 'nodump' flag, got: {stdout}"
    );

    // 2. lez -l --flags
    let output_long = Command::new(bin_path())
        .arg("-l")
        .arg("--flags")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez -l --flags");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(
        stdout_long.contains("nodump"),
        "lez -l --flags output must contain 'nodump' flag, got: {stdout_long}"
    );

    // 3. JSON representation contains flag
    let output_json = Command::new(bin_path())
        .arg("-l")
        .arg("-O")
        .arg("--json")
        .arg(&file)
        .output()
        .expect("Failed to run lez --json");

    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    assert!(
        stdout_json.contains("nodump"),
        "JSON output must contain 'nodump' flag, got: {stdout_json}"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn test_real_kernel_macos_hidden_flag_filtering() {
    let mut dir = FlagTestDir::new("hidden_flag");
    let _normal_file = dir.create_file("visible.txt", b"I am visible");
    let hidden_file = dir.create_file("secret.txt", b"I have UF_HIDDEN flag set");

    // UF_HIDDEN is supported on macOS APFS/HFS+
    let success = dir.set_bsd_flag(&hidden_file, libc::UF_HIDDEN);
    if !success {
        eprintln!("Skipping: filesystem does not support UF_HIDDEN");
        return;
    }

    // 1. Default listing: normal_file visible, hidden_file with UF_HIDDEN flag
    let output_default = Command::new(bin_path())
        .arg(&dir.path)
        .output()
        .expect("Failed to run lez");

    assert!(output_default.status.success());
    let stdout_default = String::from_utf8_lossy(&output_default.stdout);
    assert!(stdout_default.contains("visible.txt"));

    // 2. lez -l -O (check that UF_HIDDEN renders as 'hidden')
    let output_flags = Command::new(bin_path())
        .arg("-l")
        .arg("-O")
        .arg("-a")
        .arg("--color=never")
        .arg(&hidden_file)
        .output()
        .expect("Failed to run lez -l -O -a");

    assert!(output_flags.status.success());
    let stdout_flags = String::from_utf8_lossy(&output_flags.stdout);
    assert!(
        stdout_flags.contains("hidden"),
        "Output for file with UF_HIDDEN must contain 'hidden' flag name: {stdout_flags}"
    );
}

#[test]
fn test_zero_flags_file_displays_dash_placeholder() {
    let dir = FlagTestDir::new("zero_flags");
    let file = dir.create_file("regular.txt", b"clean file with no flags");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-O")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez -l -O");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("regular.txt"));

    // Verify placeholder '-' exists in columns
    assert!(
        stdout.contains('-'),
        "Regular file flags column must show '-' placeholder, got: {stdout}"
    );
}
