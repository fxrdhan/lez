// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Flags whose value is optional must be given that value with an equals
//! sign. Without that, clap treats the next word as the value, so a shell
//! glob such as `lsr --color *.md` is rejected outright and
//! `lsr -T --absolute /some/path` never gets its tree root.

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
            "lsr_optional_value_flags_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, b"").unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_lsr"))
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

/// The shape a shell hands us after expanding `lsr --color *.md`.
#[test]
fn optional_value_flags_list_the_paths_a_glob_expands_to() {
    let fixture = TempTestDir::new("glob");
    let first = fixture.create_file("alpha.md");
    let second = fixture.create_file("beta.md");

    for flag in ["--color", "--colour", "--absolute", "--color-scale"] {
        let output = run_lsr(&[flag, first.to_str().unwrap(), second.to_str().unwrap()]);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "{flag} rejected the paths that followed it: {stderr}"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("alpha.md"), "{flag}: missing alpha.md");
        assert!(stdout.contains("beta.md"), "{flag}: missing beta.md");
    }
}

/// Upstream eza#995: `--absolute` used to swallow the tree root, so this
/// printed a usage error instead of a tree.
#[test]
fn absolute_tree_accepts_an_explicit_root() {
    let fixture = TempTestDir::new("tree");
    fixture.create_file("nested/leaf.txt");

    // The root has to sit directly behind the flag: that adjacency is what
    // used to make clap read it as the flag's value.
    let root = fixture.path.to_str().unwrap();
    let output = run_lsr(&["-T", "--absolute", root]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "--absolute -T failed: {stderr}");

    // Compare on the components we created rather than the whole path:
    // Windows hands back a short 8.3 temp prefix that the binary resolves to
    // its long form, so only the tail is stable across platforms.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let dir_name = fixture.path.file_name().unwrap().to_str().unwrap();
    let leaf_line = stdout
        .lines()
        .find(|line| line.contains("leaf.txt"))
        .unwrap_or_else(|| panic!("no leaf.txt in the tree:\n{stdout}"));

    assert!(
        leaf_line.contains(dir_name),
        "the leaf should carry its whole path, got: {leaf_line}"
    );
    assert!(
        leaf_line.contains("nested"),
        "the leaf should carry its parent directory, got: {leaf_line}"
    );
}

/// The equals form is the documented one, and it still carries the value.
#[test]
fn attached_values_are_still_honoured() {
    let fixture = TempTestDir::new("attached");
    let file = fixture.create_file("gamma.txt");
    let path = file.to_str().unwrap();

    let absolute = run_lsr(&["--absolute=on", "--color=never", path]);
    assert!(absolute.status.success());
    assert!(
        String::from_utf8_lossy(&absolute.stdout).contains(path),
        "--absolute=on should print the absolute path"
    );

    let plain = run_lsr(&["--absolute=off", "--color=never", path]);
    assert!(plain.status.success());
    assert_eq!(
        String::from_utf8_lossy(&plain.stdout).trim(),
        path,
        "an explicit path argument is echoed as given"
    );

    let always = run_lsr(&["--color=always", path]);
    assert!(always.status.success());
    assert!(
        String::from_utf8_lossy(&always.stdout).contains('\u{1b}'),
        "--color=always should emit escape sequences even when piped"
    );
}

/// A value handed over with a space is a path, exactly as it is for
/// `ls --color always`.
#[test]
fn a_spaced_value_is_listed_as_a_path() {
    let fixture = TempTestDir::new("spaced");
    fixture.create_file("always");
    // A second entry separates the two readings: listing only `always`
    // means it was taken as a path, listing both means it was taken as the
    // value of --color and the whole directory was listed instead.
    fixture.create_file("bystander.txt");

    let output = Command::new(env!("CARGO_BIN_EXE_lsr"))
        .current_dir(&fixture.path)
        .args(["--color", "always"])
        .output()
        .expect("Failed to execute lsr binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("always"),
        "the word after a bare --color is the file to list, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("bystander.txt"),
        "--color swallowed its neighbour and listed the directory:\n{stdout}"
    );
}
