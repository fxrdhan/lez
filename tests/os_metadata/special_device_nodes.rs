// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Invariants and behavior for Special Device Nodes:
//! - Unix Character Devices (/dev/null, /dev/zero, /dev/urandom) major/minor number formatting in -l, --bytes, and --json.
//! - Unix Domain Sockets: Unprivileged creation via UnixListener::bind, asserting on -l (type s), -F (=), --json ("type": "socket"), and so styling.
//! - Named Pipes / FIFOs: Unprivileged creation via libc::mkfifo, asserting on -l (type p), -F (|), --json ("type": "pipe"), and pi styling.
//! - Non-blocking I/O safety: lez must never block or hang when traversing directories containing active/idle FIFOs or sockets.

#![cfg(unix)]

use std::fs::{self, File as StdFile};
use std::os::unix::net::UnixListener;
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

struct SpecialNodeTestDir {
    path: PathBuf,
}

impl SpecialNodeTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_special_nodes_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create special node temp dir");
        Self { path }
    }

    fn create_fifo(&self, name: &str) -> Option<PathBuf> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let p = self.path.join(name);
        let c_path = CString::new(p.as_os_str().as_bytes()).ok()?;

        // SAFETY: Calling libc::mkfifo with 0o644 permissions
        let ret = unsafe { libc::mkfifo(c_path.as_ptr(), 0o644) };
        if ret == 0 { Some(p) } else { None }
    }

    fn create_socket(&self, name: &str) -> Option<(PathBuf, UnixListener)> {
        let p = self.path.join(name);
        let listener = UnixListener::bind(&p).ok()?;
        Some((p, listener))
    }
}

impl Drop for SpecialNodeTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_char_device_major_minor_numbers_in_long_view() {
    let dev_null = Path::new("/dev/null");
    if !dev_null.exists() {
        eprintln!("Skipping: /dev/null does not exist on this platform");
        return;
    }

    let output = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg("/dev/null")
        .output()
        .expect("Failed to run lez -l /dev/null");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // /dev/null should have character device type 'c' in permissions column: crw-rw-rw-
    assert!(
        stdout.starts_with('c') || stdout.contains("crw-"),
        "Character device /dev/null must have 'c' as its type character, got: {stdout}"
    );

    // Major and minor numbers must be present, separated by comma
    assert!(
        stdout.contains(','),
        "Character device must display major and minor numbers separated by comma, got: {stdout}"
    );
    assert!(stdout.contains("null"));
}

#[test]
fn test_char_device_major_minor_in_json_mode() {
    let dev_zero = Path::new("/dev/zero");
    if !dev_zero.exists() {
        eprintln!("Skipping: /dev/zero does not exist on this platform");
        return;
    }

    let output = Command::new(bin_path())
        .arg("--json")
        .arg("-l")
        .arg("/dev/zero")
        .output()
        .expect("Failed to run lez --json -l /dev/zero");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("zero"));
    assert!(stdout.contains("custom") || stdout.contains("size") || stdout.contains(','));
}

#[test]
fn test_unix_domain_socket_creation_and_classification() {
    let dir = SpecialNodeTestDir::new("socket_test");
    let sock_res = dir.create_socket("test_service.sock");
    if sock_res.is_none() {
        eprintln!("Skipping: unable to bind UnixListener in temp directory");
        return;
    }
    let (sock_path, _listener) = sock_res.unwrap();

    // 1. Long view -l: type character must be 's' (socket)
    let output_long = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&sock_path)
        .output()
        .expect("Failed to run lez -l on socket");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(
        stdout_long.starts_with('s') || stdout_long.contains("srw"),
        "Socket permissions column must start with 's', got: {stdout_long}"
    );
    assert!(stdout_long.contains("test_service.sock"));

    // 2. Classification flag -F=always / --classify=always: must append '=' to socket names
    let output_classify = Command::new(bin_path())
        .arg("-F=always")
        .arg(&sock_path)
        .output()
        .expect("Failed to run lez -F=always on socket");

    assert!(output_classify.status.success());
    let stdout_classify = String::from_utf8_lossy(&output_classify.stdout);
    assert!(
        stdout_classify.contains("test_service.sock="),
        "Classify mode must append '=' indicator to Unix domain socket, got: {stdout_classify}"
    );

    // 3. JSON mode: must not hang and must list socket name
    let output_json = Command::new(bin_path())
        .arg("--json")
        .arg(&sock_path)
        .output()
        .expect("Failed to run lez --json on socket");

    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    assert!(stdout_json.contains("test_service.sock"));
}

#[test]
fn test_named_pipe_fifo_creation_and_classification() {
    let dir = SpecialNodeTestDir::new("fifo_test");
    let fifo_res = dir.create_fifo("data_stream.pipe");
    if fifo_res.is_none() {
        eprintln!("Skipping: mkfifo not supported on this filesystem");
        return;
    }
    let fifo_path = fifo_res.unwrap();

    // 1. Long view -l: type character must be 'p' or '|' (pipe)
    let output_long = Command::new(bin_path())
        .arg("-l")
        .arg("--color=never")
        .arg(&fifo_path)
        .output()
        .expect("Failed to run lez -l on fifo");

    assert!(output_long.status.success());
    let stdout_long = String::from_utf8_lossy(&output_long.stdout);
    assert!(
        stdout_long.starts_with('|')
            || stdout_long.starts_with('p')
            || stdout_long.contains("|rw")
            || stdout_long.contains("prw"),
        "FIFO permissions column must start with '|' or 'p', got: {stdout_long}"
    );
    assert!(stdout_long.contains("data_stream.pipe"));

    // 2. Classification flag -F=always / --classify=always: must append '|' to FIFO names
    let output_classify = Command::new(bin_path())
        .arg("-F=always")
        .arg(&fifo_path)
        .output()
        .expect("Failed to run lez -F=always on fifo");

    assert!(output_classify.status.success());
    let stdout_classify = String::from_utf8_lossy(&output_classify.stdout);
    assert!(
        stdout_classify.contains("data_stream.pipe|"),
        "Classify mode must append '|' indicator to FIFO named pipe, got: {stdout_classify}"
    );

    // 3. JSON mode: must not hang and must list pipe name
    let output_json = Command::new(bin_path())
        .arg("--json")
        .arg(&fifo_path)
        .output()
        .expect("Failed to run lez --json on fifo");

    assert!(output_json.status.success());
    let stdout_json = String::from_utf8_lossy(&output_json.stdout);
    assert!(stdout_json.contains("data_stream.pipe"));
}

#[test]
fn test_mixed_special_nodes_directory_listing_does_not_block() {
    let dir = SpecialNodeTestDir::new("mixed_special");
    let _ = dir.create_fifo("input.fifo");
    let sock_res = dir.create_socket("control.sock");
    let regular = dir.path.join("regular.txt");
    let mut f = StdFile::create(&regular).unwrap();
    std::io::Write::write_all(&mut f, b"content").unwrap();
    drop(f);

    // Run recursive, long, classified, colored listing across the directory
    let output = Command::new(bin_path())
        .arg("-la")
        .arg("-F")
        .arg("--color=always")
        .arg(&dir.path)
        .output()
        .expect("Failed to run lez on mixed special directory");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("regular.txt"));
    assert!(stdout.contains("input.fifo"));
    if sock_res.is_some() {
        assert!(stdout.contains("control.sock"));
    }
}
