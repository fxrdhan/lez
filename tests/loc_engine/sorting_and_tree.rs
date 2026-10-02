// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! How `--code` orders its languages, and which files it counts: the same
//! filters as a listing (`-I`, `-f`, `-D`, `--since`, `.gitignore` unless
//! `--no-git`) decide what is walked.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// 6 lines of Rust, 2 of Python and 1 of shell.
fn three_languages() -> TempTestDir {
    let dir = TempTestDir::new("code_order");
    dir.create_file(
        "main.rs",
        b"fn main() {\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    println!(\"{c}\");\n}\n",
    );
    dir.create_file("script.py", b"print('hello')\nprint('world')\n");
    dir.create_file("run.sh", b"echo 'run'\n");
    dir
}

fn code(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).arg("--code=lines").args(args))
}

/// The languages in the order the table lists them.
fn order(dir: &TempTestDir, args: &[&str]) -> Vec<String> {
    code(dir, args)
        .lines()
        .skip(1)
        .take_while(|row| !row.starts_with('─'))
        .map(|row| row.split_whitespace().next().unwrap_or_default().to_owned())
        .collect()
}

const NOTHING: &str = "No recognised source code found.\n";

fn table(rows: &[&str], total: &str) -> String {
    let mut table = String::from(" Language  Files  Lines  Code  Comments  Blanks\n");
    for row in rows {
        table.push_str(row);
        table.push('\n');
    }
    table.push_str(&"─".repeat(47));
    table.push('\n');
    table.push_str(total);
    table.push('\n');
    table
}

/// Most code first; `-r` the least first. The name sorts and `-s none`
/// order by name, and every other field, whatever it would sort a listing
/// by, orders by code.
#[test]
fn languages_are_ordered_by_code_unless_sorted_by_name() {
    let dir = three_languages();
    assert_eq!(
        code(&dir, &[]),
        table(
            &[
                " Rust          1      6     6         0       0",
                " Python        1      2     2         0       0",
                " Shell         1      1     1         0       0",
            ],
            " Total         3      9     9         0       0",
        )
    );
    for (args, expected) in [
        (&[][..], ["Rust", "Python", "Shell"]),
        (&["-r"], ["Shell", "Python", "Rust"]),
        (&["-s", "name"], ["Python", "Rust", "Shell"]),
        (&["-s", "Name"], ["Python", "Rust", "Shell"]),
        (&["-s", "none"], ["Python", "Rust", "Shell"]),
        (&["-s", "name", "-r"], ["Shell", "Rust", "Python"]),
        (&["-s", "size"], ["Rust", "Python", "Shell"]),
        (&["-s", "modified"], ["Rust", "Python", "Shell"]),
        (&["-s", "percent", "-r"], ["Shell", "Python", "Rust"]),
        (&["-s", "percentage", "-r"], ["Shell", "Python", "Rust"]),
        (&["-s", "loc", "-r"], ["Shell", "Python", "Rust"]),
        (&["-s", "code", "-r"], ["Shell", "Python", "Rust"]),
    ] {
        assert_eq!(order(&dir, args), expected, "{args:?}");
    }
}

#[test]
fn the_listing_filters_decide_what_is_counted() {
    let dir = TempTestDir::new("code_filters");
    dir.create_file("main.rs", b"fn main() {}\n");
    dir.create_file("script.py", b"print('hello')\n");
    dir.create_file("nested/child.py", b"print('child')\n");
    let rust_only = table(
        &[" Rust          1      1     1         0       0"],
        " Total         1      1     1         0       0",
    );

    assert_eq!(
        code(&dir, &[]),
        table(
            &[
                " Python        2      2     2         0       0",
                " Rust          1      1     1         0       0",
            ],
            " Total         3      3     3         0       0",
        )
    );
    assert_eq!(code(&dir, &["-I", "*.py"]), rust_only);
    assert_eq!(code(&dir, &["-I", "*.py", "-I", "*.rs"]), NOTHING);
    assert_eq!(code(&dir, &["-I", "*.rs", "main.rs"]), NOTHING);
    // `-f` keeps a directory named on the command line from being walked,
    // and `-D` leaves no file to count.
    assert_eq!(code(&dir, &["-f", "main.rs", "nested"]), rust_only);
    assert_eq!(code(&dir, &["-D"]), NOTHING);
}

/// Older files drop out with `--since`.
#[test]
fn since_counts_only_what_changed_recently() {
    let dir = TempTestDir::new("code_since");
    dir.create_file("fresh.rs", b"fn fresh() {}\n");
    let old = dir.create_file("old.py", b"print('old')\n");
    std::fs::File::options()
        .write(true)
        .open(&old)
        .and_then(|file| {
            file.set_times(std::fs::FileTimes::new().set_modified(
                std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86_400),
            ))
        })
        .expect("backdate old.py");
    assert_eq!(
        code(&dir, &["--since", "1h"]),
        table(
            &[" Rust          1      1     1         0       0"],
            " Total         1      1     1         0       0",
        )
    );
}

/// In a repository, what `.gitignore` ignores is not counted, unless
/// `--no-git` says not to look.
#[test]
fn ignored_files_count_only_with_no_git() {
    crate::common::require_git();
    let repo = crate::common::TempGitRepo::new("code_git");
    repo.create_file(".gitignore", b"ignored.py\n");
    repo.create_file("main.rs", b"fn main() {}\n");
    repo.create_file("script.py", b"print('kept')\n");
    repo.create_file("ignored.py", b"print('ignored')\n");
    let code = |args: &[&str]| success_stdout(lez_in(repo.path()).arg("--code=lines").args(args));
    assert_eq!(
        code(&[]),
        table(
            &[
                " Python        1      1     1         0       0",
                " Rust          1      1     1         0       0",
            ],
            " Total         2      2     2         0       0",
        )
    );
    assert_eq!(
        code(&["--no-git"]),
        table(
            &[
                " Python        2      2     2         0       0",
                " Rust          1      1     1         0       0",
            ],
            " Total         3      3     3         0       0",
        )
    );
}
