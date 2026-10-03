// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Data files (Parquet, CSV, NumPy arrays, HDF5, SQLite) take the `dt`
//! colour, and each format its own icon.

use crate::common::{TempTestDir, lez_in, success_stdout};

const NAMES: [&str; 6] = [
    "database.sqlite",
    "dataset.parquet",
    "db.sqlite3",
    "embeddings.npy",
    "records.csv",
    "store.h5",
];

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("data_files");
    for name in NAMES {
        dir.create_file(name, b"");
    }
    dir
}

#[test]
fn data_files_take_the_dt_colour() {
    let dir = fixture();
    let expected: String = NAMES
        .iter()
        .map(|name| format!("\x1b[1;35m{name}\x1b[0m\n"))
        .collect();
    assert_eq!(
        success_stdout(
            lez_in(dir.path())
                .env("LEZ_COLORS", "dt=35;1")
                .args(["-1", "--color=always"])
        ),
        expected
    );
}

#[test]
fn each_data_format_has_its_icon() {
    let dir = fixture();
    let (database, sqlite, python, csv) = ('\u{f1c0}', '\u{e7c4}', '\u{e606}', '\u{eefc}');
    assert_eq!(
        success_stdout(lez_in(dir.path()).args(["-1", "--icons=always"])),
        format!(
            "{sqlite} database.sqlite\n{database} dataset.parquet\n{sqlite} db.sqlite3\n\
             {python} embeddings.npy\n{csv} records.csv\n{database} store.h5\n"
        )
    );
}
