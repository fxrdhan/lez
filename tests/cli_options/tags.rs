// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
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
            "lez_tags_{prefix}_{}_{}",
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

#[cfg(target_os = "macos")]
fn set_macos_tags(file_path: &Path, tags: &[&str]) {
    let plist_arr: Vec<plist::Value> = tags
        .iter()
        .map(|t| plist::Value::String((*t).to_string()))
        .collect();
    let val = plist::Value::Array(plist_arr);
    let mut buf = Vec::new();
    val.to_writer_binary(&mut buf)
        .expect("Failed to serialize binary plist");

    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(file_path.as_os_str().as_bytes()).unwrap();
    let c_name = std::ffi::CString::new("com.apple.metadata:_kMDItemUserTags").unwrap();
    unsafe {
        let ret = libc::setxattr(
            c_path.as_ptr(),
            c_name.as_ptr(),
            buf.as_ptr() as *const libc::c_void,
            buf.len(),
            0,
            0,
        );
        assert_eq!(ret, 0, "libc::setxattr for macOS tags should succeed");
    }
}

#[test]
fn test_tags_cli_flag() {
    let temp = TempTestDir::new("tags_flag");
    let _file = temp.create_file("document.pdf", b"pdf content");

    #[cfg(target_os = "macos")]
    set_macos_tags(&_file, &["Work\n6", "Review\n1"]);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--tags")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez with --tags");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("document.pdf"));

    #[cfg(target_os = "macos")]
    {
        assert!(
            stdout.contains("Work"),
            "Output should render 'Work' tag: {stdout}"
        );
        assert!(
            stdout.contains("Review"),
            "Output should render 'Review' tag: {stdout}"
        );
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_macos_finder_tags_display() {
    let temp = TempTestDir::new("macos_tags");
    let file = temp.create_file("tagged_file.txt", b"tagged content");

    set_macos_tags(&file, &["Important\n6"]);

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("-e")
        .arg(&file)
        .output()
        .expect("Failed to execute lez -l -e");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tagged_file.txt"));
    assert!(
        stdout.contains("Important"),
        "Output should render 'Important' tag with -l -e: {stdout}"
    );
}
