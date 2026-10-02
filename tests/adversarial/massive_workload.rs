// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A directory of well over a thousand entries of mixed kinds, checked by
//! exact counts: a large listing is where an entry quietly going missing
//! would hide.

use std::process::Output;

use crate::common::{TempTestDir, lez_in};

const DATA_FILES: usize = 1200;
const EMPTY_FILES: usize = 10;
const SUBFOLDERS: usize = 5;

/// Entries every platform gets: data files, empty files, one sparse file and
/// some subfolders. Unix adds a FIFO and two symlinks.
fn corpus() -> TempTestDir {
    let dir = TempTestDir::new("massive");
    for i in 0..DATA_FILES {
        dir.create_file(
            &format!("data_{i:04}.dat"),
            format!("payload {i}\n").as_bytes(),
        );
    }
    for i in 0..EMPTY_FILES {
        dir.create_empty_file(&format!("empty_{i}.zero"));
    }
    std::fs::File::create(dir.path().join("sparse_large.bin"))
        .and_then(|f| f.set_len(10 * 1024 * 1024))
        .expect("create sparse file");
    for d in 0..SUBFOLDERS {
        dir.create_file(&format!("subfolder_{d}/nested.txt"), b"nested");
    }
    #[cfg(unix)]
    {
        let fifo =
            std::ffi::CString::new(dir.path().join("test_pipe.fifo").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0, "mkfifo");
        dir.create_symlink("data_0000.dat", "link_valid.lnk");
        dir.create_symlink("non_existent_target.missing", "link_dangling.lnk");
    }
    dir
}

const SPECIAL: usize = if cfg!(unix) { 3 } else { 0 };
const TOTAL: usize = DATA_FILES + EMPTY_FILES + 1 + SUBFOLDERS + SPECIAL;

fn run(dir: &TempTestDir, args: &[&str]) -> String {
    let output: Output = lez_in(dir.path())
        .args(args)
        .output()
        .expect("failed to run lez");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("UTF-8 stdout")
}

#[test]
fn every_entry_is_listed_exactly_once_in_lines_and_grid() {
    let dir = corpus();

    let lines = run(&dir, &["-1"]);
    assert_eq!(lines.lines().count(), TOTAL);
    let mut sorted: Vec<&str> = lines.lines().collect();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), TOTAL, "no entry may repeat");

    let grid = run(&dir, &["-G", "--width=120"]);
    let mut cells: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for cell in grid.split_whitespace() {
        *cells.entry(cell).or_default() += 1;
    }
    for name in lines.lines() {
        assert_eq!(cells.get(name), Some(&1), "{name} in the grid");
    }
    assert_eq!(cells.len(), TOTAL, "the grid holds nothing else");
}

#[test]
fn the_long_view_and_json_hold_one_record_per_entry() {
    let dir = corpus();

    assert_eq!(run(&dir, &["-l"]).lines().count(), TOTAL);

    let json: serde_json::Value =
        serde_json::from_str(&run(&dir, &["--json"])).expect("valid JSON");
    assert_eq!(json.as_array().expect("JSON array").len(), TOTAL);
}

#[test]
fn summary_and_print_total_count_every_entry() {
    let dir = corpus();
    let files = DATA_FILES + EMPTY_FILES + 1 + usize::from(cfg!(unix));
    let symlinks = if cfg!(unix) { 2 } else { 0 };

    let summary = run(&dir, &["-l", "--summary"]);
    assert_eq!(
        summary.lines().last(),
        Some(
            format!("{SUBFOLDERS} directories, {files} files, {symlinks} symlinks ({TOTAL} total)")
                .as_str()
        )
    );

    let total = run(&dir, &["-l", "--print-total"]);
    assert_eq!(
        total.lines().last(),
        Some(format!("total: {TOTAL}").as_str())
    );
}

#[test]
fn size_sorting_puts_the_sparse_file_by_its_length() {
    let dir = corpus();

    let by_size = run(&dir, &["-1", "--sort=size", "-r"]);
    assert_eq!(by_size.lines().next(), Some("sparse_large.bin"));
}

#[test]
#[cfg(unix)]
fn block_sorting_puts_the_sparse_file_by_its_allocation() {
    let dir = corpus();

    // Nothing is allocated for the sparse file, so it sorts among the empty
    // entries and ahead of every data file.
    let by_blocks: Vec<String> = run(&dir, &["-1", "--sort=blocks"])
        .lines()
        .map(str::to_owned)
        .collect();
    let sparse = by_blocks
        .iter()
        .position(|name| name == "sparse_large.bin")
        .expect("sparse file listed");
    let first_data = by_blocks
        .iter()
        .position(|name| name.starts_with("data_"))
        .expect("data files listed");
    assert!(sparse < first_data, "{by_blocks:?}");
}
