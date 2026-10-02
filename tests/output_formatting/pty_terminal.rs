// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! What a terminal changes. When stdout is a TTY, `--color=auto` turns on
//! (unless `NO_COLOR` is set and not empty), `--icons=auto` turns on, the
//! default view is the grid, and the grid takes its width from the
//! terminal's `winsize` when neither `--width` nor `COLUMNS` gives one.
//!
//! Each run goes through a real pseudo-terminal. Its output is compared
//! with the same listing written to a pipe with those settings given
//! outright, and pinned where both sides of that comparison could be wrong
//! the same way. The terminal turns each `\n` into `\r\n` on its way out.

use std::fs::File as StdFile;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::Path;
use std::process::Stdio;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Runs lez in `dir` on a fresh pseudo-terminal `cols` columns wide, and
/// returns everything it wrote to stdout and stderr, after checking it
/// succeeded.
fn on_a_terminal(dir: &Path, args: &[&str], cols: u16, envs: &[(&str, &str)]) -> String {
    // SAFETY: plain libc calls on descriptors this function owns; each
    // result is checked before it is used, and each descriptor is handed to
    // exactly one owner.
    let (master, slave) = unsafe {
        let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        assert!(
            master >= 0,
            "posix_openpt: {}",
            std::io::Error::last_os_error()
        );
        let master = OwnedFd::from_raw_fd(master);
        assert_eq!(libc::grantpt(master.as_raw_fd()), 0);
        assert_eq!(libc::unlockpt(master.as_raw_fd()), 0);
        let name = libc::ptsname(master.as_raw_fd());
        assert!(!name.is_null(), "ptsname");
        let slave = libc::open(name, libc::O_RDWR | libc::O_NOCTTY);
        assert!(
            slave >= 0,
            "open the slave: {}",
            std::io::Error::last_os_error()
        );
        let slave = OwnedFd::from_raw_fd(slave);
        let size = libc::winsize {
            ws_row: 24,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        assert_eq!(
            libc::ioctl(slave.as_raw_fd(), libc::TIOCSWINSZ, &size),
            0,
            "TIOCSWINSZ: {}",
            std::io::Error::last_os_error()
        );
        (master, slave)
    };

    let mut cmd = lez_in(dir);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(slave.try_clone().expect("dup the slave")))
        .stderr(Stdio::from(slave));
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().expect("run lez");
    // The parent's copies of the slave went with `cmd`, so once lez exits
    // nothing holds the terminal open and the reads below come to an end.
    drop(cmd);

    let mut master = StdFile::from(master);
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match master.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            // Linux reports the closed slave as EIO once the data is read.
            Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
            Err(e) => panic!("read the terminal: {e}"),
        }
    }
    let status = child.wait().expect("wait for lez");
    assert!(status.success(), "{args:?}: {status}");
    String::from_utf8(output).expect("UTF-8 output")
}

/// The same listing written to a pipe, as the terminal would show it.
fn on_a_pipe(dir: &Path, args: &[&str]) -> String {
    success_stdout(lez_in(dir).args(args)).replace('\n', "\r\n")
}

/// Ten names of sixteen characters: four fit in 80 columns, two in 40.
fn ten_files() -> TempTestDir {
    let dir = TempTestDir::new("pty");
    for i in 0..10 {
        dir.create_file(&format!("file_{i:02}_data.txt"), b"data");
    }
    dir
}

const GRID_80: &str = "file_00_data.txt  file_03_data.txt  file_06_data.txt  file_09_data.txt\r\n\
                       file_01_data.txt  file_04_data.txt  file_07_data.txt\r\n\
                       file_02_data.txt  file_05_data.txt  file_08_data.txt\r\n";

#[test]
fn colours_turn_on_by_default_and_icons_when_asked() {
    let dir = ten_files();
    let path = dir.path();
    let painted = on_a_pipe(path, &["--grid", "--width=80", "--color=always"]);
    assert_eq!(on_a_terminal(path, &[], 80, &[]), painted);
    assert_eq!(on_a_terminal(path, &["--color=auto"], 80, &[]), painted);
    assert_eq!(
        painted,
        GRID_80
            .replace("file_", "\x1b[32mfile_")
            .replace(".txt", ".txt\x1b[0m")
    );

    let with_icons = on_a_pipe(
        path,
        &["--grid", "--width=80", "--icons=always", "--color=never"],
    );
    assert_eq!(
        on_a_terminal(path, &["--icons=auto", "--color=never"], 80, &[]),
        with_icons
    );
    assert_eq!(with_icons, GRID_80.replace("file_", "\u{f15c} file_"));
}

/// On a pipe the same `auto` settings stay off, so the terminal is what
/// turned them on above.
#[test]
fn auto_stays_off_on_a_pipe() {
    let dir = ten_files();
    let plain: String = (0..10)
        .map(|i| format!("file_{i:02}_data.txt\r\n"))
        .collect();
    assert_eq!(
        on_a_pipe(dir.path(), &["--color=auto", "--icons=auto"]),
        plain
    );
}

/// The other views keep their layout on a terminal; only the colours and
/// icons come on.
#[test]
fn every_view_on_a_terminal_matches_the_pipe_with_auto_made_explicit() {
    let dir = TempTestDir::new("pty_views");
    dir.create_file("main.rs", b"fn main() {}\n");
    dir.create_file("README.md", b"# Read me\n");
    dir.create_file("src/lib.rs", b"\n");
    for view in [
        &["-1"][..],
        &["-T"],
        &["-l", "--no-time"],
        &["-lT", "--no-time"],
    ] {
        let auto = [view, &["--color=auto", "--icons=auto"]].concat();
        let explicit = [view, &["--color=always", "--icons=always"]].concat();
        let piped = on_a_pipe(dir.path(), &explicit);
        assert_ne!(
            piped,
            on_a_pipe(dir.path(), &[view, &["--color=never"]].concat()),
            "{view:?}"
        );
        assert_eq!(on_a_terminal(dir.path(), &auto, 80, &[]), piped, "{view:?}");
    }
}

/// The grid takes the terminal's width; `COLUMNS` comes first, and a
/// terminal that reports no width at all gets one name per line.
#[test]
fn the_grid_takes_the_terminals_width() {
    let dir = ten_files();
    let path = dir.path();
    let one_per_line: String = (0..10)
        .map(|i| format!("file_{i:02}_data.txt\r\n"))
        .collect();
    let two_columns: String = (0..5)
        .map(|i| format!("file_{i:02}_data.txt  file_{:02}_data.txt\r\n", i + 5))
        .collect();
    let one_row = format!(
        "{}\r\n",
        (0..10)
            .map(|i| format!("file_{i:02}_data.txt"))
            .collect::<Vec<_>>()
            .join("  ")
    );
    for (cols, expected) in [
        (10, &one_per_line),
        (40, &two_columns),
        (200, &one_row),
        (400, &one_row),
    ] {
        let width = format!("--width={cols}");
        assert_eq!(
            on_a_pipe(path, &["--grid", &width, "--color=never"]),
            *expected,
            "{cols}"
        );
        assert_eq!(
            on_a_terminal(path, &["--color=never"], cols, &[]),
            *expected,
            "{cols}"
        );
    }
    assert_eq!(
        on_a_terminal(path, &["--color=never"], 200, &[("COLUMNS", "40")]),
        two_columns
    );
    assert_eq!(
        on_a_terminal(path, &["--color=never"], 0, &[]),
        one_per_line
    );
}

/// `NO_COLOR` set to anything turns `auto` off; set but empty it does not
/// (<https://no-color.org>). An explicit `--color=always` still wins.
#[test]
fn no_color_turns_auto_off_unless_empty() {
    let dir = ten_files();
    let path = dir.path();
    let painted = on_a_pipe(path, &["--grid", "--width=80", "--color=always"]);
    let plain = on_a_pipe(path, &["--grid", "--width=80", "--color=never"]);
    assert_eq!(plain, GRID_80);
    for (no_color, args, expected) in [
        ("1", &["--color=auto"][..], &plain),
        ("1", &[], &plain),
        ("", &["--color=auto"], &painted),
        ("1", &["--color=always"], &painted),
    ] {
        assert_eq!(
            on_a_terminal(path, args, 80, &[("NO_COLOR", no_color)]),
            *expected,
            "NO_COLOR={no_color:?} {args:?}"
        );
    }
    assert_eq!(on_a_terminal(path, &["--color=never"], 80, &[]), plain);
}
