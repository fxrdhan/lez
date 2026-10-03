// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! A directory of well over a thousand entries of mixed kinds, compared
//! whole: a large listing is where an entry quietly going missing would
//! hide.

use std::process::Output;

use crate::common::{NAME_COLUMN_ONLY, TempTestDir, lez_in};

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

/// The corpus's names in the order lez lists them, each with what the long
/// view's name column adds to it: a link's target.
fn names() -> Vec<(String, &'static str)> {
    let mut names: Vec<(String, &str)> = Vec::new();
    names.extend((0..DATA_FILES).map(|i| (format!("data_{i:04}.dat"), "")));
    names.extend((0..EMPTY_FILES).map(|i| (format!("empty_{i}.zero"), "")));
    if cfg!(unix) {
        names.push((
            "link_dangling.lnk".into(),
            " -> non_existent_target.missing",
        ));
        names.push(("link_valid.lnk".into(), " -> data_0000.dat"));
    }
    names.push(("sparse_large.bin".into(), ""));
    names.extend((0..SUBFOLDERS).map(|d| (format!("subfolder_{d}"), "")));
    if cfg!(unix) {
        names.push(("test_pipe.fifo".into(), ""));
    }
    assert_eq!(names.len(), TOTAL);
    names
}

/// One name per line, as the lines view prints them.
fn lines() -> String {
    names()
        .iter()
        .map(|(name, _)| format!("{name}\n"))
        .collect()
}

/// One name per line with link targets, as the long view's name column
/// prints them.
fn rows() -> String {
    names()
        .iter()
        .map(|(name, target)| format!("{name}{target}\n"))
        .collect()
}

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
    assert_eq!(lines, self::lines());

    // Laying out a grid of this size is the grid's own tests' business;
    // here every name has to be in it exactly once.
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

    assert_eq!(run(&dir, &NAME_COLUMN_ONLY), rows());

    let quoted: Vec<String> = names()
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect();
    assert_eq!(run(&dir, &["--json"]), format!("[{}]\n", quoted.join(",")));
}

#[test]
fn summary_and_print_total_count_every_entry() {
    let dir = corpus();
    let files = DATA_FILES + EMPTY_FILES + 1 + usize::from(cfg!(unix));
    let symlinks = if cfg!(unix) { 2 } else { 0 };

    assert_eq!(
        run(&dir, &[&NAME_COLUMN_ONLY[..], &["--summary"]].concat()),
        format!(
            "{}{SUBFOLDERS} directories, {files} files, {symlinks} symlinks ({TOTAL} total)\n",
            rows()
        )
    );
    assert_eq!(
        run(&dir, &[&NAME_COLUMN_ONLY[..], &["--print-total"]].concat()),
        format!("{}total: {TOTAL}\n", rows())
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

/// Two thousand links to one file: the long view gives each its target,
/// and `--no-symlink-targets` drops exactly those.
#[test]
#[cfg(unix)]
fn two_thousand_links_to_one_file_are_listed_in_full() {
    let dir = TempTestDir::new("many_links");
    dir.create_file("shared_target.txt", b"common target");
    let links: Vec<String> = (0..2000).map(|i| format!("link_{i:04}.lnk")).collect();
    for link in &links {
        dir.create_symlink("shared_target.txt", link);
    }
    let rows = |extra: &[&str]| run(&dir, &[&NAME_COLUMN_ONLY[..], extra].concat());

    let with_targets: String = links
        .iter()
        .map(|link| format!("{link} -> shared_target.txt\n"))
        .chain(["shared_target.txt\n".to_owned()])
        .collect();
    assert_eq!(rows(&[]), with_targets);

    let names: String = links
        .iter()
        .map(|link| format!("{link}\n"))
        .chain(["shared_target.txt\n".to_owned()])
        .collect();
    assert_eq!(rows(&["--no-symlink-targets"]), names);
}
