// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
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
            "lsr_adv_b5_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
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

    fn create_dir(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_m1_json_cli_short_single_directory() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_short");
    temp.create_file("alpha.txt", b"a");
    temp.create_file("beta.rs", b"b");
    temp.create_dir("gamma_dir");

    let output = Command::new(bin_path)
        .args(["--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    let arr = val.as_array().expect("Expected JSON array");
    let items: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(items.contains(&"alpha.txt"));
    assert!(items.contains(&"beta.rs"));
    assert!(items.contains(&"gamma_dir"));
}

#[test]
fn test_m1_json_cli_short_empty_directory() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_empty");

    let output = Command::new(bin_path)
        .args(["--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    let arr = val.as_array().expect("Expected JSON array");
    assert!(arr.is_empty());
}

#[test]
fn test_m1_json_cli_short_single_file() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_file");
    let file_path = temp.create_file("solo.txt", b"solo");

    let output = Command::new(bin_path)
        .args(["--json", file_path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    let arr = val.as_array().expect("Expected JSON array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].as_str().unwrap(), "solo.txt");
}

#[test]
fn test_m1_json_cli_short_multi_directories() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_multidir");
    let dir_a = temp.create_dir("dirA");
    let dir_b = temp.create_dir("dirB");
    temp.create_file("dirA/file_a.txt", b"a");
    temp.create_file("dirB/file_b.txt", b"b");

    let output = Command::new(bin_path)
        .args(["--json", dir_a.to_str().unwrap(), dir_b.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    let obj = val.as_object().expect("Expected JSON map for multi dirs");
    assert!(obj.contains_key(dir_a.to_str().unwrap()));
    assert!(obj.contains_key(dir_b.to_str().unwrap()));

    let arr_a = obj
        .get(dir_a.to_str().unwrap())
        .unwrap()
        .as_array()
        .expect("dirA must be array");
    assert_eq!(arr_a[0].as_str().unwrap(), "file_a.txt");
}

#[test]
fn test_m1_json_cli_short_mixed_files_and_directories() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_mixed");
    let f1 = temp.create_file("top.txt", b"top");
    let dir1 = temp.create_dir("subfolder");
    temp.create_file("subfolder/inner.txt", b"inner");

    let output = Command::new(bin_path)
        .args(["--json", f1.to_str().unwrap(), dir1.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    let obj = val.as_object().expect("Expected JSON object for mixed");
    assert!(obj.contains_key("files"));
    assert!(obj.contains_key("directories"));
}

#[test]
fn test_m1_json_cli_long_metadata_schema() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_long");
    temp.create_file("test.txt", b"content of test file");

    let output = Command::new(bin_path)
        .args([
            "-l",
            "--octal-permissions",
            "--json",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));
    let obj = val.as_object().expect("Expected JSON map");

    let file_meta = obj.get("test.txt").expect("test.txt must exist in map");
    let meta_obj = file_meta.as_object().expect("Metadata must be an object");

    assert!(meta_obj.contains_key("Permissions"));
    assert!(meta_obj.contains_key("Size"));
    #[cfg(unix)]
    {
        assert!(meta_obj.contains_key("Octal"));
        assert_eq!(meta_obj.get("Octal").unwrap().as_str().unwrap(), "0644");
    }
}

#[test]
fn test_m1_json_cli_long_empty_directory() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_long_empty");

    let output = Command::new(bin_path)
        .args(["-l", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));
    let obj = val.as_object().expect("Expected JSON map");
    assert!(obj.is_empty());
}

#[test]
fn test_m1_json_cli_all_hidden_files() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_hidden");
    temp.create_file(".secret.txt", b"secret");
    temp.create_file("visible.txt", b"visible");

    let output = Command::new(bin_path)
        .args(["-a", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));
    let arr = val.as_array().unwrap();
    let items: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(items.contains(&".secret.txt"));
    assert!(items.contains(&"visible.txt"));
}

#[test]
fn test_m1_json_cli_bytes_and_binary_units() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_units");
    temp.create_file("large.bin", &vec![0u8; 1024 * 1024]);

    // --bytes mode
    let out_bytes = Command::new(bin_path)
        .args(["-l", "--bytes", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");
    assert!(out_bytes.status.success());
    let val_bytes: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out_bytes.stdout)).unwrap();
    let size_bytes = val_bytes
        .get("large.bin")
        .unwrap()
        .get("Size")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(size_bytes, "1,048,576");

    // --binary mode
    let out_binary = Command::new(bin_path)
        .args(["-l", "--binary", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");
    assert!(out_binary.status.success());
    let val_binary: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out_binary.stdout)).unwrap();
    let size_binary = val_binary
        .get("large.bin")
        .unwrap()
        .get("Size")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(size_binary, "1.0Mi");
}

#[test]
fn test_m1_json_cli_time_styles() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_time");
    temp.create_file("stamp.txt", b"timestamp test");

    let output = Command::new(bin_path)
        .args([
            "-l",
            "--time-style=iso",
            "--json",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let mod_time = val
        .get("stamp.txt")
        .unwrap()
        .get("Date Modified")
        .unwrap()
        .as_str()
        .unwrap();
    // ISO format: YYYY-MM-DD HH:MM
    assert!(mod_time.contains('-'));
    assert!(mod_time.contains(':'));
}

#[test]
fn test_m1_json_cli_recursive_tree() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_tree");
    temp.create_file("root_file.txt", b"root");
    temp.create_file("sub/nested_file.txt", b"nested");

    let output = Command::new(bin_path)
        .args(["-R", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    assert!(val.is_object());
    let top_dir = temp.path.file_name().unwrap().to_str().unwrap();
    let top_obj = val.get(top_dir).expect("Must contain top directory");
    assert!(top_obj.get("files").is_some());
    assert!(top_obj.get("directories").is_some());
}

#[test]
fn test_m1_json_cli_recursive_long_tree() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_long_tree");
    temp.create_file("root_file.txt", b"root");
    temp.create_file("sub/nested_file.txt", b"nested");

    let output = Command::new(bin_path)
        .args(["-l", "-R", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Invalid JSON: {e}, stdout: {stdout}"));

    assert!(val.is_object());
    let top_dir = temp.path.file_name().unwrap().to_str().unwrap();
    let top_obj = val.get(top_dir).expect("Must contain top directory");
    let files_obj = top_obj
        .get("files")
        .expect("Must contain files object")
        .as_object()
        .unwrap();
    assert!(files_obj.contains_key("root_file.txt"));
    assert!(files_obj.get("root_file.txt").unwrap().is_object());
}

#[test]
fn test_m1_json_cli_special_characters_escaping() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_escaping");
    temp.create_file("file with spaces.txt", b"1");
    temp.create_file("file\"with\"quotes.txt", b"2");
    temp.create_file("emoji_🚀_tag.txt", b"3");
    temp.create_file("unicode_日本語_test.txt", b"4");

    let output = Command::new(bin_path)
        .args(["--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse failed for escaped chars: {e}\nOutput:\n{stdout}"));

    let arr = val.as_array().unwrap();
    let items: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(items.contains(&"file with spaces.txt"));
    assert!(items.contains(&"file\"with\"quotes.txt"));
    assert!(items.contains(&"emoji_🚀_tag.txt"));
    assert!(items.contains(&"unicode_日本語_test.txt"));
}

#[test]
fn test_m1_json_cli_no_ansi_escapes() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_no_ansi");
    temp.create_file("plain.txt", b"plain");

    let output = Command::new(bin_path)
        .args([
            "-l",
            "--color=always",
            "--json",
            temp.path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1B["),
        "JSON output must never contain ANSI escape codes"
    );
    let _: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
#[cfg(unix)]
fn test_m1_json_cli_symlinks() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_symlink");
    let target = temp.create_file("target.txt", b"target");
    let link_path = temp.path.join("link.txt");
    std::os::unix::fs::symlink(&target, &link_path).unwrap();

    let output = Command::new(bin_path)
        .args(["-l", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let link_meta = val.get("link.txt").unwrap().as_object().unwrap();
    let perms = link_meta.get("Permissions").unwrap().as_str().unwrap();
    assert!(
        perms.starts_with('l'),
        "Symlink permission string must start with 'l', got {perms}"
    );
}

#[test]
#[cfg(feature = "git")]
fn test_m1_json_cli_git_status() {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    let temp = TempTestDir::new("json_git");
    let repo = git2::Repository::init(&temp.path).expect("Failed to init git repo");

    let file_path = temp.create_file("tracked.txt", b"initial");
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("tracked.txt")).unwrap();
    index.write().unwrap();

    // Now modify the file
    let mut f = StdFile::create(&file_path).unwrap();
    f.write_all(b"modified").unwrap();

    let output = Command::new(bin_path)
        .args(["-l", "--git", "--json", temp.path.to_str().unwrap()])
        .output()
        .expect("Failed to run lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let git_status = val
        .get("tracked.txt")
        .unwrap()
        .get("Git")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(git_status == "NM" || git_status == "-M" || git_status == "N-");
}
