// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! E2E tests for `--short-nix`: abbreviating 32-character Nix base32 store
//! hashes in filenames, parent paths, grid width calculation, coloured output,
//! and symlink targets while leaving non-Nix names untouched.

use crate::common::{TempTestDir, lez_in, success_stdout};

/// Valid 32-char Nix base32 hash (alphabet excludes `e`, `o`, `t`, `u`).
const NIX_HASH: &str = "0123456789abcdfghijklmnpqrsvwxyz";
const NIX_SHORT: &str = "01234567…";

#[test]
fn abbreviates_valid_nix_hashes_and_leaves_non_nix_names_intact() {
    let dir = TempTestDir::new("short_nix_names");
    let valid = format!("{NIX_HASH}-hello-2.12");
    // Invalid Nix base32: contains 'e', 'o', 't', 'u'
    let invalid_alphabet = "0123456789abcdefghijklmnotuvwxyz-fake-1.0";
    // Wrong hash length (31 chars)
    let too_short = format!("{}-short-1.0", &NIX_HASH[..31]);
    // Missing dash separator after 32 chars
    let no_dash = format!("{NIX_HASH}_nodash");

    dir.create_file(&valid, b"ok");
    dir.create_file(invalid_alphabet, b"bad");
    dir.create_file(&too_short, b"bad");
    dir.create_file(&no_dash, b"bad");

    let out = success_stdout(lez_in(dir.path()).args([
        "-1",
        "--short-nix",
        "--color=never",
        "--icons=never",
    ]));
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(&format!("{NIX_SHORT}-hello-2.12").as_str()),
        "valid Nix store entry must be abbreviated:\n{out}"
    );
    assert!(
        lines.contains(&invalid_alphabet),
        "non-Nix-base32 hash must not be abbreviated:\n{out}"
    );
    assert!(
        lines.contains(&too_short.as_str()),
        "31-char hash must not be abbreviated:\n{out}"
    );
    assert!(
        lines.contains(&no_dash.as_str()),
        "entry without dash separator must not be abbreviated:\n{out}"
    );
}

#[test]
fn short_nix_abbreviates_parent_paths_and_grid_widths_and_colours_hash() {
    let dir = TempTestDir::new("short_nix_paths");
    let store_dir = format!("{NIX_HASH}-pkg-1.0");
    let nested = format!("{store_dir}/bin.txt");
    dir.create_file(&nested, b"x");
    dir.create_file(&format!("{NIX_HASH}-alpha-1.0"), b"a");
    dir.create_file(&format!("{NIX_HASH}-beta-2.0"), b"b");

    // Explicit path with Nix hash in parent directory component.
    let explicit = success_stdout(lez_in(dir.path()).args([
        "-1",
        "--short-nix",
        "--color=never",
        "--icons=never",
        &nested,
    ]));
    assert!(
        explicit.contains(&format!("{NIX_SHORT}-pkg-1.0")) && explicit.contains("bin.txt"),
        "parent Nix store segment must be abbreviated:\n{explicit}"
    );

    // Grid layout uses shortened width so both abbreviated names fit on one 60-col row
    // (their full 44-char names would never fit side by side in 60 columns).
    let grid = success_stdout(lez_in(dir.path()).args([
        "-G",
        "--width=60",
        "--short-nix",
        "--color=never",
        "--icons=never",
        &format!("{NIX_HASH}-alpha-1.0"),
        &format!("{NIX_HASH}-beta-2.0"),
    ]));
    assert_eq!(
        grid.lines().count(),
        1,
        "abbreviated names should fit on a single 60-col grid row:\n{grid}"
    );
    assert!(grid.contains(&format!("{NIX_SHORT}-alpha-1.0")));
    assert!(grid.contains(&format!("{NIX_SHORT}-beta-2.0")));

    // Coloured output paints the abbreviated hash prefix distinctly (dimmed).
    let coloured = success_stdout(lez_in(dir.path()).args([
        "-1",
        "--short-nix",
        "--color=always",
        "--icons=never",
        &format!("{NIX_HASH}-alpha-1.0"),
    ]));
    assert!(
        coloured.contains(NIX_SHORT) && coloured.contains("\x1b["),
        "expected ANSI-styled abbreviated hash:\n{coloured:?}"
    );
}

#[cfg(unix)]
#[test]
fn short_nix_abbreviates_symlink_targets_in_long_view() {
    let dir = TempTestDir::new("short_nix_symlink");
    let target_name = format!("{NIX_HASH}-tool-3.4");
    dir.create_file(&target_name, b"bin");
    dir.create_symlink(&target_name, "current");

    let out = success_stdout(lez_in(dir.path()).args([
        "-l",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
        "--no-extended",
        "--short-nix",
        "--color=never",
        "--icons=never",
        "current",
    ]));
    assert!(
        out.contains(&format!("current -> {NIX_SHORT}-tool-3.4")),
        "expected abbreviated symlink target:\n{out}"
    );
}
