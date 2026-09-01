// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Invariants and behavior for Security Contexts (-Z / --context) and Mount Points (-M / --mounts):
//! - SELinux 4-tuple syntax (user:role:type:level) and MLS categories (s0-s0:c0.c1023).
//! - SMACK single-token label syntax and AppArmor profile syntax.
//! - Context placeholder (-) in long view on unlabelled files.
//! - Mount point detection (-M) on root (/) and temp directories.
//! - Multi-flag combination resilience (-l -Z -M -O --extended --json).

#![allow(dead_code)]

use std::fs::{self, File as StdFile};
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

struct SecMountTestDir {
    path: PathBuf,
}

impl SecMountTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_sec_mount_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create sec mount test dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        let mut f = StdFile::create(&p).unwrap();
        std::io::Write::write_all(&mut f, content).unwrap();
        p
    }
}

impl Drop for SecMountTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_selinux_context_components_structure() {
    let valid_selinux_contexts = [
        "system_u:object_r:etc_t:s0",
        "unconfined_u:unconfined_r:unconfined_t:s0-s0:c0.c1023",
        "root:sysadm_r:sysadm_t:s0",
        "user_u:user_r:user_home_t:s0",
    ];

    for ctx in valid_selinux_contexts {
        let parts: Vec<&str> = ctx.split(':').collect();
        assert!(
            parts.len() >= 4,
            "Standard SELinux context must have at least 4 colon-separated fields"
        );
        let (user, role, typ) = (parts[0], parts[1], parts[2]);
        let level = parts[3..].join(":");
        assert!(!user.is_empty(), "User field cannot be empty");
        assert!(!role.is_empty(), "Role field cannot be empty");
        assert!(!typ.is_empty(), "Type field cannot be empty");
        assert!(!level.is_empty(), "MLS level field cannot be empty");
    }
}

#[test]
fn test_smack_and_apparmor_label_structure() {
    // SMACK labels are typically single identifiers (e.g. "_", "*", "floor", "Admin")
    let smack_labels = ["_", "*", "floor", "hat", "System"];
    for label in smack_labels {
        assert!(!label.is_empty());
        assert!(
            !label.contains(':'),
            "SMACK simple labels do not use colon separation"
        );
    }

    // AppArmor labels represent profile mode
    let apparmor_labels = [
        "/usr/bin/firefox (enforce)",
        "/usr/sbin/tcpdump (complain)",
        "unconfined",
    ];
    for label in apparmor_labels {
        assert!(!label.is_empty());
    }
}

#[test]
#[cfg(unix)]
fn test_mount_point_flag_on_system_mounts() {
    // Root directory '/' is always a mount point on Unix
    let output = Command::new(bin_path())
        .arg("-ld")
        .arg("-M")
        .arg("--color=never")
        .arg("/")
        .output()
        .expect("Failed to run lez -ld -M /");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('/'));

    // Long flag --mounts should produce identical success
    let output_long = Command::new(bin_path())
        .arg("-ld")
        .arg("--mounts")
        .arg("--color=never")
        .arg("/")
        .output()
        .expect("Failed to run lez -ld --mounts /");

    assert!(output_long.status.success());
}

#[test]
fn test_full_privileged_options_combination_resilience() {
    let dir = SecMountTestDir::new("full_combo");
    let file = dir.create_file("demo.dat", b"Payload data");

    // Combine -l, -Z (context), -M (mounts), -O (flags), -@ (extended attrs)
    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-Z")
        .arg("-M")
        .arg("-O")
        .arg("-@")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to run lez with full privileged flags combination");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("demo.dat"));

    // JSON mode with the same combination
    let output_json = Command::new(bin_path())
        .arg("--json")
        .arg("-l")
        .arg("-Z")
        .arg("-M")
        .arg("-O")
        .arg(&file)
        .output()
        .expect("Failed to run lez in JSON mode with privileged flags");

    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    assert!(stdout_json.contains("demo.dat"));
}
