// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Choosing a colour for a name means asking whether the file is
//! executable, and that means its mode, and that means a `stat` for every
//! regular file listed. When colours are off there is no colour to choose,
//! and `ls -1 --color=never` makes no such call — this is where we stopped
//! making it either.
//!
//! `LEZ_DEBUG` is the portable window onto that: `File::metadata` logs each
//! time it goes to the filesystem, so the tests below compare the exact set
//! of paths it went to.

#[cfg(unix)]
use std::path::{Path, PathBuf};

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Every path lez stats while listing `root` with `args`, and with
/// `LEZ_COLORS` set to `colours` when given, sorted as `LEZ_DEBUG` logs them.
#[cfg(unix)]
fn statted(root: &Path, colours: Option<&str>, args: &[&str]) -> Vec<String> {
    let mut cmd = crate::common::lez_cmd();
    if let Some(colours) = colours {
        cmd.env("LEZ_COLORS", colours);
    }
    let output = cmd
        .env("LEZ_DEBUG", "1")
        .args(args)
        .arg(root)
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    let mut paths: Vec<String> = String::from_utf8(output.stderr)
        .expect("UTF-8 stderr")
        .lines()
        .filter_map(|line| {
            line.split_once(" Statting file ")
                .map(|(_, path)| path.to_owned())
        })
        .collect();
    paths.sort();
    paths
}

/// `paths` as `LEZ_DEBUG` writes them, sorted.
#[cfg(unix)]
fn logged(paths: impl IntoIterator<Item = PathBuf>) -> Vec<String> {
    let mut paths: Vec<String> = paths.into_iter().map(|path| format!("{path:?}")).collect();
    paths.sort();
    paths
}

/// Forty empty files whose names have no extension, so no theme rule can
/// colour them by name.
#[cfg(unix)]
fn plain_files() -> (TempTestDir, Vec<PathBuf>) {
    let dir = TempTestDir::new("colourless");
    let files = (0..40)
        .map(|i| dir.create_file(&format!("f{i:03}"), b""))
        .collect();
    (dir, files)
}

/// A directory holding one of everything the style code branches on, with
/// the modes pinned so the umask does not show in the long view.
fn one_of_everything(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    let adir = dir.create_dir("adir");
    let plain = dir.create_file("plain.txt", b"");
    let script = dir.create_file("script.sh", b"");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, mode) in [(&adir, 0o755), (&plain, 0o644), (&script, 0o755)] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
        }
        dir.create_symlink("plain.txt", "good.link");
        dir.create_symlink("nowhere", "broken.link");
    }
    #[cfg(not(unix))]
    let _ = (adir, plain, script);
    dir
}

/// The listing walks the directory once and prints what readdir gave it.
/// The directory named on the command line is statted; its forty files are
/// not. `--color=auto` is off here too, as stdout is a pipe.
#[cfg(unix)]
#[test]
fn a_colourless_listing_stats_only_the_directory_it_was_given() {
    let (dir, _) = plain_files();
    for args in [["-1", "--color=never"], ["-1", "--color=auto"]] {
        assert_eq!(
            statted(dir.path(), None, &args),
            logged([dir.path().to_path_buf()]),
            "{args:?}"
        );
    }
}

/// And the check is skipped only because nothing needs it. Turn colours on
/// and the executable style has to be resolved for a file nothing else
/// colours, which means its mode, which means the stat is back. If this ever
/// stops being true, the shortcut has grown past what it can justify.
#[cfg(unix)]
#[test]
fn a_coloured_listing_still_looks_for_executables() {
    let (dir, files) = plain_files();
    assert_eq!(
        statted(dir.path(), None, &["-1", "--color=always"]),
        logged(std::iter::once(dir.path().to_path_buf()).chain(files))
    );
}

/// A regular file whose name already picks its colour is not statted: the
/// built-in theme colours `*.txt`, and so does an `LEZ_COLORS` glob. Clear
/// the theme with `reset` and `plain.txt` has to be asked about again.
/// Directories and links take their style from what readdir reported.
#[cfg(unix)]
#[test]
fn a_name_that_picks_the_colour_spares_the_stat() {
    let dir = one_of_everything("named_style");
    let root = dir.path().to_path_buf();
    let run = |colours| statted(&root, colours, &["-1", "--color=always"]);
    let script_only = logged([root.clone(), root.join("script.sh")]);
    assert_eq!(run(None), script_only);
    assert_eq!(run(Some("*.txt=33")), script_only);
    assert_eq!(
        run(Some("reset:ex=31")),
        logged([root.clone(), root.join("plain.txt"), root.join("script.sh")])
    );
}

/// The visible half of the same guard: with colours on, the executable is
/// painted, and with the theme cleared it is the only thing that is.
#[cfg(unix)]
#[test]
fn a_coloured_listing_still_paints_executables() {
    let dir = one_of_everything("painted");
    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_COLORS", "reset:ex=31")
                .args(["-1", "--color=always"])
        ),
        "adir\nbroken.link\ngood.link\nplain.txt\n\x1b[31mscript.sh\x1b[0m\n"
    );
}

/// Skipping the choice must not change what is printed.
#[test]
fn the_names_are_unchanged() {
    let dir = one_of_everything("names");
    let expected = if cfg!(unix) {
        "adir\nbroken.link\ngood.link\nplain.txt\nscript.sh\n"
    } else {
        "adir\nplain.txt\nscript.sh\n"
    };
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--color=never"])),
        expected
    );
}

/// `--classify` needs the executable bit for its own reasons and is not
/// part of the colour question, so it keeps paying for the stat of each
/// regular file and keeps marking them.
#[cfg(unix)]
#[test]
fn classify_still_marks_executables_without_colour() {
    let dir = one_of_everything("classify");
    let args = ["-1", "--classify=always", "--color=never"];
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(args)),
        "adir/\nbroken.link@\ngood.link@\nplain.txt\nscript.sh*\n"
    );
    let root = dir.path().to_path_buf();
    assert_eq!(
        statted(&root, None, &args),
        logged([root.clone(), root.join("plain.txt"), root.join("script.sh")])
    );
}

/// The long view reads metadata for its own columns, so the shortcut must
/// not have taken anything away from it.
#[cfg(unix)]
#[test]
fn the_long_view_is_unaffected() {
    let dir = one_of_everything("long");
    let link = |name: &str| crate::common::symlink_permissions(&dir.path().join(name));
    assert_eq!(
        success_stdout(lez_in(dir.path()).args([
            "-l",
            "--color=never",
            "--no-user",
            "--no-time",
            "--no-filesize",
        ])),
        format!(
            "drwxr-xr-x adir\n\
             {} broken.link -> nowhere\n\
             {} good.link -> plain.txt\n\
             .rw-r--r-- plain.txt\n\
             .rwxr-xr-x script.sh\n",
            link("broken.link"),
            link("good.link")
        )
    );
}
