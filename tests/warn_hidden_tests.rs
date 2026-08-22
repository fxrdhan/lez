// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--warn-hidden` / `-W`: report entries filtered out by visibility rules.
//! Given once, the tally appears only when something was hidden; given
//! twice, it always prints the numbers.

use std::fs;
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
            "lsr_warn_hidden_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str) {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, "x").unwrap();
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .args(args)
        .output()
        .expect("Failed to execute lsr binary");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("visible.txt");
    dir.create_file(".secret");
    dir.create_file("clean/inner.txt");
    dir
}

#[test]
fn warn_hidden_stays_silent_when_nothing_was_filtered() {
    let fixture = fixture("silent");

    let stdout = run_lsr(&[
        "-1",
        "--color=never",
        "-W",
        fixture.path.join("clean").to_str().unwrap(),
    ]);
    assert!(stdout.contains("inner.txt"), "{stdout}");
    assert!(
        !stdout.contains("hidden"),
        "no tally without filtered entries: {stdout}"
    );
}

#[test]
fn warn_hidden_reports_once_something_was_hidden() {
    let fixture = fixture("auto");

    let stdout = run_lsr(&["-1", "--color=never", "-W", fixture.path.to_str().unwrap()]);
    assert!(stdout.contains("visible.txt"), "{stdout}");
    assert!(
        stdout.contains("hidden items"),
        "must warn about the hidden dotfile: {stdout}"
    );
}

#[test]
fn warn_hidden_twice_always_prints_the_tally() {
    let fixture = fixture("verbose");

    // A directory whose contents are all visible still gets a tally line.
    let stdout = run_lsr(&[
        "-1",
        "--color=never",
        "-WW",
        fixture.path.join("clean").to_str().unwrap(),
    ]);
    assert!(
        stdout.contains("0 hidden and 0 ignored"),
        "double flag forces the tally: {stdout}"
    );

    let stdout = run_lsr(&["-1", "--color=never", "-WW", fixture.path.to_str().unwrap()]);
    assert!(stdout.contains("1 hidden"), "{stdout}");
}
