// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--mime-types` (or `LEZ_MIME_TYPES`, `EZA_MIME_TYPES`, set to anything)
//! sniffs the contents of files whose names say nothing, and styles them by
//! what they turn out to be. Directories are not sniffed. A theme's
//! `mimetypes` section styles a type of its own.

use crate::common::{TempTestDir, lez_in, success_stdout};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\
                     \x08\x06\x00\x00\x00\x1f\x15c4";

/// Files with no extension holding a PNG, a GIF, a gzip stream, a Python
/// script, C source and plain text, beside a directory.
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("mime");
    dir.create_file("png_data", PNG);
    dir.create_file(
        "gif_data",
        b"GIF89a\x01\x00\x01\x00\x80\x00\x00\xff\xff\xff\x00\x00\x00,\x00\x00\x00\x00\
          \x01\x00\x01\x00\x00\x02\x02D\x01\x00;",
    );
    dir.create_file(
        "gz_data",
        b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03\x03\x00\x00\x00\x00\x00\x00\x00\x00\x00",
    );
    dir.create_file(
        "py_script",
        b"#!/usr/bin/env python3\nimport sys\nprint(\"hello\")\n",
    );
    dir.create_file(
        "c_source",
        b"#include <stdio.h>\nint main(void) { return 0; }\n",
    );
    dir.create_file("text_data", b"plain words\n");
    dir.create_dir("folder");
    dir
}

fn run(dir: &TempTestDir, envs: &[(&str, &str)], args: &[&str]) -> String {
    let mut cmd = lez_in(dir.path());
    for (key, value) in envs {
        cmd.env(key, value);
    }
    success_stdout(cmd.arg("-1").args(args))
}

const UNKNOWN: char = '\u{f086f}';
const FOLDER: char = '\u{f115}';

#[test]
fn sniffing_gives_each_file_its_types_icon() {
    let dir = fixture();
    assert_eq!(
        run(&dir, &[], &["--icons=always"]),
        format!(
            "{UNKNOWN} c_source\n{FOLDER} folder\n{UNKNOWN} gif_data\n{UNKNOWN} gz_data\n\
             {UNKNOWN} png_data\n{UNKNOWN} py_script\n{UNKNOWN} text_data\n"
        )
    );
    let sniffed = format!(
        "\u{e61e} c_source\n{FOLDER} folder\n\u{f1c5} gif_data\n\u{f410} gz_data\n\
         \u{f1c5} png_data\n\u{e606} py_script\n{UNKNOWN} text_data\n"
    );
    assert_eq!(run(&dir, &[], &["--icons=always", "--mime-types"]), sniffed);
    for (var, value) in [
        ("LEZ_MIME_TYPES", "1"),
        ("LEZ_MIME_TYPES", ""),
        ("EZA_MIME_TYPES", "1"),
    ] {
        assert_eq!(
            run(&dir, &[(var, value)], &["--icons=always"]),
            sniffed,
            "{var}={value:?}"
        );
    }
}

/// The same for colours: images magenta, archives red, source bold yellow.
#[test]
fn sniffing_gives_each_file_its_types_colour() {
    let dir = fixture();
    assert_eq!(
        run(&dir, &[], &["--color=always", "--mime-types"]),
        "\x1b[1;33mc_source\x1b[0m\n\x1b[1;34mfolder\x1b[0m\n\x1b[35mgif_data\x1b[0m\n\
         \x1b[31mgz_data\x1b[0m\n\x1b[35mpng_data\x1b[0m\n\x1b[1;33mpy_script\x1b[0m\n\
         text_data\n"
    );
}

#[test]
fn a_theme_can_style_a_type() {
    let dir = TempTestDir::new("mime_theme");
    dir.create_file("photo_file", PNG);
    dir.create_file(
        ".config/theme.yml",
        "mimetypes:\n  image/png:\n    filename:\n      foreground: Cyan\n    icon:\n      glyph: \"🖼️\"\n"
            .as_bytes(),
    );
    assert_eq!(
        run(
            &dir,
            &[(
                "LEZ_CONFIG_DIR",
                dir.path().join(".config").to_str().expect("UTF-8")
            )],
            &["--icons=always", "--mime-types", "--color=always"]
        ),
        "\x1b[36m🖼️ photo_file\x1b[0m\n"
    );
}
