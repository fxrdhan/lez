// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--inspect-archives`: uncompressed `.tar` files list their entries below
//! themselves in the long view; corrupt archives fail silently and are
//! listed like regular files.

use std::fs::File;
use std::path::Path;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, grouped, lez_in, success_stdout};

/// Writes a tar archive of regular files, in the order given.
fn write_tar(path: &Path, entries: &[(&str, &[u8])]) {
    let mut builder = tar::Builder::new(File::create(path).expect("create the archive"));
    for (rel, content) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, rel, *content)
            .expect("append an entry");
    }
    builder.into_inner().expect("finish the archive");
}

/// `foo.tar` holds `inner.txt` (5 bytes), `nested/deep.bin` (4 bytes) and
/// `nested/main.rs` (12 bytes), next to a `.tar` that is not an archive
/// and a plain file.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    write_tar(
        &dir.path().join("foo.tar"),
        &[
            ("inner.txt", b"hello"),
            ("nested/deep.bin", b"data"),
            // A name the default theme has a rule for, so the colouring of
            // the leaf can be told apart from the punctuation around it.
            ("nested/main.rs", b"fn main() {}"),
        ],
    );
    dir.create_file("broken.tar", b"this is definitely not a tar archive");
    dir.create_file("plain.txt", b"plain");
    dir
}

/// The long view of the fixture, reduced to the name column.
fn names(fixture: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(fixture.path()).args(NAME_COLUMN_ONLY).args(args))
}

/// Each entry hangs below the archive with its path inside it and its
/// size; the last one closes the branch. A file that is not a tar archive
/// gets no entries, and nothing is reported.
#[test]
fn the_long_view_lists_tar_entries_below_the_archive() {
    let fixture = fixture("list");
    assert_eq!(
        names(&fixture, &["--inspect-archives"]),
        "broken.tar\n\
         foo.tar\n\
         ├── foo.tar/inner.txt (5 B)\n\
         ├── foo.tar/nested/deep.bin (4 B)\n\
         └── foo.tar/nested/main.rs (12 B)\n\
         plain.txt\n"
    );
}

/// Entries come in the order the archive stores them, not sorted, and
/// sizes past 1023 bytes use binary units.
#[test]
fn entries_keep_the_archive_order_and_binary_sizes() {
    let dir = TempTestDir::new("inspect_order");
    write_tar(
        &dir.path().join("ordered.tar"),
        &[("zeta.bin", &[0; 1536]), ("alpha.txt", b"abc")],
    );
    assert_eq!(
        names(&dir, &["--inspect-archives"]),
        "ordered.tar\n\
         ├── ordered.tar/zeta.bin (1.5 KiB)\n\
         └── ordered.tar/alpha.txt (3 B)\n"
    );
}

/// In a tree the entries hang one level below the archive, under the
/// branch of the directory that holds it.
#[test]
fn in_a_tree_entries_hang_below_the_archive() {
    let fixture = fixture("tree");
    assert_eq!(
        names(&fixture, &["-T", "--inspect-archives"]),
        ".\n\
         ├── broken.tar\n\
         ├── foo.tar\n\
         │   ├── foo.tar/inner.txt (5 B)\n\
         │   ├── foo.tar/nested/deep.bin (4 B)\n\
         │   └── foo.tar/nested/main.rs (12 B)\n\
         └── plain.txt\n"
    );
}

/// Only regular files whose name ends in `.tar`, in any case, are opened:
/// not a directory with that name, and not a compressed `.tar.gz`, even
/// when its contents are an uncompressed archive.
#[test]
fn only_files_named_dot_tar_are_opened() {
    let dir = TempTestDir::new("inspect_names");
    write_tar(&dir.path().join("UPPER.TAR"), &[("a.txt", b"a")]);
    write_tar(&dir.path().join("plain.tar.gz"), &[("b.txt", b"b")]);
    dir.create_file("folder.tar/c.txt", b"c");
    assert_eq!(
        names(&dir, &["--inspect-archives"]),
        "folder.tar\nplain.tar.gz\nUPPER.TAR\n└── UPPER.TAR/a.txt (1 B)\n"
    );
}

/// Outside the long view the flag does nothing, and JSON lists the archive
/// as the file it is.
#[test]
fn without_the_flag_or_the_long_view_archives_stay_opaque() {
    let fixture = fixture("off");
    assert_eq!(names(&fixture, &[]), "broken.tar\nfoo.tar\nplain.txt\n");
    assert_eq!(
        success_stdout(lez_in(fixture.path()).args(["-1", "--inspect-archives"])),
        "broken.tar\nfoo.tar\nplain.txt\n"
    );

    let json: serde_json::Value =
        serde_json::from_str(&success_stdout(lez_in(fixture.path()).args([
            "--json",
            "-lB",
            "--inspect-archives",
            "--no-permissions",
            "--no-user",
            "--no-time",
            "foo.tar",
        ])))
        .expect("valid JSON");
    let size = std::fs::metadata(fixture.path().join("foo.tar"))
        .expect("stat the archive")
        .len();
    assert_eq!(
        json,
        serde_json::json!({"foo.tar": {"Size": grouped(size)}})
    );
}

/// The archive path and the size stay in the punctuation style; the leaf
/// name takes its own file colour when the theme has one (`.txt`, `.rs`),
/// and keeps the punctuation style when it has none (`.bin`).
#[test]
fn an_entrys_leaf_name_is_coloured_by_type() {
    let fixture = fixture("colour");
    assert_eq!(
        names(
            &fixture,
            &["--inspect-archives", "--color=always", "foo.tar"]
        ),
        "\u{1b}[31mfoo.tar\u{1b}[0m\n\
         \u{1b}[1;90m├── foo.tar/\u{1b}[0m\u{1b}[32minner.txt\u{1b}[1;90m (5 B)\u{1b}[0m\n\
         \u{1b}[1;90m├── foo.tar/nested/deep.bin (4 B)\u{1b}[0m\n\
         \u{1b}[1;90m└── foo.tar/nested/\u{1b}[33mmain.rs\u{1b}[90m (12 B)\u{1b}[0m\n"
    );
}
