// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The shapes `--json` writes: an array of names for one directory, an
//! object keyed by path for several arguments, an object of columns per
//! file in the long view, and `files`/`directories` nesting under
//! recursion. Each case compares the document as printed: parsed, an
//! object would come back with its keys sorted whatever order lez wrote
//! them in.

use std::fs;
use std::time::{Duration, UNIX_EPOCH};

#[cfg(unix)]
use crate::common::symlink_permissions;
use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

fn json(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).env("TZ", "UTC").arg("--json").args(args))
}

/// Long-view columns pinned down to the ones a test asks for.
const BARE: [&str; 5] = [
    "-l",
    "--no-permissions",
    "--no-filesize",
    "--no-user",
    "--no-time",
];

#[test]
fn one_directory_is_an_array_of_names() {
    let dir = TempTestDir::new("json_names");
    dir.create_file("full/alpha.txt", b"a");
    dir.create_file("full/beta.rs", b"b");
    dir.create_file("full/.secret", b"s");
    dir.create_dir("full/gamma_dir");
    dir.create_dir("empty");
    dir.create_file("solo.txt", b"solo");

    assert_eq!(
        json(&dir, &["full"]),
        "[\"alpha.txt\",\"beta.rs\",\"gamma_dir\"]\n"
    );
    assert_eq!(
        json(&dir, &["-a", "full"]),
        "[\".secret\",\"alpha.txt\",\"beta.rs\",\"gamma_dir\"]\n"
    );
    assert_eq!(
        json(&dir, &["-aa", "full"]),
        "[\".\",\"..\",\".secret\",\"alpha.txt\",\"beta.rs\",\"gamma_dir\"]\n"
    );
    assert_eq!(json(&dir, &["empty"]), "[]\n");
    assert_eq!(json(&dir, &["solo.txt"]), "[\"solo.txt\"]\n");
}

/// Several arguments are keyed by the path given, so two directories that
/// share a name stay apart; files and directories together are split. Files
/// in the long view are keyed by name, unless two share one: then every
/// file is keyed by its path.
#[test]
fn several_arguments_are_keyed_by_path() {
    let dir = TempTestDir::new("json_arguments");
    dir.create_file("parent1/common/a.txt", b"a");
    dir.create_file("parent2/common/b.txt", b"b");
    dir.create_file("top.txt", b"top");
    dir.create_file("x/same.txt", b"");
    dir.create_file("x/other.txt", b"");
    dir.create_file("y/same.txt", b"");

    assert_eq!(
        json(&dir, &["parent1/common", "parent2/common"]),
        "{\"parent1/common\":[\"a.txt\"],\"parent2/common\":[\"b.txt\"]}\n"
    );
    assert_eq!(
        json(&dir, &["top.txt", "parent1/common"]),
        "{\"files\":[\"top.txt\"], \"directories\":{\"parent1/common\":[\"a.txt\"]}}\n"
    );
    assert_eq!(
        json(&dir, &[&BARE[..], &["x/other.txt", "y/same.txt"]].concat()),
        "{\"other.txt\":{},\"same.txt\":{}}\n"
    );
    assert_eq!(
        json(
            &dir,
            &[&BARE[..], &["x/same.txt", "y/same.txt", "x/other.txt"]].concat()
        ),
        "{\"x/other.txt\":{},\"x/same.txt\":{},\"y/same.txt\":{}}\n"
    );
    assert_eq!(
        json(&dir, &["x/same.txt", "y/same.txt"]),
        "[\"same.txt\",\"same.txt\"]\n"
    );
}

/// The long view gives each file an object of its columns, under the
/// table's headings; colours never reach it.
#[test]
fn the_long_view_is_an_object_of_columns() {
    let dir = TempTestDir::new("json_long");
    let file = dir.create_file("test.txt", b"content of test file");
    #[cfg(unix)]
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o644)).expect("chmod");
    // 2023-11-14 22:13:20 UTC: old enough for the full-date form of `iso`.
    fs::File::options()
        .write(true)
        .open(&file)
        .and_then(|f| f.set_modified(UNIX_EPOCH + Duration::from_secs(1_700_000_000)))
        .expect("set the modified time");
    dir.create_file("large.bin", &vec![0; 1024 * 1024]);
    dir.create_dir("empty");

    #[cfg(unix)]
    for colour in ["never", "always"] {
        assert_eq!(
            json(
                &dir,
                &[
                    "-l",
                    "--no-user",
                    "--no-time",
                    "-o",
                    &format!("--color={colour}"),
                    "test.txt"
                ]
            ),
            "{\"test.txt\":{\"Octal\": \"0644\",\"Permissions\": \".rw-r--r--\",\"Size\": \"20\"}}\n"
        );
    }
    let size = |flag: &str| {
        json(
            &dir,
            &[
                "-l",
                "--no-permissions",
                "--no-user",
                "--no-time",
                flag,
                "large.bin",
            ],
        )
    };
    assert_eq!(
        size("--bytes"),
        format!(
            "{{\"large.bin\":{{\"Size\": \"{}\"}}}}\n",
            grouped(1_048_576)
        )
    );
    assert_eq!(size("--binary"), "{\"large.bin\":{\"Size\": \"1.0Mi\"}}\n");
    assert_eq!(
        json(
            &dir,
            &[
                "-l",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--time-style=iso",
                "test.txt"
            ]
        ),
        "{\"test.txt\":{\"Date Modified\": \"2023-11-14\"}}\n"
    );
    assert_eq!(json(&dir, &["-l", "empty"]), "{}\n");
}

/// A link's target is given as the link holds it, relative or absolute.
#[test]
#[cfg(unix)]
fn a_link_carries_its_target() {
    let dir = TempTestDir::new("json_links");
    let target = dir.create_file("target.txt", b"target");
    dir.create_symlink("target.txt", "relative.txt");
    std::os::unix::fs::symlink(&target, dir.path().join("absolute.txt")).expect("symlink");

    assert_eq!(
        json(
            &dir,
            &[
                "-l",
                "--no-user",
                "--no-time",
                "relative.txt",
                "absolute.txt"
            ]
        ),
        format!(
            "{{\"absolute.txt\":{{\"Permissions\": \"{}\",\"Target\": \"{}\"}},\
             \"relative.txt\":{{\"Permissions\": \"{}\",\"Target\": \"target.txt\"}}}}\n",
            symlink_permissions(&dir.path().join("absolute.txt")),
            target.display(),
            symlink_permissions(&dir.path().join("relative.txt")),
        )
    );
}

/// Recursion nests each directory's `files` and `directories`, keyed by
/// name; a link back up the tree is a file and is not followed.
#[test]
fn recursion_nests_files_and_directories() {
    let dir = TempTestDir::new("json_recursion");
    dir.create_file("top/root_file.txt", b"");
    dir.create_file("top/sub/nested_file.txt", b"");

    let tree = "{\"top\":{\"files\":[\"root_file.txt\"], \"directories\":\
                {\"sub\":{\"files\":[\"nested_file.txt\"], \"directories\":{}}}}}\n";
    assert_eq!(json(&dir, &["-R", "top"]), tree);
    assert_eq!(
        json(
            &dir,
            &["-R", dir.path().join("top").to_str().expect("UTF-8")]
        ),
        tree
    );
    assert_eq!(
        json(
            &dir,
            &[
                "-R",
                "-l",
                "--no-permissions",
                "--no-user",
                "--no-time",
                "top"
            ]
        ),
        "{\"top\":{\"files\":{\"root_file.txt\":{\"Size\": \"0\"}}, \"directories\":\
         {\"sub\":{\"files\":{\"nested_file.txt\":{\"Size\": \"0\"}}, \"directories\":{}}}}}\n"
    );

    #[cfg(unix)]
    {
        let cycle = dir.create_dir("cycle");
        dir.create_file("cycle/hello.txt", b"");
        std::os::unix::fs::symlink(&cycle, cycle.join("loop")).expect("symlink");
        assert_eq!(
            json(&dir, &["-R", "cycle"]),
            "{\"cycle\":{\"files\":[\"hello.txt\",\"loop\"], \"directories\":{}}}\n"
        );
    }
}

/// Names are JSON strings, escaped as JSON escapes them.
#[test]
fn names_are_escaped_as_json_strings() {
    let dir = TempTestDir::new("json_escaping");
    // Characters Windows does not allow in a name.
    let unix_only: &[&str] = if cfg!(unix) {
        &[
            "back\\slash",
            "ctl\u{1}x",
            "file\"with\"quotes.txt",
            "tab\tname",
        ]
    } else {
        &[]
    };
    let names = [
        "emoji_🚀_tag.txt",
        "file with spaces.txt",
        "unicode_日本語_test.txt",
    ];
    for &name in names.iter().chain(unix_only) {
        dir.create_file(name, b"");
        assert_eq!(
            json(&dir, &[name]),
            format!(
                "[{}]\n",
                serde_json::to_string(name).expect("a JSON string")
            ),
            "{name:?}"
        );
    }
}

#[test]
#[cfg(feature = "git")]
fn git_status_is_a_column_like_any_other() {
    crate::common::require_git();
    let repo = crate::common::TempGitRepo::new("json_git");
    repo.create_file("tracked.txt", b"initial");
    repo.create_file("untracked/inner.txt", b"");
    repo.git(&["add", "tracked.txt"]);
    repo.create_file("tracked.txt", b"modified");

    assert_eq!(
        success_stdout(lez_in(repo.path()).args(BARE).args(["--git", "--json"])),
        "{\"tracked.txt\":{\"Git\": \"NM\"},\"untracked\":{\"Git\": \"-N\"}}\n"
    );
}

#[test]
#[cfg(unix)]
fn test_json_permission_denied_exit_code_and_json() {
    if !crate::common::permission_checks_apply() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let temp = TempTestDir::new("json_perm_denied");
    let restricted = temp.create_dir("restricted");
    temp.create_file("restricted/secret.txt", b"secret");
    fs::set_permissions(&restricted, fs::Permissions::from_mode(0o000)).expect("lock");

    let output = lez_in(temp.path())
        .args(["--json", "restricted"])
        .output()
        .expect("Failed to run lez");

    // Restore permissions so drop cleanup succeeds
    let _ = fs::set_permissions(&restricted, fs::Permissions::from_mode(0o755));

    assert_eq!(
        output.status.code(),
        Some(13),
        "Must exit with code 13 (PERMISSION_DENIED) on permission error"
    );
    // The listing it could not read is an empty array, still valid JSON.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "[]\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "Permission denied: restricted - code: 13\n"
    );
}
