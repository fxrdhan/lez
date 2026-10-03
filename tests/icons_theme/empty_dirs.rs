// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--icons` gives an empty directory a different glyph from a full one.
//! Working out which it is costs a `stat` for every directory listed, and a
//! read of its contents when the link count does not settle it. On a local
//! disk that is invisible; on a FUSE mount or a network share each one is a
//! round trip, which is what the reports behind this are about.
//!
//! `LEZ_NO_EMPTY_DIR_ICON` (or `EZA_`, `EXA_`) gives every directory the
//! full one's glyph and asks the filesystem nothing.

use crate::common::{TempTestDir, lez_cmd, success_stdout};

const DIRS: usize = 30;
const FULL: char = '\u{e5ff}';
const EMPTY: char = '\u{f115}';

/// `d00` with a file in it, then 29 empty directories.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("empty_dir_icon");
    dir.create_file("d00/inside", b"");
    for i in 1..DIRS {
        dir.create_dir(&format!("d{i:02}"));
    }
    dir
}

fn listing(dir: &TempTestDir, var: Option<(&str, &str)>) -> String {
    let mut cmd = lez_cmd();
    if let Some((name, value)) = var {
        cmd.env(name, value);
    }
    success_stdout(
        cmd.args(["-1", "--icons=always", "--color=never"])
            .arg(dir.path()),
    )
}

fn rows(glyph_of: impl Fn(usize) -> char) -> String {
    (0..DIRS)
        .map(|i| format!("{} d{i:02}\n", glyph_of(i)))
        .collect()
}

#[test]
fn an_empty_directory_looks_different_by_default() {
    let dir = fixture();
    assert_eq!(
        listing(&dir, None),
        rows(|i| if i == 0 { FULL } else { EMPTY })
    );
}

/// Presence is the switch, as with the other icon variables: an empty
/// value, or `0`, still turns the distinction off. Every directory then
/// gets the full one's glyph, so the listing never claims a directory is
/// empty without having looked.
#[test]
fn the_variable_gives_every_directory_the_full_glyph() {
    let dir = fixture();
    for name in [
        "LEZ_NO_EMPTY_DIR_ICON",
        "EZA_NO_EMPTY_DIR_ICON",
        "EXA_NO_EMPTY_DIR_ICON",
    ] {
        for value in ["1", "", "0"] {
            assert_eq!(
                listing(&dir, Some((name, value))),
                rows(|_| FULL),
                "{name}={value:?}"
            );
        }
    }
}

/// And the point of all this: with it set, the listing stops asking the
/// filesystem about each directory. `LEZ_DEBUG` logs every stat and every
/// read of a directory's contents.
#[cfg(unix)]
#[test]
fn the_variable_stops_the_filesystem_being_asked() {
    let dir = fixture();
    let trips = |var: Option<&str>| {
        let mut cmd = lez_cmd();
        if let Some(value) = var {
            cmd.env("LEZ_NO_EMPTY_DIR_ICON", value);
        }
        let output = cmd
            .env("LEZ_DEBUG", "trace")
            .args(["-1", "--icons=always", "--color=never"])
            .arg(dir.path())
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(0));
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
        let mut statted: Vec<String> = stderr
            .lines()
            .filter_map(|line| {
                line.split_once(" Statting file ")
                    .map(|(_, path)| path.to_owned())
            })
            .collect();
        statted.sort();
        let reads = stderr.matches("is_empty_directory: reading dir").count();
        (statted, reads)
    };
    let logged = |paths: &mut dyn Iterator<Item = std::path::PathBuf>| {
        let mut paths: Vec<String> = paths.map(|path| format!("{path:?}")).collect();
        paths.sort();
        paths
    };

    let (statted, reads) = trips(None);
    assert_eq!(
        statted,
        logged(
            &mut std::iter::once(dir.path().to_path_buf())
                .chain((0..DIRS).map(|i| dir.path().join(format!("d{i:02}"))))
        )
    );
    // Each empty directory has to be read; the full one may be settled by
    // its link count, depending on the filesystem.
    assert!(
        (DIRS - 1..=DIRS).contains(&reads),
        "{reads} reads for {DIRS} directories"
    );

    assert_eq!(
        trips(Some("1")),
        (logged(&mut std::iter::once(dir.path().to_path_buf())), 0)
    );
}
