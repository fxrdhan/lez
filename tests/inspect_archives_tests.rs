// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--inspect-archives`: uncompressed `.tar` files list their entries below
//! themselves in the long view; corrupt archives fail silently and are
//! listed like regular files.

use std::fs::{self, File};
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
            "lsr_inspect_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn make_tar(&self, name: &str) {
        let tar_path = self.path.join(name);
        let file = File::create(&tar_path).unwrap();
        let mut builder = tar::Builder::new(file);

        fn add(builder: &mut tar::Builder<File>, rel: &str, content: &[u8]) {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, rel, content).unwrap();
        }
        add(&mut builder, "inner.txt", b"hello");
        add(&mut builder, "nested/deep.bin", b"data");
        builder.into_inner().unwrap();
    }

    fn write(&self, name: &str, content: &str) {
        fs::write(self.path.join(name), content).unwrap();
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The entries the fixture archive holds, in listing order.
const ENTRIES: [&str; 2] = ["foo.tar/inner.txt", "foo.tar/nested/deep.bin"];

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
    dir.make_tar("foo.tar");
    dir.write("broken.tar", "this is definitely not a tar archive");
    dir.write("plain.txt", "plain");
    dir
}

#[test]
fn long_view_lists_tar_entries_below_the_archive() {
    let fixture = fixture("list");

    let stdout = run_lsr(&[
        "-1",
        "-l",
        "--color=never",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(stdout.contains("foo.tar"), "{stdout}");
    assert!(
        stdout.contains("foo.tar/inner.txt"),
        "flat entry must be listed: {stdout}"
    );
    assert!(
        stdout.contains("foo.tar/nested/deep.bin"),
        "nested entry must be listed with its archive path: {stdout}"
    );
}

/// The last entry closes the branch. Every row used to sit on an edge, so the
/// listing never terminated — asserting only that some connector was present
/// could not tell the two apart.
#[test]
fn the_last_archive_entry_closes_the_branch() {
    let fixture = fixture("edges");

    let stdout = run_lsr(&[
        "-1",
        "-l",
        "--color=never",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);

    let rows: Vec<&str> = stdout
        .lines()
        .filter(|l| ENTRIES.iter().any(|e| l.contains(e)))
        .collect();
    assert_eq!(rows.len(), ENTRIES.len(), "both entries listed: {stdout}");

    let (last, rest) = rows.split_last().expect("at least one entry");
    for row in rest {
        assert!(
            row.contains('\u{251c}') && !row.contains('\u{2514}'),
            "a non-final entry stays on an edge: {row}"
        );
    }
    assert!(
        last.contains('\u{2514}') && !last.contains('\u{251c}'),
        "the final entry closes the branch: {last}"
    );
}

#[test]
fn without_the_flag_archives_stay_opaque() {
    let fixture = fixture("off");

    let stdout = run_lsr(&["-l", "--color=never", fixture.path.to_str().unwrap()]);
    assert!(stdout.contains("foo.tar"), "{stdout}");
    assert!(!stdout.contains("inner.txt"), "{stdout}");
}

#[test]
fn corrupt_archive_fails_silently() {
    let fixture = fixture("corrupt");

    let stdout = run_lsr(&[
        "-l",
        "--color=never",
        "--inspect-archives",
        fixture.path.to_str().unwrap(),
    ]);
    assert!(stdout.contains("broken.tar"), "{stdout}");
    assert!(
        !stdout.lines().any(|l| l.contains("broken.tar/")),
        "no entries may be invented for a corrupt archive: {stdout}"
    );
}
