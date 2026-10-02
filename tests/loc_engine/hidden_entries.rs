// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The tree walk behind `--code` and the `--loc` percentage column used to
//! skip every dot-prefixed entry, with no way to ask for them. Passing
//! `--all` did nothing, and for percentages the mismatch was visible: a
//! hidden file counted in the numerator but not the denominator reported
//! more than 100% of the tree.

use lez::loc::count_roots;

use crate::common::{TempTestDir, lez_in, success_stdout};

/// One visible and two hidden Rust files, one of them inside a hidden
/// directory: 1, 2 and 1 lines of code.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("loc_hidden");
    dir.create_file("visible.rs", b"fn main() {}\n");
    dir.create_file(".secret.rs", b"fn a() {}\nfn b() {}\n");
    dir.create_file(".hidden/deep.rs", b"fn c() {}\n");
    dir
}

fn total(dir: &TempTestDir, hidden: bool) -> usize {
    count_roots(&[dir.path().to_path_buf()], hidden)
        .total()
        .code
}

#[test]
fn hidden_entries_are_counted_only_when_asked_for() {
    let dir = fixture();
    assert_eq!(total(&dir, false), 1);
    assert_eq!(total(&dir, true), 4);
}

/// The percentage denominator comes from the same walk, so with `-a` a
/// hidden file's share is of a total that includes it. Before, `.secret.rs`
/// was two lines out of a one-line total: 200%.
#[test]
fn a_hidden_files_share_is_of_a_total_that_includes_it() {
    let dir = fixture();
    let shares = |all: &[&str]| {
        success_stdout(
            lez_in(dir.path())
                .args([
                    "-l",
                    "--no-permissions",
                    "--no-filesize",
                    "--no-user",
                    "--no-time",
                    "--loc=percent",
                    "--no-language",
                ])
                .args(all),
        )
    };
    assert_eq!(shares(&[]), "100.0% visible.rs\n");
    assert_eq!(
        shares(&["-a"]),
        "    - .hidden\n50.0% .secret.rs\n25.0% visible.rs\n"
    );
}

/// A repository's own directory is not source and holds a great many files,
/// so it stays out of the walk whether or not hidden entries were asked for.
#[test]
fn a_git_directory_is_never_walked() {
    let dir = fixture();
    dir.create_file(".git/objects.rs", b"fn nope() {}\n");
    assert_eq!(total(&dir, true), 4);
}
