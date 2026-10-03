// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Regression tests for the defects in `docs/audit/bug_bounty_report.md`.
//! Each one pins the corrected output, not merely the absence of a panic.
//! Bug 7 (`--ignore-submodule-contents`) is covered in `git/submodules.rs`.

use std::fs;
use std::io::Write;
use std::process::{Output, Stdio};

use crate::common::{TempGitRepo, TempTestDir, lez_in};

fn run(dir: &TempTestDir, args: &[&str]) -> Output {
    lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Bug 1: the column width sum overflowed and panicked. The spacing is now
/// capped at a thousand columns.
#[test]
fn huge_spacing_is_capped_instead_of_overflowing() {
    let dir = TempTestDir::new("spacing_overflow");
    dir.create_file("f.txt", b"x");

    let output = run(
        &dir,
        &[
            "-l",
            "--spacing",
            "5000000000000000000",
            "--no-permissions",
            "--no-user",
            "--no-time",
            "f.txt",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!("1{}f.txt\n", " ".repeat(1000))
    );
}

/// Bug 2: following a directory symlink back to an ancestor recursed until
/// `ELOOP`. The link is listed but not entered.
#[test]
#[cfg(unix)]
fn a_symlink_cycle_is_listed_once_and_not_entered() {
    let dir = TempTestDir::new("symlink_cycle");
    dir.create_dir("cyc/sub");
    dir.create_symlink("..", "cyc/sub/loop_to_root");

    let output = run(&dir, &["-R", "--follow-symlinks", "cyc"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "sub\n\ncyc/sub:\nloop_to_root\n");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
}

/// Bug 3: a non-UTF-8 path on stdin aborted the whole read. It is now looked
/// up like any other path, so only that entry fails.
#[test]
#[cfg(unix)]
fn a_non_utf8_stdin_path_fails_alone() {
    let dir = TempTestDir::new("stdin_raw");
    dir.create_file("f.txt", b"x");

    let mut child = lez_in(dir.path())
        .arg("--stdin")
        .env("LEZ_STDIN_SEPARATOR", "\\0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn lez");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"f.txt\0\xff\xfe\0")
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for lez");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "f.txt\n");
    assert_eq!(
        text(&output.stderr),
        "\"\\xFF\\xFE\": No such file or directory (os error 2)\n"
    );
}

/// Bug 4: an explicit `--config` that was missing or broken was ignored
/// without a word.
#[test]
fn an_explicit_config_that_cannot_be_used_is_reported() {
    let dir = TempTestDir::new("explicit_config");
    dir.create_file("f.txt", b"x");
    dir.create_file("bad.toml", b"[[[ syntax");
    dir.create_file("config.yaml", b"display:\n  header: true\n");

    let missing = dir.path().join("missing.toml");
    // The OS words "not found" differently per platform, so ask it.
    let not_found = fs::read(&missing).expect_err("the config must be missing");
    let output = run(&dir, &["--config", missing.to_str().unwrap(), "f.txt"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stdout), "f.txt\n");
    assert_eq!(
        text(&output.stderr),
        format!("lez: Failed to read config file {missing:?}: {not_found}\n")
    );

    let output = run(&dir, &["--config", "bad.toml", "f.txt"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output.stdout), "f.txt\n");
    assert_eq!(
        text(&output.stderr),
        "lez: Failed to parse config file \"bad.toml\": TOML parse error at line 1, column 3\n  \
         |\n\
         1 | [[[ syntax\n  \
         |   ^\n\
         unquoted keys cannot be empty, expected letters, numbers, `-`, `_`\n"
    );

    // A YAML config is a supported format, not a parse failure.
    let output = run(
        &dir,
        &[
            "--config",
            "config.yaml",
            "-l",
            "--no-permissions",
            "--no-user",
            "--no-time",
            "f.txt",
        ],
    );
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "Size Name\n   1 f.txt\n");
}

/// Bug 5: `mode = "tree"` from the config skipped the `-a -a` check that the
/// `--tree` flag gets.
#[test]
fn tree_mode_from_config_rejects_all_all_like_the_flag() {
    let dir = TempTestDir::new("tree_cfg");
    dir.create_file("f.txt", b"x");
    dir.create_file("tree.toml", b"[display]\nmode = \"tree\"\n");

    let output = run(&dir, &["--config", "tree.toml", "-a", "-a", "f.txt"]);
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        text(&output.stderr),
        "lez: Option --tree is useless given --all --all\n"
    );

    // A view chosen on the command line replaces the configured tree.
    let output = run(&dir, &["--config", "tree.toml", "-1", "-a", "-a", "f.txt"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "f.txt\n");
}

/// Bug 6: strict mode let newer long-view columns through without `--long`.
#[test]
fn strict_mode_rejects_long_view_columns_without_long() {
    let dir = TempTestDir::new("strict_long_only");
    dir.create_file("f.txt", b"x");

    for (flag, option) in [
        ("--git-repos", "git-repos"),
        ("--octal-permissions", "octal-permissions"),
        ("--total-size", "total-size"),
        ("--flags", "flags"),
        ("--context", "context"),
        ("--smart-group", "smart-group"),
        ("--extended", "extended"),
        ("--no-permissions", "no-permissions"),
    ] {
        let output = lez_in(dir.path())
            .env("LEZ_STRICT", "1")
            .args([flag, "f.txt"])
            .output()
            .expect("failed to run lez");
        assert_eq!(output.status.code(), Some(3), "{flag}");
        assert_eq!(
            text(&output.stderr),
            format!("lez: Option {option} is useless without option long\n"),
            "{flag}"
        );
    }

    let output = lez_in(dir.path())
        .env("LEZ_STRICT", "1")
        .args(["-l", "--octal-permissions", "f.txt"])
        .output()
        .expect("failed to run lez");
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
}

/// Strict mode refused options that change what a view prints without
/// `--long`: `--print-total` in every view, and in a tree the link targets,
/// attributes and archive contents it draws (and mount details, which need a
/// mount point and are left to the unit tests). It now takes them, and
/// prints what the same command prints without it.
#[test]
#[cfg(unix)]
fn strict_mode_takes_options_that_change_what_is_printed() {
    let dir = TempTestDir::new("strict_takes");
    dir.create_file("f.txt", b"x");
    dir.create_symlink("f.txt", "link");

    let mut cases = vec![
        vec!["-1", "--print-total"],
        vec!["--grid", "--print-total"],
        vec!["-T", "--print-total"],
        vec!["-T", "--no-symlink-targets"],
    ];
    #[cfg(feature = "inspect-archives")]
    {
        let mut builder = tar::Builder::new(
            fs::File::create(dir.path().join("a.tar")).expect("create the archive"),
        );
        let mut header = tar::Header::new_gnu();
        header.set_size(1);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "inner.txt", &b"x"[..])
            .expect("append an entry");
        builder.into_inner().expect("finish the archive");
        cases.push(vec!["-T", "--inspect-archives"]);
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if crate::common::set_xattr(&dir.path().join("f.txt"), b"v") {
        cases.push(vec!["-T", "--extended"]);
    }

    for args in cases {
        let lenient = run(&dir, &args);
        let output = lez_in(dir.path())
            .env("LEZ_STRICT", "1")
            .args(&args)
            .output()
            .expect("failed to run lez");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            text(&output.stderr)
        );
        assert_eq!(text(&output.stderr), "", "{args:?}");
        assert_eq!(text(&output.stdout), text(&lenient.stdout), "{args:?}");
        // The option did something, so strict mode had something to take.
        let without = run(&dir, &args[..1]);
        assert_ne!(text(&output.stdout), text(&without.stdout), "{args:?}");
    }
}

/// Bug 8: a work tree described by `GIT_DIR`/`GIT_WORK_TREE` (a bare-repo
/// dotfiles setup) lost its git column.
#[test]
#[cfg(feature = "git")]
fn a_bare_repository_work_tree_from_the_environment_shows_status() {
    crate::common::require_git();
    let dir = TempTestDir::new("bare_git");
    let bare = dir.create_dir("bare.git");
    let work = dir.create_dir("work");
    let git = |args: &[&str]| {
        let status = crate::common::git_command(&work)
            .env("GIT_DIR", &bare)
            .env("GIT_WORK_TREE", &work)
            .args(args)
            .status()
            .expect("failed to run git");
        assert!(status.success(), "git {args:?}");
    };

    crate::common::git_command(&bare)
        .args(["init", "--bare", "-q"])
        .status()
        .expect("git init --bare")
        .success()
        .then_some(())
        .expect("git init --bare failed");
    fs::write(work.join("test.txt"), b"hello\n").unwrap();
    git(&["add", "test.txt"]);
    git(&["commit", "-q", "-m", "init"]);
    fs::write(work.join("test.txt"), b"hello\nmodified\n").unwrap();

    let output = crate::common::lez_in(&work)
        .args([
            "-l",
            "--git",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ])
        .env("GIT_DIR", &bare)
        .env("GIT_WORK_TREE", &work)
        .output()
        .expect("failed to run lez");
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "-M test.txt\n");
}

/// Bug 9: git-ignored files were left out of the `--loc=percent`
/// denominator but not the numerator, giving `1000.0%`.
#[test]
#[cfg(feature = "git")]
fn loc_percent_shares_one_denominator_with_ignored_files() {
    let repo = TempGitRepo::new("loc_git_ignored");
    repo.write_file("tracked.rs", b"fn main() {}\n");
    let ignored: String = (0..10).map(|i| format!("fn f{i}() {{}}\n")).collect();
    repo.write_file("ignored.rs", ignored.as_bytes());
    repo.write_file(".gitignore", b"ignored.rs\n");
    repo.git(&["add", ".gitignore", "tracked.rs"]);
    repo.git(&["commit", "-q", "-m", "init"]);

    for view in [&["-l"][..], &["-l", "--grid"][..]] {
        let mut args = view.to_vec();
        args.extend([
            "--loc=percent",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ]);
        let output = crate::common::lez_in(repo.path())
            .args(&args)
            .output()
            .expect("failed to run lez");
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert_eq!(
            text(&output.stdout),
            "Rust 90.9% ignored.rs\nRust  9.1% tracked.rs\n",
            "{args:?}"
        );
    }
}

/// Bug 10: `--code` counted a path once per time it was named.
#[test]
fn code_summary_counts_a_repeated_path_once() {
    let dir = TempTestDir::new("loc_dedupe");
    dir.create_file("main.rs", b"fn main() {}\n");

    let output = run(&dir, &["--code", "main.rs", "main.rs", "./main.rs"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!(
            " Language  Files  Lines  Code  Comments  Blanks  Code %\n\
             \x20Rust          1      1     1         0       0  100.0%  ████████████████\n\
             {}\n\
             \x20Total         1      1     1         0       0  100.0%\n",
            "─".repeat(73)
        )
    );
}

/// Bug 11: a literal `%%Z` in a custom time style panicked; it prints `%Z`.
#[test]
fn an_escaped_percent_z_in_a_time_style_prints_literally() {
    let dir = TempTestDir::new("percent_z");
    let path = dir.create_file("f.txt", b"x");
    let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|f| f.set_modified(mtime))
        .expect("set mtime");

    let output = lez_in(dir.path())
        .env("TZ", "UTC")
        .args([
            "-l",
            "--time-style=+%%Z|%Y",
            "--no-permissions",
            "--no-user",
            "f.txt",
        ])
        .output()
        .expect("failed to run lez");
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "1 %Z|2023 f.txt\n");

    let output = lez_in(dir.path())
        .env("TZ", "UTC")
        .args([
            "--json",
            "-l",
            "--time-style=+%Y-%%Z",
            "--no-permissions",
            "--no-user",
            "f.txt",
        ])
        .output()
        .expect("failed to run lez");
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "{\"f.txt\":{\"Size\":\"1\",\"Date Modified\":\"2023-%Z\"}}\n"
    );
}

/// Bug 12: C1 control characters reached the terminal raw, so U+009B could
/// start an escape sequence. They are printed as the octal escapes of their
/// UTF-8 bytes, inside the ANSI-C quotes that read them back.
#[test]
#[cfg(unix)]
fn c1_control_characters_in_names_are_escaped() {
    let dir = TempTestDir::new("c1_controls");
    dir.create_file("test\u{0085}nel\u{009b}31mcsi.txt", b"");

    let output = run(&dir, &["--color=never", "-1"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "$'test\\302\\205nel\\302\\23331mcsi.txt'\n"
    );
}

/// Bug 13: JSON reported every directory error as a permission failure.
#[test]
#[cfg(unix)]
fn json_reports_permission_errors_and_nothing_else_as_code_13() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempTestDir::new("json_error_code");
    let missing = dir.path().join("non_existent_dir");
    let output = run(&dir, &["--json", missing.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "[]\n");
    assert_eq!(
        text(&output.stderr),
        format!("{missing:?}: No such file or directory (os error 2)\n")
    );

    if !crate::common::permission_checks_apply() {
        return;
    }
    let locked = dir.create_dir("locked_dir");
    dir.create_file("locked_dir/file.txt", b"data");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let output = run(&dir, &["--json", "locked_dir"]);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(output.status.code(), Some(13));
    assert_eq!(text(&output.stdout), "[]\n");
    assert_eq!(
        text(&output.stderr),
        "Permission denied: locked_dir - code: 13\n"
    );
}

/// Bug 14: the JSON permission string lost the `D` that marks a mount point.
#[test]
#[cfg(unix)]
fn json_permissions_mark_mount_points_with_an_upper_case_d() {
    use std::os::unix::fs::PermissionsExt;

    // The nine `rwx` letters `stat` gives the directory at `path`.
    let letters = |path: &std::path::Path| -> String {
        let mode = fs::metadata(path).expect("stat").permissions().mode();
        (0..9)
            .map(|i| {
                if mode & (0o400 >> i) == 0 {
                    '-'
                } else {
                    ['r', 'w', 'x'][i % 3]
                }
            })
            .collect()
    };
    let dir = TempTestDir::new("json_mount");
    let plain = dir.create_dir("plain");
    // `--no-extended` keeps an `@` for the root's attributes, which macOS
    // may give it, off the end of the string.
    let args = [
        "--json",
        "-l",
        "-d",
        "--no-user",
        "--no-time",
        "--no-extended",
    ];

    let output = run(&dir, &[&args[..], &["/"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!(
            "{{\"/\":{{\"Permissions\":\"D{}\"}}}}\n",
            letters(std::path::Path::new("/"))
        )
    );

    let output = run(&dir, &[&args[..], &["plain"]].concat());
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        format!(
            "{{\"plain\":{{\"Permissions\":\"d{}\"}}}}\n",
            letters(&plain)
        )
    );
}
