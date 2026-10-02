// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Lines of code for symlinks. Without `-X` a link is not counted; with it,
//! a link to a file is counted as that file, in the language of the file it
//! leads to, its own name serving only when the target's says nothing. The
//! `Code %` column is a share of the code under the listed directory, each
//! file counted once and links followed only with `--follow-symlinks`, so
//! JSON, the long view and `--code` must agree on it.

#![cfg(unix)]

use crate::common::{TempTestDir, lez_in, success_stdout};

/// - `target.rs`: three lines of Rust; `link.rs` and `link_no_ext` lead to it.
/// - `root.py`: a comment and two lines of Python; `chain_a -> chain_b.sh ->
///   root.py`, whose middle hop is named like a shell script, and
///   `link_as_rust.rs`, named like Rust, lead to it, as do the two links in
///   `sub/`.
/// - `noext_script`: the same Python with no extension; `link_with_ext.py`
///   names its language.
/// - `actual_dir/inner.rs`: one line, behind `link_dir` and
///   `link_dir_ext.rs`.
/// - Two dangling links and a loop, none of which leads to a file.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    dir.create_file("target.rs", b"fn foo() {}\nfn bar() {}\nfn baz() {}\n");
    dir.create_symlink("target.rs", "link.rs");
    dir.create_symlink("target.rs", "link_no_ext");
    dir.create_file(
        "root.py",
        b"# Python script\nprint('hello')\nprint('world')\n",
    );
    dir.create_symlink("root.py", "chain_b.sh");
    dir.create_symlink("chain_b.sh", "chain_a");
    dir.create_symlink("root.py", "link_as_rust.rs");
    dir.create_symlink("../root.py", "sub/link_up.py");
    dir.create_symlink("../root.py", "sub/link_up_no_ext");
    dir.create_file("noext_script", b"# Python script\nprint('a')\nprint('b')\n");
    dir.create_symlink("noext_script", "link_with_ext.py");
    dir.create_file("actual_dir/inner.rs", b"// inside\nfn inner() {}\n");
    dir.create_symlink("actual_dir", "link_dir");
    dir.create_symlink("actual_dir", "link_dir_ext.rs");
    dir.create_symlink("nonexistent.rs", "broken.rs");
    dir.create_symlink("nonexistent_file", "broken_no_ext");
    dir.create_symlink("loop_b", "loop_a");
    dir.create_symlink("loop_a", "loop_b");
    dir
}

/// The language, code and share columns, and the name.
fn loc_rows(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(
        lez_in(dir.path())
            .args([
                "-l",
                "--loc",
                "--no-permissions",
                "--no-filesize",
                "--no-user",
                "--no-time",
            ])
            .args(args),
    )
}

/// Without `-X` a link shows the language its own name suggests and no
/// count. The share is of `target.rs`, `root.py` and `actual_dir/inner.rs`:
/// six lines.
#[test]
fn without_dereferencing_links_are_not_counted() {
    let dir = fixture("plain");
    assert_eq!(
        loc_rows(&dir, &[]),
        "-      -     - actual_dir\n\
         Rust   -     - broken.rs -> nonexistent.rs\n\
         -      -     - broken_no_ext -> nonexistent_file\n\
         -      -     - chain_a -> chain_b.sh\n\
         Shell  -     - chain_b.sh -> root.py\n\
         Rust   -     - link.rs -> target.rs\n\
         Rust   -     - link_as_rust.rs -> root.py\n\
         -      -     - link_dir -> actual_dir\n\
         Rust   -     - link_dir_ext.rs -> actual_dir\n\
         -      -     - link_no_ext -> target.rs\n\
         Python -     - link_with_ext.py -> noext_script\n\
         -      -     - loop_a -> loop_b\n\
         -      -     - loop_b -> loop_a\n\
         -      -     - noext_script\n\
         Python 2 33.3% root.py\n\
         -      -     - sub\n\
         Rust   3 50.0% target.rs\n"
    );
}

/// With `-X` each link to a file is counted as that file, through any
/// number of hops, in the target's language: `link_as_rust.rs` and the
/// `.sh` hop are Python. `link_with_ext.py` is Python by its own name, as
/// its target has none. Links to directories, dangling links and loops
/// count nothing. The share stays of the six lines.
#[test]
fn dereferenced_links_are_counted_as_their_targets() {
    let dir = fixture("deref");
    assert_eq!(
        loc_rows(&dir, &["-X"]),
        "-      -     - actual_dir\n\
         -      -     - broken.rs\n\
         -      -     - broken_no_ext\n\
         Python 2 33.3% chain_a\n\
         Python 2 33.3% chain_b.sh\n\
         Rust   3 50.0% link.rs\n\
         Python 2 33.3% link_as_rust.rs\n\
         -      -     - link_dir\n\
         -      -     - link_dir_ext.rs\n\
         Rust   3 50.0% link_no_ext\n\
         Python 2 33.3% link_with_ext.py\n\
         -      -     - loop_a\n\
         -      -     - loop_b\n\
         -      -     - noext_script\n\
         Python 2 33.3% root.py\n\
         -      -     - sub\n\
         Rust   3 50.0% target.rs\n"
    );
    // Relative links resolve from their own directory. `sub` itself holds
    // no code, so there is nothing to take a share of.
    assert_eq!(
        loc_rows(&dir, &["-X", "sub"]),
        "Python 2 - link_up.py\nPython 2 - link_up_no_ext\n"
    );
    // Named on the command line, files share their own total.
    assert_eq!(
        loc_rows(&dir, &["-X", "link.rs", "link_no_ext", "target.rs"]),
        "Rust 3 100.0% link.rs\nRust 3 100.0% link_no_ext\nRust 3 100.0% target.rs\n"
    );
}

/// JSON gives the same counts and the same shares as the long view. It
/// used to take each entry as a root of its own, following links under
/// `-X`, so a link's target was counted again and the shares here read
/// 33.3% without `-X` and 20% with it.
#[test]
fn json_shares_match_the_long_view() {
    let dir = TempTestDir::new("json");
    dir.create_file("main.rs", b"fn main() {}\n");
    dir.create_file("py_script.py", b"# comment\nprint('hi')\n");
    dir.create_symlink("main.rs", "sym_with_ext.rs");
    dir.create_symlink("main.rs", "sym_no_ext");
    dir.create_symlink("missing.rs", "broken_link.rs");
    dir.create_symlink("py_script.py", "mismatched.rs");

    let json = |extra: &[&str]| -> serde_json::Value {
        serde_json::from_str(&loc_rows(&dir, &[&["--json"][..], extra].concat()))
            .expect("valid JSON")
    };
    let counted =
        |language: &str| serde_json::json!({"Language": language, "Code": "1", "Code %": "50.0%"});

    assert_eq!(
        json(&[]),
        serde_json::json!({
            "broken_link.rs": {"Language": "Rust", "Target": "missing.rs"},
            "main.rs": counted("Rust"),
            "mismatched.rs": {"Language": "Rust", "Target": "py_script.py"},
            "py_script.py": counted("Python"),
            "sym_no_ext": {"Target": "main.rs"},
            "sym_with_ext.rs": {"Language": "Rust", "Target": "main.rs"},
        })
    );
    let linked = |language: &str, target: &str| {
        let mut entry = counted(language);
        entry["Target"] = target.into();
        entry
    };
    assert_eq!(
        json(&["-X"]),
        serde_json::json!({
            "broken_link.rs": {"Target": "missing.rs"},
            "main.rs": counted("Rust"),
            "mismatched.rs": linked("Python", "py_script.py"),
            "py_script.py": counted("Python"),
            "sym_no_ext": linked("Rust", "main.rs"),
            "sym_with_ext.rs": linked("Rust", "main.rs"),
        })
    );
    assert_eq!(
        loc_rows(&dir, &["-X"]),
        "-      -     - broken_link.rs\n\
         Rust   1 50.0% main.rs\n\
         Python 1 50.0% mismatched.rs\n\
         Python 1 50.0% py_script.py\n\
         Rust   1 50.0% sym_no_ext\n\
         Rust   1 50.0% sym_with_ext.rs\n"
    );
}

/// The same holds through `-R`, where each directory is its own listing,
/// and `-T`, where the whole tree is one.
#[test]
fn json_shares_match_the_long_view_when_recursing() {
    let dir = TempTestDir::new("json_recursive");
    dir.create_file("top/a.rs", b"fn a() {}\n");
    dir.create_file("top/sub/b.rs", b"fn b() {}\nfn c() {}\nfn d() {}\n");

    let file = |code: &str, share: &str| serde_json::json!({"Language": "Rust", "Code": code, "Code %": share});
    let tree = |a: &str, b: &str| {
        serde_json::json!({"top": {
            "files": {"a.rs": file("1", a)},
            "directories": {"sub": {"files": {"b.rs": file("3", b)}, "directories": {}}}
        }})
    };
    let json = |args: &[&str]| -> serde_json::Value {
        serde_json::from_str(&loc_rows(&dir, &[&["--json"][..], args].concat()))
            .expect("valid JSON")
    };

    assert_eq!(
        loc_rows(&dir, &["-R", "top"]),
        crate::common::native(
            "Rust 1 25.0% a.rs\n-    -     - sub\n\ntop/sub:\nRust 3 100.0% b.rs\n"
        )
    );
    assert_eq!(json(&["-R", "top"]), tree("25.0%", "100.0%"));

    assert_eq!(
        loc_rows(&dir, &["-T", "top"]),
        "-    -     - top\n\
         Rust 1 25.0% ├── a.rs\n\
         -    -     - └── sub\n\
         Rust 3 75.0%     └── b.rs\n"
    );
    assert_eq!(json(&["-T", "top"]), tree("25.0%", "75.0%"));
}

fn code(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).arg("--code").args(args))
}

/// `--code` counts a link only with `-X`, and then as a file of its own,
/// unless `--no-symlinks` drops it. The same path given twice is one file.
#[test]
fn code_mode_counts_links_only_when_dereferencing() {
    let dir = TempTestDir::new("code_mode");
    dir.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    dir.create_symlink("main.rs", "symlink.rs");

    let table = |files: u32, lines: u32| {
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n \
             Rust          {files}      {lines}     {lines}         0       0  100.0%  ████████████████\n\
             ─────────────────────────────────────────────────────────────────────────\n \
             Total         {files}      {lines}     {lines}         0       0  100.0%\n"
        )
    };
    assert_eq!(code(&dir, &[]), table(1, 3));
    assert_eq!(code(&dir, &["-X"]), table(2, 6));
    assert_eq!(code(&dir, &["-X", "--no-symlinks"]), table(1, 3));
    let absolute = dir.path().join("main.rs");
    let absolute = absolute.to_str().expect("UTF-8 path");
    assert_eq!(
        code(&dir, &["-X", "main.rs", "./main.rs", absolute]),
        table(1, 3)
    );
}

/// A followed link is in its target's language in `--code` too: this link
/// named `.rs` holds Python, whose `#` line is a comment. Counted as Rust,
/// that line was code.
#[test]
fn code_mode_names_a_followed_link_by_its_target() {
    let dir = TempTestDir::new("code_language");
    dir.create_file("py_script.py", b"# comment\nprint('hi')\n");
    dir.create_symlink("py_script.py", "mismatched.rs");

    assert_eq!(
        code(&dir, &["-X"]),
        " Language  Files  Lines  Code  Comments  Blanks  Code %\n \
         Python        2      4     2         2       0  100.0%  ████████████████\n\
         ─────────────────────────────────────────────────────────────────────────\n \
         Total         2      4     2         2       0  100.0%\n"
    );
}
