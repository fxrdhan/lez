// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--warn-hidden` / `-W`: report entries filtered out by visibility rules.
//! Given once, the tally appears only when something was hidden; given
//! twice, it always prints the numbers.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

struct TempTestDir {
    inner: TempDir,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let inner = tempfile::Builder::new()
            .prefix(&format!("lez_warn_hidden_{prefix}_"))
            .tempdir()
            .expect("Failed to create temp test directory");
        Self { inner }
    }

    fn path(&self) -> &Path {
        self.inner.path()
    }

    fn create_file(&self, rel_path: &str) {
        let file_path = self.path().join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, "x").unwrap();
    }
}

fn run_lez(args: &[&str]) -> (String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .args(args)
        .output()
        .expect("Failed to execute lez binary");
    assert!(output.status.success());
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
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

    let (stdout, stderr) = run_lez(&[
        "-1",
        "--color=never",
        "-W",
        fixture.path().join("clean").to_str().unwrap(),
    ]);
    assert!(stdout.contains("inner.txt"), "{stdout}");
    assert!(
        !stdout.contains("hidden"),
        "no tally in stdout without filtered entries: {stdout}"
    );
    assert!(
        !stderr.contains("hidden"),
        "no tally in stderr without filtered entries: {stderr}"
    );
}

#[test]
fn warn_hidden_reports_once_something_was_hidden() {
    let fixture = fixture("auto");

    let (stdout, stderr) = run_lez(&[
        "-1",
        "--color=never",
        "-W",
        fixture.path().to_str().unwrap(),
    ]);
    assert!(stdout.contains("visible.txt"), "{stdout}");
    assert!(
        !stdout.contains("hidden items"),
        "stdout must remain pure data payload without warnings: {stdout}"
    );
    assert!(
        stderr.contains("hidden items"),
        "stderr must contain the warning tally: {stderr}"
    );
}

#[test]
fn warn_hidden_twice_always_prints_the_tally() {
    let fixture = fixture("verbose");

    // A directory whose contents are all visible still gets a tally line on stderr.
    let (stdout, stderr) = run_lez(&[
        "-1",
        "--color=never",
        "-WW",
        fixture.path().join("clean").to_str().unwrap(),
    ]);
    assert!(
        !stdout.contains("0 hidden and 0 ignored"),
        "stdout must remain pure: {stdout}"
    );
    assert!(
        stderr.contains("0 hidden and 0 ignored"),
        "double flag forces the tally to stderr: {stderr}"
    );

    let (stdout, stderr) = run_lez(&[
        "-1",
        "--color=never",
        "-WW",
        fixture.path().to_str().unwrap(),
    ]);
    assert!(stdout.contains("visible.txt"), "{stdout}");
    assert!(
        !stdout.contains("1 hidden"),
        "stdout must remain pure: {stdout}"
    );
    assert!(
        stderr.contains("1 hidden"),
        "double flag prints tally to stderr: {stderr}"
    );
}
