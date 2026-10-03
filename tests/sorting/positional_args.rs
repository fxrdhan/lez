// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Paths named on the command line are sorted like a directory's entries:
//! files first, then each directory under its header, in the order the sort
//! field gives, reversed by `-r` and left as typed by `--sort=none`.

use std::fs::{File, FileTimes};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::common::{TempTestDir, lez_in, success_stdout};

fn listing(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

/// Sets the modification time of the file or, on Unix, the directory at
/// `path`. Windows opens a directory only with `FILE_FLAG_BACKUP_SEMANTICS`,
/// which std does not expose.
fn set_mtime(path: &Path, time: SystemTime) {
    let file = if path.is_dir() {
        File::open(path)
    } else {
        File::options().write(true).open(path)
    };
    file.and_then(|f| f.set_times(FileTimes::new().set_modified(time)))
        .expect("set the modification time");
}

/// `dir_a`, `dir_m` and `dir_z`, each holding one file named after it.
fn three_dirs(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for name in ["z", "a", "m"] {
        dir.create_file(&format!("dir_{name}/file_{name}.txt"), name.as_bytes());
    }
    dir
}

#[test]
fn directories_are_listed_by_name() {
    let dir = three_dirs("pos_dirs_default");
    assert_eq!(
        listing(&dir, &["-1", "dir_z", "dir_a", "dir_m"]),
        "dir_a:\nfile_a.txt\n\ndir_m:\nfile_m.txt\n\ndir_z:\nfile_z.txt\n"
    );
}

#[test]
fn directories_are_listed_in_reverse() {
    let dir = three_dirs("pos_dirs_reverse");
    assert_eq!(
        listing(&dir, &["-1", "-r", "dir_a", "dir_m", "dir_z"]),
        "dir_z:\nfile_z.txt\n\ndir_m:\nfile_m.txt\n\ndir_a:\nfile_a.txt\n"
    );
}

#[test]
fn sort_none_keeps_the_order_typed() {
    let dir = three_dirs("pos_dirs_none");
    for sort in [&["--sort=none"][..], &["-s", "none"]] {
        assert_eq!(
            listing(
                &dir,
                &[&["-1"], sort, &["dir_z", "dir_a", "dir_m"]].concat()
            ),
            "dir_z:\nfile_z.txt\n\ndir_a:\nfile_a.txt\n\ndir_m:\nfile_m.txt\n",
            "{sort:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn directories_are_listed_by_modification_time() {
    let dir = TempTestDir::new("pos_dirs_mtime");
    let now = SystemTime::now();
    for (name, age) in [("dir_old", 300), ("dir_mid", 150), ("dir_new", 10)] {
        dir.create_file(&format!("{name}/file.txt"), b"x");
        set_mtime(&dir.path().join(name), now - Duration::from_secs(age));
    }

    assert_eq!(
        listing(
            &dir,
            &["-1", "--sort=modified", "dir_new", "dir_old", "dir_mid"]
        ),
        "dir_old:\nfile.txt\n\ndir_mid:\nfile.txt\n\ndir_new:\nfile.txt\n"
    );
    assert_eq!(
        listing(
            &dir,
            &["-1", "--sort=newest", "dir_old", "dir_new", "dir_mid"]
        ),
        "dir_new:\nfile.txt\n\ndir_mid:\nfile.txt\n\ndir_old:\nfile.txt\n"
    );
}

#[test]
fn files_are_listed_by_extension() {
    let dir = TempTestDir::new("pos_files_ext");
    for name in ["item.zzz", "item.aaa", "item.mmm"] {
        dir.create_file(name, b"x");
    }
    let files = ["item.zzz", "item.aaa", "item.mmm"];

    assert_eq!(
        listing(&dir, &[&["-1", "--sort=extension"][..], &files].concat()),
        "item.aaa\nitem.mmm\nitem.zzz\n"
    );
    assert_eq!(
        listing(
            &dir,
            &[&["-1", "--sort=extension", "-r"][..], &files].concat()
        ),
        "item.zzz\nitem.mmm\nitem.aaa\n"
    );
}

#[test]
fn files_are_listed_by_size() {
    let dir = TempTestDir::new("pos_files_size");
    dir.create_file("large.txt", &[b'x'; 3000]);
    dir.create_file("small.txt", &[b'x'; 10]);
    dir.create_file("medium.txt", &[b'x'; 500]);
    let files = ["large.txt", "small.txt", "medium.txt"];

    assert_eq!(
        listing(&dir, &[&["-1", "--sort=size"][..], &files].concat()),
        "small.txt\nmedium.txt\nlarge.txt\n"
    );
    assert_eq!(
        listing(&dir, &[&["-1", "--sort=size", "-r"][..], &files].concat()),
        "large.txt\nmedium.txt\nsmall.txt\n"
    );
}

#[test]
fn files_are_listed_by_modification_time() {
    let dir = TempTestDir::new("pos_files_mtime");
    let now = SystemTime::now();
    for (name, age) in [
        ("file_old.txt", 300),
        ("file_mid.txt", 150),
        ("file_new.txt", 10),
    ] {
        set_mtime(&dir.create_file(name, b"x"), now - Duration::from_secs(age));
    }

    assert_eq!(
        listing(
            &dir,
            &[
                "-1",
                "--sort=modified",
                "file_new.txt",
                "file_old.txt",
                "file_mid.txt"
            ]
        ),
        "file_old.txt\nfile_mid.txt\nfile_new.txt\n"
    );
    assert_eq!(
        listing(
            &dir,
            &[
                "-1",
                "--sort=newest",
                "file_old.txt",
                "file_new.txt",
                "file_mid.txt"
            ]
        ),
        "file_new.txt\nfile_mid.txt\nfile_old.txt\n"
    );
}

#[test]
fn files_come_before_directories_in_either_direction() {
    let dir = TempTestDir::new("pos_mixed");
    dir.create_file("file_z.txt", b"z");
    dir.create_file("file_a.txt", b"a");
    dir.create_file("dir_z/child.txt", b"z");
    dir.create_file("dir_a/child.txt", b"a");
    let args = ["file_z.txt", "dir_z", "file_a.txt", "dir_a"];

    assert_eq!(
        listing(&dir, &[&["-1"][..], &args].concat()),
        "file_a.txt\nfile_z.txt\n\ndir_a:\nchild.txt\n\ndir_z:\nchild.txt\n"
    );
    assert_eq!(
        listing(&dir, &[&["-1", "-r"][..], &args].concat()),
        "file_z.txt\nfile_a.txt\n\ndir_z:\nchild.txt\n\ndir_a:\nchild.txt\n"
    );
}

/// Compared as printed: parsed, the object would come back with its keys
/// sorted whatever order lez wrote them in, since `serde_json` keeps no
/// insertion order here.
#[test]
fn json_keys_directories_in_sort_order() {
    let dir = TempTestDir::new("pos_json_dirs");
    for name in ["c", "a", "b"] {
        dir.create_file(&format!("dir_{name}/{name}.txt"), name.as_bytes());
    }

    assert_eq!(
        listing(&dir, &["--json", "dir_c", "dir_b", "dir_a"]),
        "{\"dir_a\":[\"a.txt\"],\"dir_b\":[\"b.txt\"],\"dir_c\":[\"c.txt\"]}\n"
    );
    assert_eq!(
        listing(&dir, &["--json", "-r", "dir_a", "dir_b", "dir_c"]),
        "{\"dir_c\":[\"c.txt\"],\"dir_b\":[\"b.txt\"],\"dir_a\":[\"a.txt\"]}\n"
    );
}

#[test]
fn json_lists_one_directory_as_its_sorted_names() {
    let dir = TempTestDir::new("pos_json_single_dir");
    for name in ["z", "a", "m"] {
        dir.create_file(&format!("mydir/{name}.txt"), name.as_bytes());
    }

    assert_eq!(
        listing(&dir, &["--json", "mydir"]),
        "[\"a.txt\",\"m.txt\",\"z.txt\"]\n"
    );
}

#[test]
fn a_missing_path_is_reported_and_the_rest_sorted() {
    let dir = TempTestDir::new("pos_files_missing");
    dir.create_file("file_z.txt", b"z");
    dir.create_file("file_a.txt", b"a");

    let output = lez_in(dir.path())
        .args(["-1", "file_z.txt", "does_not_exist.txt", "file_a.txt"])
        .output()
        .expect("failed to run lez");
    let missing = std::fs::metadata(dir.path().join("does_not_exist.txt")).expect_err("missing");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("\"does_not_exist.txt\": {missing}\n")
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "file_a.txt\nfile_z.txt\n"
    );
}

#[test]
fn empty_directories_are_listed_by_name() {
    let dir = TempTestDir::new("pos_empty_dirs");
    for name in ["empty_z", "empty_a", "empty_m"] {
        dir.create_dir(name);
    }

    assert_eq!(
        listing(&dir, &["-1", "empty_z", "empty_a", "empty_m"]),
        "empty_a:\n\nempty_m:\n\nempty_z:\n"
    );
}

#[test]
fn directories_taken_as_files_sort_among_the_files() {
    let dir = TempTestDir::new("pos_dirs_as_files");
    dir.create_dir("dir_z");
    dir.create_dir("dir_a");
    dir.create_file("file_m.txt", b"m");
    dir.create_file("a_file.txt", b"a");
    let args = ["dir_z", "file_m.txt", "dir_a", "a_file.txt"];

    assert_eq!(
        listing(&dir, &[&["-1", "-d"][..], &args].concat()),
        "a_file.txt\ndir_a\ndir_z\nfile_m.txt\n"
    );
    assert_eq!(
        listing(
            &dir,
            &[&["-1", "-d", "--group-directories-first"][..], &args].concat()
        ),
        "dir_a\ndir_z\na_file.txt\nfile_m.txt\n"
    );
}

#[cfg(unix)]
#[test]
fn an_extension_sort_reads_a_dereferenced_link_as_a_directory() {
    let dir = TempTestDir::new("sort_ext_deref");
    dir.create_dir("real_dir");
    dir.create_file("file.aaa", b"x");
    dir.create_symlink("real_dir", "link_dir.zzz");

    // A link's name has the extension `zzz`, which sorts after `aaa`.
    assert_eq!(
        listing(&dir, &["-1", "--sort=ext"]),
        "real_dir\nfile.aaa\nlink_dir.zzz\n"
    );
    // Dereferenced it is a directory, whose extension is not read.
    assert_eq!(
        listing(&dir, &["-1", "-X", "--sort=ext"]),
        "link_dir.zzz\nreal_dir\nfile.aaa\n"
    );
}
