// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Writing to a pipe whose reader has gone away.
//!
//! The read end is closed before lez starts, so its first write is the one
//! that hits the closed pipe. Closing it after reading a line instead would
//! race the writer: a short listing fits in the pipe buffer and lez exits
//! normally before the reader ever leaves.
//!
//! Closing it here is not enough on its own under `cargo test`, which runs
//! tests as threads of one process: a child another test forks in the
//! meantime holds a copy of every descriptor until it execs, the read end
//! too, and while it does lez's write lands in the buffer. So the helper
//! first writes to the pipe itself until that fails, when no read end is
//! left anywhere and, closed here, none can be copied again.

use std::io::{ErrorKind, Write};
use std::process::Output;

use crate::common::{TempTestDir, lez_in};

fn run_into_closed_pipe(dir: &TempTestDir, args: &[&str]) -> Output {
    let (reader, mut writer) = std::io::pipe().expect("create pipe");
    drop(reader);
    // The test harness ignores SIGPIPE, so a write with no reader fails
    // with BrokenPipe instead of ending the test.
    loop {
        match writer.write(b"x") {
            Ok(_) => std::thread::yield_now(),
            Err(error) if error.kind() == ErrorKind::BrokenPipe => break,
            Err(error) => panic!("probe the pipe: {error}"),
        }
    }
    lez_in(dir.path())
        .args(args)
        .stdout(writer)
        .output()
        .expect("failed to run lez")
}

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("closed_pipe");
    dir.create_file("file.txt", b"payload\n");
    dir.create_file("sub/nested.txt", b"payload\n");
    dir
}

/// lez restores the default `SIGPIPE` disposition (`main.rs`), so like `ls`
/// it is terminated by the signal, silently, whatever view is writing.
#[test]
#[cfg(unix)]
fn a_closed_reader_ends_lez_with_sigpipe_and_no_message() {
    use std::os::unix::process::ExitStatusExt;

    let dir = fixture();
    for args in [
        &["-1"][..],
        &["-l"][..],
        &["-T"][..],
        &["-G"][..],
        &["--json"][..],
        &["--code"][..],
    ] {
        let output = run_into_closed_pipe(&dir, args);
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGPIPE),
            "{args:?} ended with {:?}",
            output.status
        );
        assert!(
            output.stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Without signals, the write fails with `BrokenPipe`, which lez treats as a
/// normal end of output rather than an error worth reporting.
#[test]
#[cfg(windows)]
fn a_closed_reader_ends_lez_successfully_and_quietly() {
    let dir = fixture();
    for args in [&["-1"][..], &["-l"][..], &["-T"][..], &["--json"][..]] {
        let output = run_into_closed_pipe(&dir, args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(
            output.stderr.is_empty(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
