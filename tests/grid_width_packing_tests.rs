// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The grid used to size every column to the widest name in the whole
//! listing, so a set of names that fits a terminal exactly still spilled onto
//! a second row. Nine names of 4 to 9 characters need 79 columns with two
//! spaces between them; they belong on one row at 96.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const NAMES: [&str; 9] = [
    "code",
    "Desktop",
    "Documents",
    "Downloads",
    "Music",
    "Pictures",
    "Public",
    "Templates",
    "Videos",
];

fn listing(dir: &Path, width: &str) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .args(["--color=never", "-w", width])
        .arg(dir)
        .output()
        .expect("lsr should run");
    assert!(
        output.status.success(),
        "lsr exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.trim().is_empty())
        .collect()
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lsr_grid_{tag}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the fixture directory should be creatable");
        for name in NAMES {
            fs::write(path.join(name), b"").expect("the fixture file should be writable");
        }
        Self { path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn nine_names_that_need_79_columns_fit_one_row_at_96() {
    let dir = Fixture::new("fits");
    let rows = listing(&dir.path, "96");
    assert_eq!(
        rows.len(),
        1,
        "the names need 79 columns, so 96 is room to spare; got:\n{}",
        rows.join("\n")
    );
    for name in NAMES {
        assert!(rows[0].contains(name), "{name} is missing from the row");
    }
}

/// The row really is 79 columns wide, so one column less has to split it. This
/// keeps the test above honest: it would also pass if the grid simply ignored
/// the width.
#[test]
fn the_same_names_split_when_the_width_cannot_hold_them() {
    let dir = Fixture::new("splits");
    assert!(
        listing(&dir.path, "78").len() > 1,
        "78 columns cannot hold a 79-column row"
    );
}
