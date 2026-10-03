// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The user's own folders (home, configuration, desktop, documents,
//! downloads, music, pictures, videos) have icons of their own. lez finds
//! them where the platform says they are: under `$HOME` on macOS, and
//! through `$XDG_CONFIG_HOME` and its `user-dirs.dirs` on Linux. The test
//! gives lez a home of its own, so the host's folders never come into it.
//! (Windows asks the shell for them, which an environment variable cannot
//! redirect.)
//!
//! A folder has its icon however it is reached: listed from its parent,
//! from inside the home, or by a relative path.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::ffi::OsStr;
use std::path::Path;

use crate::common::{TempTestDir, lez_cmd, lez_in, success_stdout};

#[test]
fn the_users_folders_get_their_own_icons() {
    let dir = TempTestDir::new("special_dirs");
    // Matched against the absolute path lez builds from its working
    // directory, which macOS reports with `/private` in front.
    let home = dir.path().canonicalize().expect("canonicalize");
    dir.create_dir("Plain");

    // Names the icon table does not know, so only the folders' places
    // can give them their icons.
    #[cfg(target_os = "linux")]
    let (folders, expected) = {
        dir.create_file(
            "Settings/user-dirs.dirs",
            b"XDG_DESKTOP_DIR=\"$HOME/Desk\"\n\
              XDG_DOCUMENTS_DIR=\"$HOME/Docs\"\n\
              XDG_DOWNLOAD_DIR=\"$HOME/Fetched\"\n\
              XDG_MUSIC_DIR=\"$HOME/Tunes\"\n\
              XDG_PICTURES_DIR=\"$HOME/Photos\"\n\
              XDG_VIDEOS_DIR=\"$HOME/Clips\"\n",
        );
        (
            ["Clips", "Desk", "Docs", "Fetched", "Photos", "Tunes"],
            "\u{f03d} Clips\n\u{f108} Desk\n\u{f0c82} Docs\n\u{f024d} Fetched\n\
             \u{f024f} Photos\n\u{f115} Plain\n\u{e5fc} Settings\n\u{f1359} Tunes\n",
        )
    };
    // The names are fixed here. The configuration folder is
    // `Library/Application Support`, so `Library` itself is a plain folder;
    // `Movies` is the videos folder, whose icon is not the one the table
    // gives the name.
    #[cfg(target_os = "macos")]
    let (folders, expected) = {
        dir.create_dir("Library/Application Support");
        (
            [
                "Desktop",
                "Documents",
                "Downloads",
                "Movies",
                "Music",
                "Pictures",
            ],
            "\u{f108} Desktop\n\u{f0c82} Documents\n\u{f024d} Downloads\n\u{e5ff} Library\n\
             \u{f03d} Movies\n\u{f1359} Music\n\u{f024f} Pictures\n\u{f115} Plain\n",
        )
    };
    for folder in folders {
        dir.create_dir(folder);
    }
    let run = |cwd: &Path, args: &[&OsStr]| {
        let mut cmd = lez_in(cwd);
        #[cfg(target_os = "linux")]
        cmd.env("XDG_CONFIG_HOME", home.join("Settings"));
        success_stdout(cmd.env("HOME", &home).arg("--icons=always").args(args))
    };

    // From inside the home, which holds relative paths.
    assert_eq!(run(&home, &["-1".as_ref()]), expected);
    // From elsewhere, by the home's absolute path.
    let elsewhere = TempTestDir::new("special_dirs_elsewhere");
    assert_eq!(
        run(elsewhere.path(), &["-1".as_ref(), home.as_os_str()]),
        expected
    );
    // From a sibling, each by `../` and its name.
    let siblings: Vec<String> = folders.iter().map(|f| format!("../{f}")).collect();
    let mut args = vec![OsStr::new("-1d")];
    args.extend(siblings.iter().map(OsStr::new));
    let by_sibling: String = expected
        .lines()
        .filter_map(|line| {
            let (icon, name) = line.split_once(' ')?;
            folders
                .contains(&name)
                .then(|| format!("{icon} ../{name}\n"))
        })
        .collect();
    assert_eq!(run(&home.join("Plain"), &args), by_sibling);

    // The home folder itself, as an entry.
    assert_eq!(
        success_stdout(
            lez_cmd()
                .env("HOME", &home)
                .args(["-d", "--icons=always"])
                .arg(&home)
        ),
        format!("\u{f10b5} {}\n", home.display())
    );
}
