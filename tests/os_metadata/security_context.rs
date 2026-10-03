// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-Z` adds a security-context column: on Linux the file's SELinux label,
//! read from `security.selinux`, and `?` where there is none or the system
//! has no SELinux, as `ls -Z` prints. Other systems have no SELinux, so the
//! column shows `?` there; it used to be left out without a word. How a
//! label is split and coloured is unit tested in
//! `src/output/render/securityctx.rs`. JSON leaves an unknown context out.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// The label lez should show for `file`: its `security.selinux`, without
/// the NUL the kernel keeps at the end, or `?`.
#[cfg(target_os = "linux")]
fn expected_context(file: &std::path::Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(file.as_os_str().as_bytes()).expect("no NUL");
    let mut buffer = vec![0u8; 1024];
    // SAFETY: valid NUL-terminated strings and a buffer of its length.
    let length = unsafe {
        libc::getxattr(
            path.as_ptr(),
            c"security.selinux".as_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if let Ok(length) = usize::try_from(length) {
        buffer.truncate(length);
        let label = String::from_utf8_lossy(&buffer);
        return label.trim_end_matches('\0').to_owned();
    }
    "?".to_owned()
}

#[cfg(not(target_os = "linux"))]
fn expected_context(_file: &std::path::Path) -> String {
    "?".to_owned()
}

#[test]
fn the_context_column_shows_the_label_or_a_question_mark() {
    let dir = TempTestDir::new("security_context");
    let file = dir.create_file("test.txt", b"content");
    let context = expected_context(&file);
    let width = context.len().max("Security Context".len());
    let long = |args: &[&str]| {
        success_stdout(
            lez_in(dir.path())
                .args([
                    "-l",
                    "-Z",
                    "--no-permissions",
                    "--no-filesize",
                    "--no-user",
                    "--no-time",
                ])
                .args(args),
        )
    };
    assert_eq!(long(&[]), format!("{context} test.txt\n"));
    assert_eq!(
        long(&["-h"]),
        format!(
            "{:<width$} Name\n{context:<width$} test.txt\n",
            "Security Context"
        )
    );
    let json = long(&["--json"]);
    if context == "?" {
        assert_eq!(json, "{\"test.txt\":{}}\n");
    }
}
