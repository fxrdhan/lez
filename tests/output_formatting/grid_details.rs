// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--long --grid`: the rows of the long view, laid out as a grid when there
//! is a width to fit them to. Each cell is exactly the long view's row, so
//! the long view itself is the oracle for what goes in a cell.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// `1.txt` to `4.txt`, of 1 to 4 bytes, so the size column tells them apart.
fn fixture(prefix: &str) -> TempTestDir {
    let dir = TempTestDir::new(prefix);
    for n in 1..=4 {
        dir.create_file(&format!("{n}.txt"), &vec![b'x'; n]);
    }
    dir
}

const SIZE_ONLY: [&str; 3] = ["--no-permissions", "--no-user", "--no-time"];

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    success_stdout(lez_in(dir.path()).args(SIZE_ONLY).args(args))
}

/// The long view's rows, in order.
fn long_rows(dir: &TempTestDir, extra: &[&str]) -> Vec<String> {
    run(dir, &[&["-l"][..], extra].concat())
        .lines()
        .map(str::to_owned)
        .collect()
}

/// With room for every cell on one row, the grid is the long view's rows
/// four spaces apart: the same spacing between the columns and the name,
/// whatever `--spacing` says, and no column left over when every column
/// is turned off.
#[test]
fn each_cell_is_the_long_views_row() {
    let dir = fixture("cells");
    for extra in [
        &[][..],
        &["--spacing=3"],
        &["--spacing=0"],
        &["--no-filesize"],
    ] {
        assert_eq!(
            run(&dir, &[&["-lG", "--width=200"][..], extra].concat()),
            format!("{}\n", long_rows(&dir, extra).join("    ")),
            "{extra:?}"
        );
    }
    assert_eq!(
        run(&dir, &["-lG", "--width=200"]),
        "1 1.txt    2 2.txt    3 3.txt    4 4.txt\n"
    );
    assert_eq!(
        run(&dir, &["-lG", "--width=200", "--no-filesize"]),
        "1.txt    2.txt    3.txt    4.txt\n"
    );
}

/// Cells go down the columns, or across the rows with `--across`, as many
/// to a row as the width holds.
#[test]
fn cells_go_down_or_across_as_the_width_allows() {
    let dir = fixture("layout");
    for (args, expected) in [
        (
            &["-lG", "--width=30"][..],
            "1 1.txt    3 3.txt\n2 2.txt    4 4.txt\n",
        ),
        (
            &["-lG", "--across", "--width=30"],
            "1 1.txt    2 2.txt    3 3.txt\n4 4.txt\n",
        ),
        (
            &["-lG", "--width=40"],
            "1 1.txt    2 2.txt    3 3.txt    4 4.txt\n",
        ),
        // Narrower than one cell: one cell to a row.
        (
            &["-lG", "--width=5"],
            "1 1.txt\n2 2.txt\n3 3.txt\n4 4.txt\n",
        ),
    ] {
        assert_eq!(run(&dir, args), expected, "{args:?}");
    }
}

/// With no width to fit, nor a terminal to take one from, the long grid is
/// the long view; and `LEZ_GRID_ROWS` asks for the long view whenever the
/// grid would have fewer rows than it says.
#[test]
fn the_long_view_when_there_is_no_width_or_too_few_rows() {
    let dir = fixture("fallback");
    let long = run(&dir, &["-l"]);
    assert_eq!(long, "1 1.txt\n2 2.txt\n3 3.txt\n4 4.txt\n");
    assert_eq!(run(&dir, &["-lG"]), long);

    let grid = |rows: &str, width: &str| {
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_GRID_ROWS", rows)
                .args(SIZE_ONLY)
                .args(["-lG", width]),
        )
    };
    let one_row = "1 1.txt    2 2.txt    3 3.txt    4 4.txt\n";
    let two_rows = "1 1.txt    3 3.txt\n2 2.txt    4 4.txt\n";
    assert_eq!(grid("1", "--width=200"), one_row);
    assert_eq!(grid("2", "--width=200"), long);
    assert_eq!(grid("2", "--width=30"), two_rows);
    assert_eq!(grid("3", "--width=30"), long);
}
