// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The grid: how many names fit on a row, and the gap between columns.
//!
//! Each column is as wide as its own widest name, not the widest in the
//! listing, so the nine names below, which need 79 columns with two spaces
//! between them, go on one row at exactly 79. Sizing every column to the
//! widest name used to spill them onto a second row even at 96.

use crate::common::{TempTestDir, lez_in, success_stdout};

const NAMES: [&str; 9] = [
    "code",
    "Desktop",
    "Documents",
    "Downloads",
    "Music",
    "Pictures",
    "Public",
    "Templates",
    "Videos",
];

fn names() -> TempTestDir {
    let dir = TempTestDir::new("grid_names");
    for name in NAMES {
        dir.create_file(name, b"");
    }
    dir
}

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(args))
}

#[test]
fn each_column_is_as_wide_as_its_own_names() {
    let dir = names();
    let one_row = format!("{}\n", NAMES.join("  "));
    for (width, expected) in [
        ("96", one_row.as_str()),
        ("79", &one_row),
        (
            "78",
            "code     Documents  Music     Public     Videos\n\
             Desktop  Downloads  Pictures  Templates\n",
        ),
        (
            "40",
            "code       Downloads  Public\n\
             Desktop    Music      Templates\n\
             Documents  Pictures   Videos\n",
        ),
    ] {
        assert_eq!(
            run(&dir, &["--grid", "--width", width]),
            expected,
            "{width}"
        );
    }
}

/// `--spacing` sets the gap between the grid's columns, two by default, and
/// between the long view's columns, one by default. Nothing stops it being
/// zero.
#[test]
fn spacing_sets_the_gap_between_columns() {
    let dir = names();
    for (args, gap) in [
        (&[][..], "  "),
        (&["--spacing=0"], ""),
        (&["--spacing=1"], " "),
        (&["--spacing=5"], "     "),
    ] {
        assert_eq!(
            run(&dir, &[&["--grid", "--width=200"][..], args].concat()),
            format!("{}\n", NAMES.join(gap)),
            "{args:?}"
        );
    }
    assert_eq!(
        run(&dir, &["--grid", "--width=30", "--spacing=0"]),
        "code     DownloadsPublic\n\
         Desktop  Music    Templates\n\
         DocumentsPictures Videos\n"
    );

    let sized = TempTestDir::new("long_spacing");
    sized.create_file("a.txt", b"1");
    sized.create_file("b.txt", b"22");
    let size_only = ["-l", "--no-permissions", "--no-user", "--no-time"];
    for (args, gap) in [
        (&[][..], " "),
        (&["--spacing=0"], ""),
        (&["--spacing=3"], "   "),
    ] {
        assert_eq!(
            run(&sized, &[&size_only[..], args].concat()),
            format!("1{gap}a.txt\n2{gap}b.txt\n"),
            "{args:?}"
        );
    }
}

#[test]
fn spacing_must_be_a_whole_number() {
    let dir = names();
    for value in ["-1", "x"] {
        let output = lez_in(dir.path())
            .arg(format!("--spacing={value}"))
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(3), "{value}");
        assert!(output.stdout.is_empty(), "{value}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!(
                "error: invalid value '{value}' for '--spacing <SPACES>': \
                 invalid digit found in string\n\nFor more information, try '--help'.\n"
            )
        );
    }
}
