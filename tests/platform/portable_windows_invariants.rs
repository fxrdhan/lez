// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Windows path syntax handed to lez on any platform: verbatim, UNC and
//! device paths, alternate data streams, and case-insensitive ignore
//! globs. On Windows a file's name is the last component of whichever
//! form its path takes; elsewhere these are just names that do not exist,
//! and are reported as such.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lez::fs::File;
use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;

struct WindowsTestDir {
    path: PathBuf,
}

impl WindowsTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_win_inv_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for WindowsTestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A file's name is the last component of its path, in every form Windows
/// writes one.
#[cfg(windows)]
#[test]
fn a_files_name_is_the_last_component_of_any_windows_path() {
    for (path, name, ext) in [
        (r"\\?\C:\Users\someone\file.rs", "file.rs", Some("rs")),
        (
            r"\\?\UNC\server\share\document.pdf",
            "document.pdf",
            Some("pdf"),
        ),
        (r"C:\Windows\System32\cmd.exe", "cmd.exe", Some("exe")),
        ("D:/Games/steamapps/common/app.bin", "app.bin", Some("bin")),
        (
            r"\\?\Volume{12345678-1234-1234-1234-1234567890ab}\boot.ini",
            "boot.ini",
            Some("ini"),
        ),
        (r"C:\dir\Makefile", "Makefile", None),
    ] {
        let file = File::from_args(PathBuf::from(path), None, None, false, false, false, None);
        assert_eq!(
            (file.name.as_str(), file.ext.as_deref()),
            (name, ext),
            "{path}"
        );
    }
}

/// A path in Windows syntax that leads nowhere is reported missing, exit 2,
/// on every platform; never a panic. On Windows the reason is the
/// system's own words.
#[test]
fn windows_paths_that_lead_nowhere_are_reported_missing() {
    for path in [
        r"C:\non_existent_folder_12345",
        r"\\?\C:\non_existent_folder_67890",
        "file.txt:Zone.Identifier",
        "archive.zip:summary:$DATA",
    ] {
        let dir = WindowsTestDir::new("missing");
        let output = crate::common::lez_in(&dir.path)
            .arg(path)
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(2), "{path}");
        assert!(output.stdout.is_empty(), "{path}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.starts_with(&format!("{path:?}: ")), "{stderr}");
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
        #[cfg(unix)]
        assert_eq!(
            stderr,
            format!(
                "{path:?}: {}\n",
                std::io::Error::from_raw_os_error(libc::ENOENT)
            )
        );
    }
}

#[test]
fn test_case_insensitive_filtering_invariants() {
    // Test case-insensitive ignore globs via --ignore-glob-ci
    let matches = get_command()
        .try_get_matches_from(["lez", "--ignore-glob-ci", "*.tmp|*.bak|thumbs.db"])
        .expect("Failed to parse case-insensitive glob");

    struct EmptyVars;
    impl Vars for EmptyVars {
        fn get(&self, _name: &'static str) -> Option<std::ffi::OsString> {
            None
        }
    }

    let options = Options::deduce(&matches, &EmptyVars).expect("Failed to deduce options");
    let filter = options.filter;

    // Filter should ignore regardless of casing
    assert!(filter.ignore_patterns_caseins.is_ignored("cache.tmp"));
    assert!(filter.ignore_patterns_caseins.is_ignored("CACHE.TMP"));
    assert!(filter.ignore_patterns_caseins.is_ignored("backup.bak"));
    assert!(filter.ignore_patterns_caseins.is_ignored("BACKUP.BAK"));
    assert!(filter.ignore_patterns_caseins.is_ignored("thumbs.db"));
    assert!(filter.ignore_patterns_caseins.is_ignored("THUMBS.DB"));
    assert!(!filter.ignore_patterns_caseins.is_ignored("normal.rs"));
}

/// `--ignore-glob-ci` leaves out names whatever their case.
#[test]
fn case_insensitive_ignore_globs_on_real_files() {
    let dir = WindowsTestDir::new("ci_glob");
    for name in [
        "Document.PDF",
        "DOCUMENT.TXT",
        "document.log",
        "image.PNG",
        "IMAGE.jpg",
    ] {
        dir.create_file(name, name.as_bytes());
    }
    let listing = |glob: &str| {
        crate::common::success_stdout(
            crate::common::lez_in(&dir.path).args(["-1", &format!("--ignore-glob-ci={glob}")]),
        )
    };
    assert_eq!(
        listing("*.pdf|*.png"),
        "document.log\nDOCUMENT.TXT\nIMAGE.jpg\n"
    );
    assert_eq!(listing("document.*"), "IMAGE.jpg\nimage.PNG\n");
}
