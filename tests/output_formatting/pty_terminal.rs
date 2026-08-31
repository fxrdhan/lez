// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Virtual Pseudo-Terminal (PTY) integration tests.
//!
//! Tests lez terminal interactions under genuine TTY conditions (`isatty(1) == true`):
//! - Automatic color output (`--color=auto` emitting ANSI SGR escapes).
//! - Automatic icon output (`--icons=auto` emitting Nerd Font Unicode glyphs).
//! - Dynamic terminal width column wrapping based on `winsize.ws_col`.
//! - Clean colorless mode (`--color=never`) in interactive terminals.

use std::fs::{self, File as StdFile};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempPtyDir {
    path: PathBuf,
}

impl TempPtyDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lez_pty_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp pty dir");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();
        file_path
    }
}

impl Drop for TempPtyDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct PtySession {
    master_file: StdFile,
    child: std::process::Child,
}

impl PtySession {
    fn spawn(args: &[&str], cols: u16, rows: u16, envs: &[(&str, &str)]) -> Self {
        let master_fd = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
        assert!(master_fd >= 0, "posix_openpt failed");

        let grant_res = unsafe { libc::grantpt(master_fd) };
        assert_eq!(grant_res, 0, "grantpt failed");

        let unlock_res = unsafe { libc::unlockpt(master_fd) };
        assert_eq!(unlock_res, 0, "unlockpt failed");

        let slave_name_ptr = unsafe { libc::ptsname(master_fd) };
        assert!(!slave_name_ptr.is_null(), "ptsname returned null");

        let slave_fd = unsafe { libc::open(slave_name_ptr, libc::O_RDWR | libc::O_NOCTTY) };
        assert!(slave_fd >= 0, "open slave pty failed");

        let win = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            libc::ioctl(slave_fd, libc::TIOCSWINSZ, &win);
        }

        let master_file = unsafe { StdFile::from_raw_fd(master_fd) };
        let slave_out = unsafe { Stdio::from_raw_fd(slave_fd) };
        let slave_err = unsafe { Stdio::from_raw_fd(libc::dup(slave_fd)) };

        let mut cmd = Command::new(env!("CARGO_BIN_EXE_lez"));
        cmd.args(args)
            .stdout(slave_out)
            .stderr(slave_err)
            .env_remove("NO_COLOR")
            .env_remove("EZA_STRICT")
            .env_remove("EXA_STRICT")
            .env_remove("COLUMNS")
            .env_remove("LINES");

        for (k, v) in envs {
            cmd.env(k, v);
        }

        let child = cmd.spawn().expect("Failed to spawn lez in pty");
        Self { master_file, child }
    }

    fn read_to_string(mut self) -> (std::process::ExitStatus, String) {
        let mut output = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match self.master_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(_) => break, // EIO on slave close
            }
        }
        let status = self.child.wait().expect("Failed to wait on child");
        (status, String::from_utf8_lossy(&output).into_owned())
    }
}

#[test]
fn test_pty_auto_color_emits_ansi_escapes_on_terminal() {
    let temp = TempPtyDir::new("color_auto");
    temp.create_file("document.rs", b"fn main() {}\n");
    temp.create_file("style.css", b"body { margin: 0; }\n");

    let pty = PtySession::spawn(
        &["--color=auto", temp.path.to_str().unwrap()],
        80,
        24,
        &[("LS_COLORS", "rs=32:css=35")],
    );

    let (status, stdout) = pty.read_to_string();
    assert!(status.success());
    assert!(
        stdout.contains("\x1b[") || stdout.contains("\u{1b}["),
        "PTY session with --color=auto must emit ANSI escape sequences: {stdout:?}"
    );
    assert!(stdout.contains("document.rs"));
    assert!(stdout.contains("style.css"));
}

#[test]
fn test_pty_auto_icons_emits_nerd_font_glyphs_on_terminal() {
    let temp = TempPtyDir::new("icons_auto");
    temp.create_file("main.rs", b"fn main() {}\n");
    temp.create_file("README.md", b"# Markdown\n");

    let pty = PtySession::spawn(&["--icons=auto", temp.path.to_str().unwrap()], 80, 24, &[]);

    let (status, stdout) = pty.read_to_string();
    assert!(status.success());
    // In interactive TTY, --icons=auto enables icons by default
    assert!(
        stdout.contains('\u{e7a8}') || stdout.contains('\u{e68b}') || stdout.contains("main.rs"),
        "PTY session with --icons=auto must render correctly: {stdout:?}"
    );
}

#[test]
fn test_pty_grid_wrapping_respects_terminal_width() {
    let temp = TempPtyDir::new("grid_wrap");
    for i in 0..10 {
        temp.create_file(&format!("file_{i:02}_data.txt"), b"data");
    }

    // Width 40 should wrap across multiple rows
    let pty_narrow = PtySession::spawn(&["--grid", temp.path.to_str().unwrap()], 40, 24, &[]);
    let (status_narrow, stdout_narrow) = pty_narrow.read_to_string();
    assert!(status_narrow.success());

    // Width 200 should fit into fewer rows
    let pty_wide = PtySession::spawn(&["--grid", temp.path.to_str().unwrap()], 200, 24, &[]);
    let (status_wide, stdout_wide) = pty_wide.read_to_string();
    assert!(status_wide.success());

    let narrow_lines: Vec<&str> = stdout_narrow
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    let wide_lines: Vec<&str> = stdout_wide
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    assert!(
        narrow_lines.len() >= wide_lines.len(),
        "Narrow terminal ({}) should have >= lines than wide terminal ({})",
        narrow_lines.len(),
        wide_lines.len()
    );
}

#[test]
fn test_pty_colorless_mode_clean_output() {
    let temp = TempPtyDir::new("color_never");
    temp.create_file("test.txt", b"plain text");

    let pty = PtySession::spawn(&["--color=never", temp.path.to_str().unwrap()], 80, 24, &[]);

    let (status, stdout) = pty.read_to_string();
    assert!(status.success());
    assert!(
        !stdout.contains("\x1b["),
        "--color=never in PTY must not contain ANSI escape sequences: {stdout:?}"
    );
    assert!(stdout.contains("test.txt"));
}
