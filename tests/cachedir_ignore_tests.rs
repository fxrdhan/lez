// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--cachedir-ignore`: directories carrying a valid `CACHEDIR.TAG` (see
//! <https://bford.info/cachedir/>) are hidden; bogus signatures and symlinks
//! never count.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHEDIR_MAGIC: &str = "Signature: 8a477f597d28d172789f06886806bc55";

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
            "lsr_cachedir_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &str) {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
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
    dir.create_file("keep.txt", "keep");
    dir.create_file("cache/data.bin", "data");
    dir.create_file("fake/junk.txt", "junk");
    dir.create_file("cache/CACHEDIR.TAG", CACHEDIR_MAGIC);
    dir.create_file("fake/CACHEDIR.TAG", "not the real magic");
    dir
}

#[test]
fn valid_cachedir_tag_hides_the_directory() {
    let fixture = fixture("valid");

    let stdout = run_lsr(&[
        "--cachedir-ignore",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(stdout.contains("keep.txt"), "{stdout}");
    assert!(
        stdout.contains("fake\n") || stdout.contains("fake"),
        "{stdout}"
    );
    assert!(
        !stdout.lines().any(|l| l.trim() == "cache"),
        "directory with a valid CACHEDIR.TAG must be hidden: {stdout}"
    );
}

#[test]
fn without_the_flag_everything_shows() {
    let fixture = fixture("off");

    let stdout = run_lsr(&["--color=never", fixture.path.to_str().unwrap()]);
    for name in ["keep.txt", "cache", "fake"] {
        assert!(stdout.contains(name), "{name} must be listed: {stdout}");
    }
}

#[test]
fn recursive_traversal_never_descends_into_tagged_dirs() {
    let fixture = fixture("recurse");
    fixture.create_file("cache/nested.txt", "nested");

    let stdout = run_lsr(&[
        "-T",
        "--cachedir-ignore",
        "--color=never",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(stdout.contains("keep.txt"), "{stdout}");
    assert!(
        !stdout.lines().any(|l| l.trim() == "cache"),
        "tagged directory must not appear as a tree row: {stdout}"
    );
    assert!(
        !stdout.contains("data.bin") && !stdout.contains("nested.txt"),
        "tagged directory contents must not be listed: {stdout}"
    );
}
