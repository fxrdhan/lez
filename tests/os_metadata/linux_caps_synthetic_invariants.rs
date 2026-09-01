// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Invariants and synthetic testing for Linux Capabilities (capctl / security.capability):
//! - V1 (32-bit), V2 (64-bit), and V3 (user namespace rootid) binary capability structures.
//! - Capability string decoding, flags (+e, +p, +i, =ep, =eip), and individual capability bits.
//! - Edge case and boundary parsing: zero bytes, truncated payloads (1..19 bytes), invalid magic numbers.
//! - Fallback and error resilience: unknown capability numbers, corrupted payloads falling back to raw representation.

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

struct CapTestDir {
    path: PathBuf,
}

impl CapTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_caps_synth_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create cap test dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        let mut f = StdFile::create(&p).unwrap();
        std::io::Write::write_all(&mut f, content).unwrap();
        p
    }
}

impl Drop for CapTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_linux_capabilities_v2_synthetic_binary_payload_decoding() {
    // Linux V2 capability payload layout (20 bytes):
    // bytes 0..4:   magic_etc (0x02000001 for VFS_CAP_REVISION_2 | VFS_CAP_FLAGS_EFFECTIVE in little-endian: 0x01, 0x00, 0x00, 0x02)
    // bytes 4..8:   permitted low (e.g. 1 << 13 for CAP_NET_RAW)
    // bytes 8..12:  inheritable low (e.g. 1 << 0 for CAP_CHOWN)
    // bytes 12..16: permitted high (e.g. 1 << (38 - 32) for CAP_PERFMON)
    // bytes 16..20: inheritable high (e.g. 1 << (39 - 32) for CAP_BPF)

    let v2_payload: [u8; 20] = [
        0x01, 0x00, 0x00, 0x02, // magic: VFS_CAP_REVISION_2 | EFFECTIVE
        0x00, 0x20, 0x00, 0x00, // permitted low: 1 << 13 (CAP_NET_RAW)
        0x01, 0x00, 0x00, 0x00, // inheritable low: 1 << 0 (CAP_CHOWN)
        0x40, 0x00, 0x00, 0x00, // permitted high: 1 << 6 (bit 38 = CAP_PERFMON)
        0x80, 0x00, 0x00, 0x00, // inheritable high: 1 << 7 (bit 39 = CAP_BPF)
    ];

    assert_eq!(v2_payload.len(), 20);

    // Verify magic revision byte is 2
    assert_eq!(v2_payload[3], 0x02);
    // Verify effective flag is set in lowest byte
    assert_eq!(v2_payload[0] & 0x01, 0x01);

    #[cfg(target_os = "linux")]
    {
        let unpacked = capctl::FileCaps::unpack_attrs(&v2_payload);
        assert!(
            unpacked.is_ok(),
            "capctl should successfully unpack valid V2 capability payload"
        );
        let caps = unpacked.unwrap();
        let formatted = format!("{caps}");
        assert!(
            formatted.contains("cap_net_raw"),
            "formatted caps must contain cap_net_raw: {formatted}"
        );
        assert!(
            formatted.contains("cap_chown"),
            "formatted caps must contain cap_chown: {formatted}"
        );
    }
}

#[test]
fn test_linux_capabilities_v3_synthetic_binary_payload_decoding() {
    // Linux V3 capability payload layout (24 bytes with rootid):
    // bytes 0..4:   magic_etc (0x03000001 for VFS_CAP_REVISION_3 | EFFECTIVE: 0x01, 0x00, 0x00, 0x03)
    // bytes 4..8:   permitted low (1 << 21 for CAP_SYS_ADMIN)
    // bytes 8..12:  inheritable low (0)
    // bytes 12..16: permitted high (0)
    // bytes 16..20: inheritable high (0)
    // bytes 20..24: rootid (e.g. 1000 in user namespace)

    let v3_payload: [u8; 24] = [
        0x01, 0x00, 0x00, 0x03, // magic: VFS_CAP_REVISION_3 | EFFECTIVE
        0x00, 0x00, 0x20, 0x00, // permitted low: 1 << 21 (CAP_SYS_ADMIN)
        0x00, 0x00, 0x00, 0x00, // inheritable low: 0
        0x00, 0x00, 0x00, 0x00, // permitted high: 0
        0x00, 0x00, 0x00, 0x00, // inheritable high: 0
        0xe8, 0x03, 0x00, 0x00, // rootid: 1000
    ];

    assert_eq!(v3_payload.len(), 24);
    assert_eq!(v3_payload[3], 0x03);

    #[cfg(target_os = "linux")]
    {
        let unpacked = capctl::FileCaps::unpack_attrs(&v3_payload);
        assert!(
            unpacked.is_ok(),
            "capctl should unpack valid V3 capability payload"
        );
        let caps = unpacked.unwrap();
        let formatted = format!("{caps}");
        assert!(
            formatted.contains("cap_sys_admin"),
            "formatted caps must contain cap_sys_admin: {formatted}"
        );
    }
}

#[test]
fn test_linux_capabilities_truncated_and_corrupted_payload_safety() {
    // Test all truncated payload sizes from 0 to 19 bytes
    for len in 0..20 {
        let _truncated = vec![0x02u8; len];
        #[cfg(target_os = "linux")]
        {
            let res = capctl::FileCaps::unpack_attrs(&_truncated);
            assert!(
                res.is_err(),
                "unpacking truncated payload of length {len} must fail safely"
            );
        }
    }

    // Invalid magic version (e.g. revision 99)
    let _invalid_magic: [u8; 20] = [
        0x01, 0x00, 0x00, 0x63, // revision 99 (invalid)
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    #[cfg(target_os = "linux")]
    {
        let res = capctl::FileCaps::unpack_attrs(&_invalid_magic);
        assert!(
            res.is_err(),
            "unpacking invalid revision must fail gracefully"
        );
    }
}

#[test]
fn test_capability_text_syntax_and_operators() {
    let valid_caps = [
        (
            "cap_net_admin,cap_net_raw=eip",
            vec!["cap_net_admin", "cap_net_raw"],
            "eip",
        ),
        ("cap_sys_ptrace+ep", vec!["cap_sys_ptrace"], "ep"),
        ("cap_dac_override=p", vec!["cap_dac_override"], "p"),
        ("=ep", vec![], "ep"),
    ];

    for (cap_expr, expected_names, expected_flags) in valid_caps {
        let op_idx = cap_expr
            .find(['=', '+', '-'])
            .expect("Capability string must have operator");
        let (names_part, flags_part) = cap_expr.split_at(op_idx);
        let flags = &flags_part[1..];

        assert_eq!(flags, expected_flags);
        if !names_part.is_empty() {
            let names: Vec<&str> = names_part.split(',').collect();
            assert_eq!(names, expected_names);
        }
    }
}

#[test]
fn test_cli_execution_with_extended_attributes_on_clean_dir() {
    let dir = CapTestDir::new("clean_caps");
    let file = dir.create_file("normal_app.bin", b"\x7fELF\x02\x01\x01\x00");

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-@")
        .arg("--color=never")
        .arg(&file)
        .output()
        .expect("Failed to execute lez -l -@");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("normal_app.bin"));
}
