// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Choosing a colour for a name means asking whether the file is
//! executable, and that means its mode, and that means a `stat` for every
//! regular file listed. When colours are off there is no colour to choose,
//! and `ls -1 --color=never` makes no such call — this is where we stopped
//! making it either.
//!
//! `LEZ_DEBUG` is the portable window onto that: `File::metadata` logs each
//! time it goes to the filesystem.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Only the syscall-counting tests need this fixture, and those are Unix
/// only: the Windows executable check reads `PATHEXT` and never stats.
#[cfg(unix)]
const ENTRIES: usize = 40;

#[cfg(unix)]
fn plain_files(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lez-colourless-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture directory");
    for i in 0..ENTRIES {
        fs::write(root.join(format!("f{i:03}")), b"").expect("fixture file");
    }
    root
}

fn run(args: &[&str], root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lez"))
        .args(args)
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lez")
}

#[cfg(unix)]
fn stat_count(args: &[&str], root: &Path) -> usize {
    let out = Command::new(env!("CARGO_BIN_EXE_lez"))
        .env("LEZ_DEBUG", "1")
        .args(args)
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lez");

    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|line| line.contains("Statting file"))
        .count()
}

/// The listing walks the directory once and prints what readdir gave it.
/// One stat for the directory named on the command line is expected; forty
/// more for its contents are not.
#[cfg(unix)]
#[test]
fn a_colourless_listing_does_not_stat_every_entry() {
    let root = plain_files("never");
    let stats = stat_count(&["-1", "--color=never"], &root);

    assert!(
        stats < ENTRIES,
        "listing {ENTRIES} files without colour took {stats} stats",
    );

    let _ = fs::remove_dir_all(&root);
}

/// And the check is skipped only because nothing needs it. Turn colours on
/// and the executable style has to be resolved, which means the mode, which
/// means the stat is back. If this ever stops being true, the shortcut has
/// grown past what it can justify.
#[cfg(unix)]
#[test]
fn a_coloured_listing_still_looks_for_executables() {
    let root = plain_files("always");
    let stats = stat_count(&["-1", "--color=always"], &root);

    assert!(
        stats >= ENTRIES,
        "listing {ENTRIES} files with colour took only {stats} stats, so the \
         executable check is no longer happening",
    );

    let _ = fs::remove_dir_all(&root);
}

/// The visible half of the same guard: with colours on, an executable is
/// painted. A shortcut applied too widely would show up here first, before
/// anyone counted a syscall.
#[cfg(unix)]
#[test]
fn a_coloured_listing_still_paints_executables() {
    let root = one_of_everything("painted");
    let out = Command::new(env!("CARGO_BIN_EXE_lez"))
        .env("LEZ_COLORS", "ex=31")
        .args(["-1", "--color=always"])
        .arg(root.to_str().unwrap())
        .output()
        .expect("failed to execute lez");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\u{1b}[31mscript.sh\u{1b}[0m"),
        "the executable should be painted; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// A directory holding one of everything the style code branches on.
fn one_of_everything(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("lez-colourless-{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("adir")).expect("fixture directory");
    fs::write(root.join("plain.txt"), b"").expect("plain file");
    fs::write(root.join("script.sh"), b"").expect("script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.join("script.sh"), fs::Permissions::from_mode(0o755))
            .expect("chmod");
        std::os::unix::fs::symlink("plain.txt", root.join("good.link")).expect("symlink");
        std::os::unix::fs::symlink("nowhere", root.join("broken.link")).expect("broken symlink");
    }

    root
}

/// Skipping the choice must not change what is printed.
#[test]
fn the_names_are_unchanged() {
    let root = one_of_everything("names");
    let out = run(&["-1", "--color=never"], &root);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    for expected in ["adir", "plain.txt", "script.sh"] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }
    #[cfg(unix)]
    for expected in ["good.link", "broken.link"] {
        assert!(
            names.contains(&expected),
            "{expected} missing from {names:?}"
        );
    }

    let _ = fs::remove_dir_all(&root);
}

/// `--classify` needs the executable bit for its own reasons and is not
/// part of the colour question, so it keeps paying for the stat and keeps
/// marking the file.
#[cfg(unix)]
#[test]
fn classify_still_marks_executables_without_colour() {
    let root = one_of_everything("classify");
    let out = run(&["-1", "--classify=always", "--color=never"], &root);

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("script.sh*"),
        "the executable should still be marked; got {stdout:?}",
    );
    assert!(
        stdout.contains("adir/"),
        "the directory should still be marked; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}

/// The long view reads metadata for its own columns, so the shortcut must
/// not have taken anything away from it.
#[cfg(unix)]
#[test]
fn the_long_view_is_unaffected() {
    let root = one_of_everything("long");
    let out = run(&["-l", "--color=never"], &root);

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("rwxr-xr-x") && stdout.contains("script.sh"),
        "the long view should still show the executable's mode; got {stdout:?}",
    );
    assert!(
        stdout.contains("broken.link"),
        "and its broken symlink; got {stdout:?}",
    );

    let _ = fs::remove_dir_all(&root);
}
