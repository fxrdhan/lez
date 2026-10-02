// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The user's own folders (home, configuration, desktop, documents,
//! downloads, music, pictures, videos) have icons of their own. lez finds
//! them where the platform says they are: under `$HOME` on macOS, and
//! through `$XDG_CONFIG_HOME/user-dirs.dirs` on Linux. The test gives lez a
//! home of its own, so the host's folders never come into it. (Windows asks
//! the shell for them, which an environment variable cannot redirect.)

#![cfg(any(target_os = "linux", target_os = "macos"))]

use crate::common::{TempTestDir, lez_cmd, lez_in, success_stdout};

#[test]
fn the_users_folders_get_their_own_icons() {
    let dir = TempTestDir::new("special_dirs");
    // Matched against the absolute path lez builds from its working
    // directory, which macOS reports with `/private` in front.
    let home = dir.path().canonicalize().expect("canonicalize");
    for folder in [
        "Desktop",
        "Documents",
        "Downloads",
        "Music",
        "Pictures",
        "Plain",
    ] {
        dir.create_dir(folder);
    }

    #[cfg(target_os = "linux")]
    let expected = {
        dir.create_dir("Videos");
        dir.create_file(
            ".config/user-dirs.dirs",
            b"XDG_DESKTOP_DIR=\"$HOME/Desktop\"\n\
              XDG_DOCUMENTS_DIR=\"$HOME/Documents\"\n\
              XDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\n\
              XDG_MUSIC_DIR=\"$HOME/Music\"\n\
              XDG_PICTURES_DIR=\"$HOME/Pictures\"\n\
              XDG_VIDEOS_DIR=\"$HOME/Videos\"\n",
        );
        "\u{e5fc} .config\n\u{f108} Desktop\n\u{f0c82} Documents\n\u{f024d} Downloads\n\
         \u{f1359} Music\n\u{f024f} Pictures\n\u{f115} Plain\n\u{f03d} Videos\n"
    };
    // The configuration folder is `Library/Application Support`, so
    // `Library` itself is a plain folder.
    #[cfg(target_os = "macos")]
    let expected = {
        dir.create_dir("Movies");
        dir.create_dir("Library/Application Support");
        "\u{f108} Desktop\n\u{f0c82} Documents\n\u{f024d} Downloads\n\u{e5ff} Library\n\
         \u{f0fce} Movies\n\u{f1359} Music\n\u{f024f} Pictures\n\u{f115} Plain\n"
    };

    assert_eq!(
        success_stdout(
            lez_in(&home)
                .env("HOME", &home)
                .args(["-1", "-a", "--icons=always"])
        ),
        expected
    );
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
