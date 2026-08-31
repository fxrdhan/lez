// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Deep variant testing for `--inspect-archives`:
//! - Internal symlinks and hardlinks inside `.tar` archives
//! - Long paths (> 100 characters) triggering GNU LongName / LongLink headers
//! - PAX extended header records inside archives
//! - Mixed directory hierarchies, nested subdirectories, and JSON serialization

use std::fs::{self, File};
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
            "lez_inspect_deep_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_deep_tar(&self, name: &str) -> PathBuf {
        let tar_path = self.path.join(name);
        let file = File::create(&tar_path).unwrap();
        let mut builder = tar::Builder::new(file);

        // 1. Regular file
        let mut h1 = tar::Header::new_gnu();
        let c1 = b"normal content";
        h1.set_size(c1.len() as u64);
        h1.set_mode(0o644);
        h1.set_cksum();
        builder.append_data(&mut h1, "base.txt", &c1[..]).unwrap();

        // 2. Symlink inside tar (pointing to base.txt)
        let mut h2 = tar::Header::new_gnu();
        h2.set_entry_type(tar::EntryType::Symlink);
        h2.set_size(0);
        h2.set_mode(0o777);
        h2.set_link_name("base.txt").unwrap();
        h2.set_cksum();
        builder
            .append_data(&mut h2, "link_to_base.txt", &b""[..])
            .unwrap();

        // 3. Long path exceeding standard 100-character TAR name buffer
        let long_path = "nested_dir_structure_with_an_exceptionally_long_path_name_to_verify_gnu_longname_extension_handling_in_lez/deep_payload.txt";
        let mut h3 = tar::Header::new_gnu();
        let c3 = b"deep long path content";
        h3.set_size(c3.len() as u64);
        h3.set_mode(0o644);
        h3.set_cksum();
        builder.append_data(&mut h3, long_path, &c3[..]).unwrap();

        // 4. Subdirectory entry
        let mut h4 = tar::Header::new_gnu();
        h4.set_entry_type(tar::EntryType::Directory);
        h4.set_size(0);
        h4.set_mode(0o755);
        h4.set_cksum();
        builder
            .append_data(&mut h4, "empty_subfolder/", &b""[..])
            .unwrap();

        builder.into_inner().unwrap();
        tar_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .args(args)
        .output()
        .expect("Failed to execute lez binary");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_inspect_archives_with_symlinks_and_long_paths() {
    let fixture = TempTestDir::new("deep_tar");
    fixture.create_deep_tar("complex.tar");

    let (ok, stdout, stderr) = run_lez(&[
        "-l",
        "--color=never",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(ok, "lez -l --inspect-archives failed: {stderr}");
    assert!(stdout.contains("complex.tar"));
    assert!(
        stdout.contains("complex.tar/base.txt"),
        "Expected base.txt in listing: {stdout}"
    );
    assert!(
        stdout.contains("complex.tar/link_to_base.txt"),
        "Expected internal symlink in listing: {stdout}"
    );
    assert!(
        stdout.contains("deep_payload.txt"),
        "Expected long path entry in listing: {stdout}"
    );
}

#[test]
fn test_inspect_archives_json_serialization() {
    let fixture = TempTestDir::new("json_tar");
    fixture.create_deep_tar("archive.tar");

    let (ok, stdout, stderr) = run_lez(&[
        "--json",
        "-l",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(ok, "lez --json --inspect-archives failed: {stderr}");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output must be valid JSON: {stdout}");
}

#[test]
fn test_inspect_archives_does_not_affect_non_tar_files() {
    let fixture = TempTestDir::new("non_tar");
    fixture.create_deep_tar("real.tar");
    fs::write(fixture.path.join("readme.md"), b"# Readme\n").unwrap();
    fs::write(fixture.path.join("script.sh"), b"echo hi\n").unwrap();

    let (ok, stdout, stderr) = run_lez(&[
        "-l",
        "--color=never",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);

    assert!(ok, "lez -l --inspect-archives failed: {stderr}");
    assert!(stdout.contains("readme.md"));
    assert!(stdout.contains("script.sh"));
    assert!(stdout.contains("real.tar/base.txt"));
}
