// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::process::Command;

use crate::common::{TempTestDir, bin_path};

#[test]
#[cfg(unix)]
fn test_loc_dereference_symlinks_with_and_without_extension() {
    let tmp = TempTestDir::new("loc_deref_ext");
    tmp.create_file("target.rs", b"fn foo() {}\nfn bar() {}\nfn baz() {}\n");

    // Symlink with extension
    tmp.create_symlink("target.rs", "link.rs");
    // Symlink without extension
    tmp.create_symlink("target.rs", "link_no_ext");

    // 1. Without dereference (-l --loc): symlinks must show '-' for LOC
    let out_no_deref = Command::new(bin_path())
        .args(["-l", "--loc", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_deref.status.success());
    let s_no_deref = String::from_utf8_lossy(&out_no_deref.stdout);
    for line in s_no_deref.lines() {
        if line.contains("link.rs") || line.contains("link_no_ext") {
            // Must contain placeholder '-' for lines of code
            let parts: Vec<&str> = line.split_whitespace().collect();
            // In non-deref long listing, symlink displays '-> target.rs'
            assert!(
                line.contains("-> target.rs"),
                "expected symlink target arrow in line: {line}"
            );
            // Verify LOC is '-'
            assert!(
                parts.contains(&"-"),
                "expected placeholder '-' for LOC in line: {line}"
            );
        }
    }

    // 2. With dereference (-l -X --loc): both symlinks must show Rust and 3 lines of code
    let out_deref = Command::new(bin_path())
        .args(["-l", "-X", "--loc", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_deref.status.success());
    let s_deref = String::from_utf8_lossy(&out_deref.stdout);

    let mut found_link_rs = false;
    let mut found_link_no_ext = false;
    let mut found_target_rs = false;

    for line in s_deref.lines() {
        if line.contains("link.rs") {
            found_link_rs = true;
            assert!(
                line.contains("Rust"),
                "link.rs should show Rust language in: {line}"
            );
            assert!(
                line.contains("3"),
                "link.rs should show 3 lines of code in: {line}"
            );
        }
        if line.contains("link_no_ext") {
            found_link_no_ext = true;
            assert!(
                line.contains("Rust"),
                "link_no_ext should show Rust language in: {line}"
            );
            assert!(
                line.contains("3"),
                "link_no_ext should show 3 lines of code in: {line}"
            );
        }
        if line.contains("target.rs") {
            found_target_rs = true;
            assert!(
                line.contains("Rust"),
                "target.rs should show Rust language in: {line}"
            );
            assert!(
                line.contains("3"),
                "target.rs should show 3 lines of code in: {line}"
            );
        }
    }

    assert!(found_link_rs, "link.rs was not found in output: {s_deref}");
    assert!(
        found_link_no_ext,
        "link_no_ext was not found in output: {s_deref}"
    );
    assert!(
        found_target_rs,
        "target.rs was not found in output: {s_deref}"
    );
}

#[test]
#[cfg(unix)]
fn test_loc_dereference_broken_symlinks_and_directory_symlinks() {
    let tmp = TempTestDir::new("loc_deref_broken_dir");
    tmp.create_dir("actual_dir");
    tmp.create_file("actual_dir/inner.rs", b"// inside dir\nfn inner() {}\n");

    // Broken symlinks (with and without extension)
    tmp.create_symlink("nonexistent.rs", "broken.rs");
    tmp.create_symlink("nonexistent_file", "broken_no_ext");

    // Directory symlink (with and without extension)
    tmp.create_symlink("actual_dir", "link_dir");
    tmp.create_symlink("actual_dir", "link_dir_ext.rs");

    // Cyclic symlink loop
    tmp.create_symlink("loop_b", "loop_a");
    tmp.create_symlink("loop_a", "loop_b");

    // Run lez -ld -X --loc to list entries as items (not descending into dirs)
    let out = Command::new(bin_path())
        .args(["-l", "-d", "-X", "--loc", "--color=never"])
        .arg(tmp.path().join("broken.rs"))
        .arg(tmp.path().join("broken_no_ext"))
        .arg(tmp.path().join("link_dir"))
        .arg(tmp.path().join("link_dir_ext.rs"))
        .arg(tmp.path().join("loop_a"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    for line in stdout.lines() {
        if line.contains("broken.rs")
            || line.contains("broken_no_ext")
            || line.contains("link_dir")
            || line.contains("link_dir_ext.rs")
            || line.contains("loop_a")
        {
            // For broken symlinks and directory symlinks, neither Language nor LOC should be present.
            // Specifically, 'Rust' must NOT appear as the language for broken.rs or link_dir_ext.rs.
            assert!(
                !line.contains("Rust"),
                "broken/dir/cyclic symlink must NOT resolve to Rust language in line: {line}"
            );
            let parts: Vec<&str> = line.split_whitespace().collect();
            // In the long format: permissions, size, language, loc...
            // Both language and loc columns should be "-"
            let dash_count = parts.iter().filter(|&&p| p == "-").count();
            assert!(
                dash_count >= 2,
                "expected at least '-' for language and LOC in line: {line}"
            );
        }
    }
}

#[test]
#[cfg(unix)]
fn test_loc_dereference_chained_and_relative_symlinks() {
    let tmp = TempTestDir::new("loc_deref_chained");
    let sub = tmp.create_dir("sub");
    tmp.create_file(
        "root.py",
        b"# Python script\nprint('hello')\nprint('world')\n",
    );
    tmp.create_file(
        "noext_script",
        b"# Python script\nprint('noext1')\nprint('noext2')\n",
    );

    // Relative symlinks inside sub pointing up to root.py
    tmp.create_symlink("../root.py", "sub/link_up.py");
    tmp.create_symlink("../root.py", "sub/link_up_no_ext");

    // Chained symlink where intermediate hop has a misleading extension (.sh)
    // chain_a -> chain_b.sh -> root.py
    tmp.create_symlink("root.py", "chain_b.sh");
    tmp.create_symlink("chain_b.sh", "chain_a");

    // Symlink with extension pointing to file without extension
    tmp.create_symlink("noext_script", "link_with_ext.py");

    let out_sub = Command::new(bin_path())
        .args(["-l", "-X", "--loc", "--color=never"])
        .arg(&sub)
        .output()
        .unwrap();
    assert!(out_sub.status.success());
    let stdout_sub = String::from_utf8_lossy(&out_sub.stdout);
    assert!(stdout_sub.contains("link_up.py"));
    assert!(stdout_sub.contains("link_up_no_ext"));
    for line in stdout_sub.lines() {
        if line.contains("link_up.py") || line.contains("link_up_no_ext") {
            assert!(
                line.contains("Python"),
                "expected Python language in: {line}"
            );
            assert!(line.contains("2"), "expected 2 lines of code in: {line}");
        }
    }

    let out_chain = Command::new(bin_path())
        .args(["-l", "-X", "--loc", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_chain.status.success());
    let stdout_chain = String::from_utf8_lossy(&out_chain.stdout);
    for line in stdout_chain.lines() {
        if line.contains("chain_a") || line.contains("chain_b.sh") {
            assert!(
                line.contains("Python"),
                "expected Python language in chained link: {line}"
            );
            assert!(
                !line.contains("Shell"),
                "chained link must NOT be resolved as Shell: {line}"
            );
            assert!(
                line.contains("2"),
                "expected 2 lines of code in chained link: {line}"
            );
        }
        if line.contains("link_with_ext.py") {
            assert!(
                line.contains("Python"),
                "expected Python language for symlink with ext: {line}"
            );
            assert!(
                line.contains("2"),
                "expected 2 lines of code for symlink with ext: {line}"
            );
        }
    }
}

#[test]
#[cfg(unix)]
fn test_loc_dereference_mismatched_extension() {
    let tmp = TempTestDir::new("loc_deref_mismatched");
    // Python script with a python comment (# is comment in Python, but code in Rust)
    tmp.create_file(
        "real.py",
        b"# python comment\nprint('code1')\nprint('code2')\n",
    );
    // Symlink named with .rs extension pointing to Python script
    tmp.create_symlink("real.py", "link_as_rust.rs");

    // 1. With -l -X --loc: dereferenced target real.py is Python, so must report Python and 2 LOC
    let out_deref = Command::new(bin_path())
        .args(["-l", "-X", "--loc", "--color=never"])
        .arg(tmp.path().join("link_as_rust.rs"))
        .output()
        .unwrap();
    assert!(out_deref.status.success());
    let stdout_deref = String::from_utf8_lossy(&out_deref.stdout);
    assert!(
        stdout_deref.contains("Python"),
        "dereferenced link must show target language Python: {stdout_deref}"
    );
    assert!(
        stdout_deref.contains("2"),
        "dereferenced link must count 2 lines of code (Python syntax): {stdout_deref}"
    );
    assert!(
        !stdout_deref.contains("Rust"),
        "dereferenced link must NOT show Rust language: {stdout_deref}"
    );

    // 2. Without -X: non-dereferenced symlink shows link's own name extension (Rust) and '-' for LOC
    let out_no_deref = Command::new(bin_path())
        .args(["-l", "--loc", "--color=never"])
        .arg(tmp.path().join("link_as_rust.rs"))
        .output()
        .unwrap();
    assert!(out_no_deref.status.success());
    let stdout_no_deref = String::from_utf8_lossy(&out_no_deref.stdout);
    assert!(
        stdout_no_deref.contains("Rust"),
        "non-dereferenced symlink shows its own filename language: {stdout_no_deref}"
    );
    let parts: Vec<&str> = stdout_no_deref.split_whitespace().collect();
    assert!(
        parts.contains(&"-"),
        "non-dereferenced symlink must have '-' for LOC: {stdout_no_deref}"
    );
}

#[test]
#[cfg(unix)]
fn test_loc_dereference_json_output() {
    let tmp = TempTestDir::new("loc_deref_json");
    tmp.create_file("main.rs", b"fn main() {}\n");
    tmp.create_file("py_script.py", b"# comment\nprint('hi')\n");
    tmp.create_symlink("main.rs", "sym_with_ext.rs");
    tmp.create_symlink("main.rs", "sym_no_ext");
    tmp.create_symlink("missing.rs", "broken_link.rs");
    tmp.create_symlink("py_script.py", "mismatched.rs");

    // 1. JSON with -X --loc
    let out_deref = Command::new(bin_path())
        .args(["--json", "-l", "-X", "--loc"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_deref.status.success());
    let json_deref: serde_json::Value = serde_json::from_slice(&out_deref.stdout).unwrap();
    let map_deref = json_deref.as_object().expect("expected json map of files");

    // sym_with_ext.rs must have Language: Rust and Code: 1
    let sym_ext = map_deref
        .get("sym_with_ext.rs")
        .expect("expected sym_with_ext.rs");
    assert_eq!(
        sym_ext.get("Language").and_then(|v| v.as_str()),
        Some("Rust")
    );
    assert_eq!(sym_ext.get("Code").and_then(|v| v.as_str()), Some("1"));

    // sym_no_ext must have Language: Rust and Code: 1
    let sym_noext = map_deref.get("sym_no_ext").expect("expected sym_no_ext");
    assert_eq!(
        sym_noext.get("Language").and_then(|v| v.as_str()),
        Some("Rust")
    );
    assert_eq!(sym_noext.get("Code").and_then(|v| v.as_str()), Some("1"));

    // mismatched.rs pointing to py_script.py must have Language: Python and Code: 1
    let mism = map_deref
        .get("mismatched.rs")
        .expect("expected mismatched.rs");
    assert_eq!(
        mism.get("Language").and_then(|v| v.as_str()),
        Some("Python")
    );
    assert_eq!(mism.get("Code").and_then(|v| v.as_str()), Some("1"));

    // broken_link.rs must NOT have Code or Language when dereferenced
    let broken = map_deref
        .get("broken_link.rs")
        .expect("expected broken_link.rs");
    assert!(broken.get("Code").is_none());
    assert!(broken.get("Language").is_none());

    // 2. JSON without -X: symlinks must NOT have Code
    let out_no_deref = Command::new(bin_path())
        .args(["--json", "-l", "--loc"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_deref.status.success());
    let json_no_deref: serde_json::Value = serde_json::from_slice(&out_no_deref.stdout).unwrap();
    let map_no_deref = json_no_deref
        .as_object()
        .expect("expected json map of files");

    let no_deref_ext = map_no_deref
        .get("sym_with_ext.rs")
        .expect("expected sym_with_ext.rs");
    assert!(
        no_deref_ext.get("Code").is_none(),
        "symlink without -X should not have Code"
    );

    let no_deref_noext = map_no_deref.get("sym_no_ext").expect("expected sym_no_ext");
    assert!(
        no_deref_noext.get("Code").is_none(),
        "symlink without -X should not have Code"
    );
    assert!(
        no_deref_noext.get("Language").is_none(),
        "symlink without -X and no ext should not have Language"
    );
}

#[test]
#[cfg(unix)]
fn test_loc_dereference_positional_files() {
    let tmp = TempTestDir::new("loc_deref_pos");
    let src = tmp.create_file("code.go", b"package main\n\nfunc main() {}\n");
    let link_ext = tmp.create_symlink("code.go", "pos_link.go");
    let link_noext = tmp.create_symlink("code.go", "pos_link_noext");

    let out = Command::new(bin_path())
        .args(["-l", "-X", "--loc", "--color=never"])
        .arg(&link_ext)
        .arg(&link_noext)
        .arg(&src)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    for line in stdout.lines() {
        if line.contains("pos_link.go")
            || line.contains("pos_link_noext")
            || line.contains("code.go")
        {
            assert!(line.contains("Go"), "expected Go language in: {line}");
            assert!(line.contains("2"), "expected 2 lines of code in: {line}");
        }
    }
}

#[test]
#[cfg(unix)]
fn test_code_mode_symlink_dereference_flags() {
    let tmp = TempTestDir::new("code_deref_flags");
    tmp.create_file("main.rs", b"fn main() {\n    println!(\"hello\");\n}\n");
    tmp.create_symlink("main.rs", "symlink.rs");

    // 1. Without -X: symlinks must NOT be dereferenced/counted in --code (Files: 1, not 2)
    let out_no_deref = Command::new(bin_path())
        .args(["--code", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_deref.status.success());
    let s_no_deref = String::from_utf8_lossy(&out_no_deref.stdout);
    for line in s_no_deref.lines() {
        if line.contains("Rust") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                parts[1], "1",
                "expected 1 file counted without -X, got: {line}"
            );
        }
    }

    // 2. With -X: symlink IS dereferenced and counted (Files: 2)
    let out_deref = Command::new(bin_path())
        .args(["--code", "-X", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_deref.status.success());
    let s_deref = String::from_utf8_lossy(&out_deref.stdout);
    for line in s_deref.lines() {
        if line.contains("Rust") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                parts[1], "2",
                "expected 2 files counted with -X, got: {line}"
            );
        }
    }

    // 3. With -X --no-symlinks: symlinks ignored even if -X is passed (Files: 1)
    let out_no_sym = Command::new(bin_path())
        .args(["--code", "-X", "--no-symlinks", "--color=never"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(out_no_sym.status.success());
    let s_no_sym = String::from_utf8_lossy(&out_no_sym.stdout);
    for line in s_no_sym.lines() {
        if line.contains("Rust") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(
                parts[1], "1",
                "expected 1 file counted with --no-symlinks, got: {line}"
            );
        }
    }
}
