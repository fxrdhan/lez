// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-S`/`--blocksize` and `--blocks`: the allocated-size columns, the sort
//! field built on them, and how strict mode treats them outside the long view.
//!
//! Expected values come from the file's own metadata rather than from a
//! hardcoded number, because allocation differs between filesystems (block
//! size, sparse support, compression) while the relation to `st_blocks` and
//! `st_blksize` does not.

use std::fs;
use std::path::Path;
use std::process::Output;

use crate::common::{TempTestDir, grouped, lez_in};

/// Writes `len` bytes that no filesystem can compress away, so the allocation
/// tracks the length.
fn incompressible(len: usize) -> Vec<u8> {
    let mut state: u32 = 0x9E37_79B9;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect()
}

fn run(dir: &Path, args: &[&str]) -> Output {
    lez_in(dir).args(args).output().expect("failed to run lez")
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "lez failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

/// Rows of a long view reduced to the allocation column and the name, with
/// runs of whitespace in the name (tree indentation included) collapsed.
fn allocation_rows(dir: &Path, flags: &[&str]) -> Vec<(String, String)> {
    let mut args = vec![
        "-l",
        "--no-filesize",
        "--no-permissions",
        "--no-user",
        "--no-time",
        "--color=never",
    ];
    args.extend_from_slice(flags);
    stdout(&run(dir, &args))
        .lines()
        .map(|line| {
            let mut fields = line.split_whitespace();
            let value = fields.next().expect("row has an allocation column");
            let name = fields.collect::<Vec<_>>().join(" ");
            (value.to_owned(), name)
        })
        .collect()
}

#[cfg(unix)]
fn allocated_bytes(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).expect("stat fixture").blocks() * 512
}

/// The number of `st_blksize` blocks the allocation spans, which is what
/// `--blocks` documents as "the allocated size of each file, in blocks".
#[cfg(unix)]
fn allocated_blocks(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    let md = fs::metadata(path).expect("stat fixture");
    let block_size = md.blksize();
    if block_size > 0 && block_size != 512 {
        (md.blocks() * 512).div_ceil(block_size)
    } else {
        md.blocks()
    }
}

/// Three files of different sizes, and a directory, a symlink and a FIFO,
/// none of which has an allocation of its own to show.
#[cfg(unix)]
fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("blocks");
    dir.create_file("one_byte.txt", b"a");
    dir.create_file("ten_k.bin", &incompressible(10_000));
    dir.create_file("quarter_mib.bin", &incompressible(256 * 1024));
    dir.create_dir("sub");
    dir.create_symlink("one_byte.txt", "link");
    let fifo = std::ffi::CString::new(dir.path().join("pipe").to_str().expect("UTF-8 path"))
        .expect("no NUL in path");
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0, "mkfifo");
    dir
}

/// The fixture's rows, in name order, with `value` giving each regular
/// file's column.
#[cfg(unix)]
fn fixture_rows(value: impl Fn(&str) -> String) -> Vec<(String, String)> {
    [
        ("link -> one_byte.txt", None),
        ("one_byte.txt", Some("one_byte.txt")),
        ("pipe", None),
        ("quarter_mib.bin", Some("quarter_mib.bin")),
        ("sub", None),
        ("ten_k.bin", Some("ten_k.bin")),
    ]
    .into_iter()
    .map(|(name, file)| (file.map_or_else(|| "-".to_owned(), &value), name.to_owned()))
    .collect()
}

#[test]
#[cfg(unix)]
fn blocks_counts_the_allocation_in_filesystem_blocks() {
    let dir = fixture();
    assert_eq!(
        allocation_rows(dir.path(), &["--blocks"]),
        fixture_rows(|file| grouped(allocated_blocks(&dir.path().join(file))))
    );
}

#[test]
#[cfg(unix)]
fn blocksize_in_bytes_is_the_allocation_not_the_length() {
    let dir = fixture();
    let sparse = dir.path().join("sparse.img");
    fs::File::create(&sparse)
        .and_then(|f| f.set_len(10 * 1024 * 1024))
        .expect("create sparse file");

    let mut expected = fixture_rows(|file| grouped(allocated_bytes(&dir.path().join(file))));
    expected.insert(
        4,
        (grouped(allocated_bytes(&sparse)), "sparse.img".to_owned()),
    );
    assert_eq!(
        allocation_rows(dir.path(), &["--blocksize", "-B"]),
        expected
    );
}

/// With `-X` a symlink stands for its target, allocation included.
#[test]
#[cfg(unix)]
fn a_dereferenced_link_shows_its_targets_allocation() {
    let dir = fixture();
    dir.create_symlink("ten_k.bin", "big_link");

    let alloc = |file: &str| grouped(allocated_bytes(&dir.path().join(file)));
    assert_eq!(
        allocation_rows(dir.path(), &["--blocksize", "-B", "-X"]),
        [
            (alloc("ten_k.bin"), "big_link"),
            (alloc("one_byte.txt"), "link"),
            (alloc("one_byte.txt"), "one_byte.txt"),
            ("-".to_owned(), "pipe"),
            (alloc("quarter_mib.bin"), "quarter_mib.bin"),
            ("-".to_owned(), "sub"),
            (alloc("ten_k.bin"), "ten_k.bin"),
        ]
        .map(|(value, name)| (value, name.to_owned()))
    );
}

/// With `--total-size` a directory reports what the files under it
/// allocate, at every depth, in the tree view too.
#[test]
#[cfg(unix)]
fn with_total_size_a_directory_shows_what_its_contents_allocate() {
    let dir = TempTestDir::new("blocks_total");
    let top = dir.create_file("outer/top.bin", &incompressible(10_000));
    let deep = dir.create_file("outer/inner/deep.bin", &incompressible(5_000));
    let deep_bytes = allocated_bytes(&deep);
    let outer_bytes = allocated_bytes(&top) + deep_bytes;

    assert_eq!(
        allocation_rows(dir.path(), &["--blocksize", "-B", "--total-size"]),
        [(grouped(outer_bytes), "outer".to_owned())]
    );
    assert_eq!(
        allocation_rows(
            dir.path(),
            &["--blocksize", "-B", "--total-size", "-T", "outer"]
        ),
        [
            (grouped(outer_bytes), "outer".to_owned()),
            (grouped(deep_bytes), "├── inner".to_owned()),
            (grouped(deep_bytes), "│ └── deep.bin".to_owned()),
            (grouped(allocated_bytes(&top)), "└── top.bin".to_owned()),
        ]
    );
}

#[test]
#[cfg(unix)]
fn blocksize_honours_the_size_unit_flags() {
    let dir = TempTestDir::new("blocks_units");
    let path = dir.create_file("ten_k.bin", &incompressible(10_000));
    let bytes = allocated_bytes(&path);
    assert_eq!(
        bytes % 1024,
        0,
        "the fixture should occupy whole KiB for the unit checks to be exact"
    );
    let kib = bytes / 1024;

    let binary = allocation_rows(dir.path(), &["--blocksize", "-b"]);
    let raw = allocation_rows(dir.path(), &["--blocksize", "-B"]);
    assert_eq!(binary, [(format!("{kib}Ki"), "ten_k.bin".to_owned())]);
    assert_eq!(raw, [(grouped(bytes), "ten_k.bin".to_owned())]);

    // The decimal rounding rules are unit tested with the renderer; here it is
    // enough that the default goes through the same formatter as the size
    // column, so a file whose length equals the allocation must read the same.
    dir.create_file(
        "twin.bin",
        &incompressible(usize::try_from(bytes).expect("fits")),
    );
    let out = stdout(&run(
        dir.path(),
        &[
            "-l",
            "--blocksize",
            "--no-permissions",
            "--no-user",
            "--no-time",
        ],
    ));
    let column = |name: &str, index: usize| -> String {
        out.lines()
            .find(|line| line.ends_with(name))
            .and_then(|line| line.split_whitespace().nth(index))
            .unwrap_or_else(|| panic!("no row for {name}:\n{out}"))
            .to_owned()
    };
    let decimal = column("ten_k.bin", 1);
    assert!(decimal.ends_with('k'), "decimal prefix expected: {decimal}");
    assert_eq!(decimal, column("twin.bin", 0));
}

#[test]
#[cfg(unix)]
fn json_carries_the_selected_allocation_column() {
    let dir = TempTestDir::new("blocks_json");
    let path = dir.create_file("ten_k.bin", &incompressible(10_000));

    let json = |flags: &[&str]| -> serde_json::Value {
        let mut args = vec![
            "--json",
            "-l",
            "--no-permissions",
            "--no-filesize",
            "--no-user",
            "--no-time",
        ];
        args.extend_from_slice(flags);
        serde_json::from_str(&stdout(&run(dir.path(), &args))).expect("valid JSON")
    };

    assert_eq!(
        json(&["--blocks"]),
        serde_json::json!({"ten_k.bin": {"Blocks": grouped(allocated_blocks(&path))}})
    );
    assert_eq!(
        json(&["--blocksize", "-B"]),
        serde_json::json!({"ten_k.bin": {"Blocksize": grouped(allocated_bytes(&path))}})
    );
}

#[test]
#[cfg(unix)]
fn short_s_is_the_same_flag_as_blocksize() {
    let dir = fixture();
    for extra in [&[][..], &["-T"][..]] {
        let mut short = vec!["-l", "-S", "--time-style=iso"];
        let mut long = vec!["-l", "--blocksize", "--time-style=iso"];
        short.extend_from_slice(extra);
        long.extend_from_slice(extra);
        assert_eq!(
            stdout(&run(dir.path(), &short)),
            stdout(&run(dir.path(), &long)),
            "with {extra:?}"
        );
    }
}

#[test]
#[cfg(unix)]
fn the_header_names_the_selected_column_and_the_last_flag_wins() {
    let dir = TempTestDir::new("blocks_header");
    dir.create_file("file.txt", b"x");

    let header = |flags: &[&str]| -> Vec<String> {
        let mut args = vec![
            "-lh",
            "--no-filesize",
            "--no-permissions",
            "--no-user",
            "--no-time",
        ];
        args.extend_from_slice(flags);
        let out = stdout(&run(dir.path(), &args));
        out.lines()
            .next()
            .expect("header row")
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    };

    assert_eq!(header(&["--blocks"]), ["Blocks", "Name"]);
    assert_eq!(header(&["--blocksize"]), ["Blocksize", "Name"]);
    assert_eq!(header(&["--blocksize", "--blocks"]), ["Blocks", "Name"]);
    assert_eq!(header(&["--blocks", "--blocksize"]), ["Blocksize", "Name"]);
    assert_eq!(header(&["--blocks", "-S"]), ["Blocksize", "Name"]);
}

#[test]
#[cfg(unix)]
fn sorting_by_blocks_uses_the_allocation_not_the_length() {
    let dir = TempTestDir::new("blocks_sort");
    dir.create_file("small_dense.bin", &incompressible(64 * 1024));
    dir.create_file("medium_dense.bin", &incompressible(256 * 1024));
    let sparse = dir.path().join("huge_sparse.img");
    fs::File::create(&sparse)
        .and_then(|f| f.set_len(64 * 1024 * 1024))
        .expect("create sparse file");
    assert!(
        allocated_bytes(&sparse) < allocated_bytes(&dir.path().join("small_dense.bin")),
        "the temporary filesystem should store the sparse file sparsely"
    );

    let names = |args: &[&str]| -> Vec<String> {
        stdout(&run(dir.path(), args))
            .lines()
            .map(str::to_owned)
            .collect()
    };

    assert_eq!(
        names(&["-1", "--sort=size"]),
        ["small_dense.bin", "medium_dense.bin", "huge_sparse.img"]
    );
    for field in ["block", "blocks", "blocksize"] {
        let sort = format!("--sort={field}");
        assert_eq!(
            names(&["-1", &sort]),
            ["huge_sparse.img", "small_dense.bin", "medium_dense.bin"],
            "{sort}"
        );
        assert_eq!(
            names(&["-1", &sort, "-r"]),
            ["medium_dense.bin", "small_dense.bin", "huge_sparse.img"],
            "{sort} -r"
        );
    }
}

#[test]
fn strict_mode_rejects_block_columns_outside_the_long_view() {
    let dir = TempTestDir::new("blocks_strict");
    dir.create_file("file.txt", b"data");

    for (flag, option) in [
        ("-S", "blocksize"),
        ("--blocksize", "blocksize"),
        ("--blocks", "blocks"),
    ] {
        for view in [&[][..], &["-1"][..], &["-G"][..], &["-T"][..]] {
            let mut args = view.to_vec();
            args.push(flag);
            let output = lez_in(dir.path())
                .env("LEZ_STRICT", "1")
                .args(&args)
                .output()
                .expect("failed to run lez");
            assert_eq!(output.status.code(), Some(3), "{args:?}");
            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                format!("lez: Option {option} is useless without option long\n"),
                "{args:?}"
            );
            assert!(output.stdout.is_empty(), "{args:?}");
        }
    }
}

#[test]
fn strict_mode_accepts_block_columns_in_the_long_view() {
    let dir = TempTestDir::new("blocks_strict_long");
    dir.create_file("file.txt", b"data");

    for flag in ["-S", "--blocksize", "--blocks"] {
        for long in ["-l", "-lT"] {
            let output = lez_in(dir.path())
                .env("LEZ_STRICT", "1")
                .args([long, flag])
                .output()
                .expect("failed to run lez");
            assert_eq!(
                output.status.code(),
                Some(0),
                "{long} {flag}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty(), "{long} {flag}");
        }
    }
}

#[test]
fn outside_strict_mode_block_flags_without_long_change_nothing() {
    let dir = TempTestDir::new("blocks_lenient");
    dir.create_file("file.txt", b"data");
    dir.create_dir("folder");

    for view in [&["-1"][..], &["-G"][..], &["-T"][..]] {
        let baseline = stdout(&run(dir.path(), view));
        for flag in ["-S", "--blocksize", "--blocks"] {
            let mut args = view.to_vec();
            args.push(flag);
            let output = run(dir.path(), &args);
            assert!(output.stderr.is_empty(), "{args:?}");
            assert_eq!(stdout(&output), baseline, "{args:?}");
        }
    }
}
