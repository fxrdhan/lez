// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The width the grid fits names into: `--width`, then `COLUMNS`, capped at
//! 65535. How the options are read is unit tested in `src/options/view.rs`;
//! these runs check the listing that comes out.

use crate::common::{TempTestDir, grouped, lez_in, success_stdout};

/// Twenty names of 36 columns.
fn twenty_names() -> (TempTestDir, Vec<String>) {
    let dir = TempTestDir::new("grid_width");
    let names: Vec<String> = (1..=20)
        .map(|i| format!("long_filename_entry_number_{i:04}.txt"))
        .collect();
    for name in &names {
        dir.create_file(name, b"test");
    }
    (dir, names)
}

/// A grid too narrow for two names gives one per line, 80 columns fit two
/// (filled down the first column), and any width up to and past the 65535
/// cap fits all twenty on one row.
#[test]
fn the_grid_follows_the_width_it_is_given() {
    let (dir, names) = twenty_names();
    let grid = |width: &str| success_stdout(lez_in(dir.path()).args(["--grid", "--width", width]));

    let one_per_line: String = names.iter().map(|name| format!("{name}\n")).collect();
    for width in ["1", "2", "3", "40"] {
        assert_eq!(grid(width), one_per_line, "--width {width}");
    }
    let two_columns: String = (0..10)
        .map(|row| format!("{}  {}\n", names[row], names[row + 10]))
        .collect();
    assert_eq!(grid("80"), two_columns);
    let one_row = format!("{}\n", names.join("  "));
    assert_eq!(grid("65535"), one_row);
    assert_eq!(grid("100000"), one_row);
}

/// `COLUMNS` gives the width when `--width` does not, and one that is not a
/// number is an option error naming it.
#[test]
fn columns_gives_the_width_unless_the_flag_does() {
    let (dir, names) = twenty_names();
    let grid = |columns: &str, args: &[&str]| {
        lez_in(dir.path())
            .env("COLUMNS", columns)
            .arg("--grid")
            .args(args)
            .output()
            .expect("run lez")
    };
    let stdout = |output: std::process::Output| {
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8(output.stdout).expect("UTF-8")
    };
    let two_columns: String = (0..10)
        .map(|row| format!("{}  {}\n", names[row], names[row + 10]))
        .collect();
    let one_row = format!("{}\n", names.join("  "));

    assert_eq!(stdout(grid("80", &[])), two_columns);
    assert_eq!(stdout(grid("100000", &[])), one_row);
    assert_eq!(stdout(grid("80", &["--width=100000"])), one_row);

    for (value, reason) in [
        ("wide", "invalid digit found in string"),
        ("-50", "invalid digit found in string"),
        ("", "cannot parse integer from empty string"),
    ] {
        let output = grid(value, &[]);
        assert_eq!(output.status.code(), Some(3), "{value:?}");
        assert!(output.stdout.is_empty(), "{value:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("lez: Value {value:?} not valid for environment variable COLUMNS: {reason}\n")
        );
    }
}

/// Of `-b` (binary units) and `-B` (bytes) the last given wins.
#[test]
fn the_last_size_unit_flag_wins_in_the_listing() {
    let dir = TempTestDir::new("size_units");
    dir.create_file("file_2048.txt", &[0u8; 2048]);
    let size = |flags: &[&str]| {
        success_stdout(
            lez_in(dir.path())
                .args(["-l", "--no-permissions", "--no-user", "--no-time"])
                .args(flags),
        )
    };
    let bytes = format!("{} file_2048.txt\n", grouped(2048));
    let binary = "2.0Ki file_2048.txt\n";
    assert_eq!(size(&[]), "2.0k file_2048.txt\n");
    assert_eq!(size(&["-b", "-B"]), bytes);
    assert_eq!(size(&["-B", "-b"]), binary);
    assert_eq!(size(&["--binary", "--bytes"]), bytes);
    assert_eq!(size(&["--bytes", "--binary"]), binary);
    assert_eq!(size(&["-b", "-B", "-b", "-B"]), bytes);
    assert_eq!(size(&["-B", "-b", "-B", "-b"]), binary);
}
