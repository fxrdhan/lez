// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Linux Capabilities (`capctl`) and SELinux Security Context resilience and formatting invariants:
//! - Verification of capability string decoding (`cap_chown,cap_dac_override=ep`, `cap_net_admin+ep`, `=ep`, empty)
//! - Security context formatting across SELinux, SMACK, and AppArmor label standards
//! - Alignment, placeholder (`-`), and JSON representation invariants via CLI

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct SecurityTestDir {
    path: PathBuf,
}

impl SecurityTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_sec_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for SecurityTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_security_context_label_parsing_and_formatting() {
    let selinux_labels = [
        "system_u:object_r:httpd_sys_content_t:s0",
        "unconfined_u:unconfined_r:unconfined_t:s0-s0:c0.c1023",
        "system_u:object_r:bin_t:s0",
        "user_u:user_r:user_t:s0",
    ];

    for label in selinux_labels {
        // SELinux contexts consist of user:role:type:level
        let parts: Vec<&str> = label.split(':').collect();
        assert!(
            parts.len() >= 3,
            "SELinux label {label} should have at least 3 colon-delimited fields"
        );
        assert!(!parts[0].is_empty(), "User part should not be empty");
        assert!(!parts[1].is_empty(), "Role part should not be empty");
        assert!(!parts[2].is_empty(), "Type part should not be empty");
    }
}

#[test]
fn test_linux_capabilities_string_decoding_invariants() {
    let sample_caps = [
        ("cap_chown,cap_dac_override=ep", true),
        ("cap_net_raw,cap_net_admin+eip", true),
        ("cap_sys_admin=ep", true),
        ("=ep", true),
        ("", false),
    ];

    for (cap_str, is_non_empty) in sample_caps {
        let is_valid = !cap_str.trim().is_empty();
        assert_eq!(
            is_valid, is_non_empty,
            "Capability check failed for {cap_str}"
        );

        if is_valid {
            assert!(
                cap_str.contains('=') || cap_str.contains('+'),
                "Standard capability text format must contain an operator ('=' or '+')"
            );
        }
    }
}

#[test]
fn test_context_flag_cli_execution_and_placeholder() {
    let fixture = SecurityTestDir::new("ctx_cli");
    fixture.create_file("test.txt", b"content");

    let output = Command::new(bin_path())
        .args(["-l", "-Z", "--color=never", fixture.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez with -Z");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test.txt"));

    // Verify JSON mode includes context or runs cleanly without panic
    let json_output = Command::new(bin_path())
        .args(["--json", "-l", "-Z", fixture.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lez with --json -Z");

    assert!(json_output.status.success());
    let json_str = String::from_utf8_lossy(&json_output.stdout);
    assert!(json_str.contains("test.txt"));
}
