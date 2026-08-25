// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDirSetup {
    path: PathBuf,
}

impl TempDirSetup {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_path_sort_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, rel: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for TempDirSetup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lez_in<P: AsRef<Path>>(working_dir: P, args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lez");
    Command::new(bin_path)
        .current_dir(working_dir)
        // Pin collation to the POSIX C locale so expectations rely on plain
        // byte order instead of the OS locale. Without this, macOS/Windows
        // fall back to sys_locale and ICU tertiary strength reorders mixed-
        // case names by base letter (e.g. "whiskey" before "Yankee"), which
        // made assertions locale-dependent and fail outside Linux CI.
        .env("LC_ALL", "C")
        .args(args)
        .output()
        .expect("Failed to execute lez binary")
}

// ----------------------------------------------------------------------------
// F5: Path & Relative-Path Sorting Tests (#1835)
// ----------------------------------------------------------------------------

#[test]
fn test_f5_sort_path_and_aliases_accepted() {
    let temp = TempDirSetup::new("path_accepted");
    temp.create_file("dir_b/item.txt", b"b");
    temp.create_file("dir_a/item.txt", b"a");

    let aliases = [
        "path",
        "relative-path",
        "relpath",
        "relative_path",
        "Path",
        "Relative-path",
        "Relative-Path",
        "Relpath",
        "Relative_path",
    ];

    for alias in aliases {
        let sort_arg = format!("--sort={alias}");
        let output = run_lez_in(
            &temp.path,
            &["-1", "--tree", &sort_arg, "--color=never", "."],
        );
        assert!(
            output.status.success(),
            "Flag --sort={alias} must be accepted: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output_short = run_lez_in(
            &temp.path,
            &["-1", "--tree", "-s", alias, "--color=never", "."],
        );
        assert!(
            output_short.status.success(),
            "Short flag -s {alias} must be accepted: {}",
            String::from_utf8_lossy(&output_short.stderr)
        );
    }
}

#[test]
fn test_f5_sort_path_ordering_files() {
    let temp = TempDirSetup::new("path_order");
    let fb = temp.create_file("dir_b/z.txt", b"bz");
    let fa1 = temp.create_file("dir_a/a.txt", b"aa");
    let fa2 = temp.create_file("dir_a/sub/nested.txt", b"sub");

    let fb_str = fb.to_str().unwrap();
    let fa1_str = fa1.to_str().unwrap();
    let fa2_str = fa2.to_str().unwrap();

    let output_path = run_lez_in(
        &temp.path,
        &[
            "-1d",
            "--sort=path",
            "--color=never",
            fb_str,
            fa1_str,
            fa2_str,
        ],
    );
    assert!(output_path.status.success());
    let stdout_path = String::from_utf8_lossy(&output_path.stdout).replace('\\', "/");
    let lines_path: Vec<&str> = stdout_path.lines().collect();

    assert_eq!(lines_path.len(), 3);
    assert!(
        lines_path[0].contains("dir_a/a.txt"),
        "dir_a/a.txt first: {:?}",
        lines_path
    );
    assert!(
        lines_path[1].contains("dir_a/sub/nested.txt"),
        "dir_a/sub/nested.txt second: {:?}",
        lines_path
    );
    assert!(
        lines_path[2].contains("dir_b/z.txt"),
        "dir_b/z.txt third: {:?}",
        lines_path
    );

    // Verify all aliases produce the exact same order
    for alias in ["relative-path", "relpath", "relative_path"] {
        let sort_arg = format!("--sort={alias}");
        let output_alias = run_lez_in(
            &temp.path,
            &["-1d", &sort_arg, "--color=never", fb_str, fa1_str, fa2_str],
        );
        assert!(output_alias.status.success());
        let stdout_alias = String::from_utf8_lossy(&output_alias.stdout).replace('\\', "/");
        let lines_alias: Vec<&str> = stdout_alias.lines().collect();
        assert_eq!(
            lines_alias, lines_path,
            "Alias {alias} output must match --sort=path"
        );
    }
}

#[test]
fn test_f5_sort_path_reverse() {
    let temp = TempDirSetup::new("path_rev");
    let fb = temp.create_file("dir_b/z.txt", b"bz");
    let fa = temp.create_file("dir_a/a.txt", b"aa");

    let fa_str = fa.to_str().unwrap();
    let fb_str = fb.to_str().unwrap();

    let output = run_lez_in(
        &temp.path,
        &["-1d", "--sort=path", "-r", "--color=never", fa_str, fb_str],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).replace('\\', "/");
    let lines: Vec<&str> = stdout.lines().collect();

    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].contains("dir_b/z.txt"),
        "dir_b/z.txt first in reverse: {:?}",
        lines
    );
    assert!(
        lines[1].contains("dir_a/a.txt"),
        "dir_a/a.txt second in reverse: {:?}",
        lines
    );

    // Verify with --sort=relative-path -r
    let output_rel = run_lez_in(
        &temp.path,
        &[
            "-1d",
            "--sort=relative-path",
            "-r",
            "--color=never",
            fa_str,
            fb_str,
        ],
    );
    assert!(output_rel.status.success());
    let stdout_rel = String::from_utf8_lossy(&output_rel.stdout).replace('\\', "/");
    let lines_rel: Vec<&str> = stdout_rel.lines().collect();
    assert_eq!(lines_rel, lines);
}

#[test]
fn test_f5_distinguish_leaf_name_sorting_from_path_sorting() {
    let temp = TempDirSetup::new("distinguish_leaf_vs_path");
    // dir_a/zeta.txt vs dir_b/alpha.txt
    // By filename (basename): "alpha.txt" < "zeta.txt", so dir_b/alpha.txt comes FIRST.
    // By path: "dir_a/zeta.txt" < "dir_b/alpha.txt", so dir_a/zeta.txt comes FIRST.
    let fa_z = temp.create_file("dir_a/zeta.txt", b"a_zeta");
    let fb_a = temp.create_file("dir_b/alpha.txt", b"b_alpha");

    let fa_z_str = fa_z.to_str().unwrap();
    let fb_a_str = fb_a.to_str().unwrap();

    // 1. Sort by name (basename)
    let output_name = run_lez_in(
        &temp.path,
        &["-1d", "--sort=name", "--color=never", fa_z_str, fb_a_str],
    );
    assert!(output_name.status.success());
    let stdout_name = String::from_utf8_lossy(&output_name.stdout).replace('\\', "/");
    let lines_name: Vec<&str> = stdout_name.lines().collect();
    assert_eq!(lines_name.len(), 2);
    assert!(
        lines_name[0].contains("dir_b/alpha.txt"),
        "Under --sort=name, alpha.txt comes before zeta.txt: {:?}",
        lines_name
    );
    assert!(
        lines_name[1].contains("dir_a/zeta.txt"),
        "Under --sort=name, zeta.txt comes second: {:?}",
        lines_name
    );

    // 2. Sort by relative-path
    let output_relpath = run_lez_in(
        &temp.path,
        &[
            "-1d",
            "--sort=relative-path",
            "--color=never",
            fa_z_str,
            fb_a_str,
        ],
    );
    assert!(output_relpath.status.success());
    let stdout_relpath = String::from_utf8_lossy(&output_relpath.stdout).replace('\\', "/");
    let lines_relpath: Vec<&str> = stdout_relpath.lines().collect();
    assert_eq!(lines_relpath.len(), 2);
    assert!(
        lines_relpath[0].contains("dir_a/zeta.txt"),
        "Under --sort=relative-path, dir_a comes before dir_b: {:?}",
        lines_relpath
    );
    assert!(
        lines_relpath[1].contains("dir_b/alpha.txt"),
        "Under --sort=relative-path, dir_b comes second: {:?}",
        lines_relpath
    );

    // 3. Sort by path (alias)
    let output_path = run_lez_in(
        &temp.path,
        &["-1d", "--sort=path", "--color=never", fa_z_str, fb_a_str],
    );
    assert!(output_path.status.success());
    let stdout_path = String::from_utf8_lossy(&output_path.stdout).replace('\\', "/");
    let lines_path: Vec<&str> = stdout_path.lines().collect();
    assert_eq!(lines_path, lines_relpath);
}

#[test]
fn test_f5_sort_path_case_sensitive_variants() {
    // Fixture note: the directory names deliberately do NOT differ only by
    // case. On case-insensitive filesystems (macOS APFS, Windows NTFS by
    // default) siblings like "Dir_A" and "dir_a" would collapse into a single
    // directory. Uppercase-initial vs lowercase-initial names still prove the
    // ABCabc semantics: byte order puts 'Y' and 'Z' before 'w' and 'x'.
    let temp = TempDirSetup::new("path_case");
    let f_zulu = temp.create_file("Zulu/file.txt", b"z");
    let f_whiskey = temp.create_file("whiskey/file.txt", b"w");
    let f_yankee = temp.create_file("Yankee/file.txt", b"y");
    let f_xray = temp.create_file("xray/file.txt", b"x");

    let fz_str = f_zulu.to_str().unwrap();
    let fw_str = f_whiskey.to_str().unwrap();
    let fy_str = f_yankee.to_str().unwrap();
    let fx_str = f_xray.to_str().unwrap();

    // Under case-sensitive sort (ABCabc): uppercase-initial directories
    // (Yankee, Zulu) come before lowercase-initial ones (whiskey, xray).
    let output_case = run_lez_in(
        &temp.path,
        &[
            "-1d",
            "--sort=Path",
            "--color=never",
            fx_str,
            fz_str,
            fw_str,
            fy_str,
        ],
    );
    assert!(output_case.status.success());
    let stdout_case = String::from_utf8_lossy(&output_case.stdout).replace('\\', "/");
    let lines_case: Vec<&str> = stdout_case.lines().collect();

    assert_eq!(lines_case.len(), 4);
    assert!(lines_case[0].contains("Yankee/file.txt"));
    assert!(lines_case[1].contains("Zulu/file.txt"));
    assert!(lines_case[2].contains("whiskey/file.txt"));
    assert!(lines_case[3].contains("xray/file.txt"));

    // Test case-sensitive aliases: Relative-path, Relative-Path, Relpath, Relative_path
    for case_alias in ["Relative-path", "Relative-Path", "Relpath", "Relative_path"] {
        let sort_arg = format!("--sort={case_alias}");
        let output_alias = run_lez_in(
            &temp.path,
            &[
                "-1d",
                &sort_arg,
                "--color=never",
                fx_str,
                fz_str,
                fw_str,
                fy_str,
            ],
        );
        assert!(output_alias.status.success());
        let stdout_alias = String::from_utf8_lossy(&output_alias.stdout).replace('\\', "/");
        let lines_alias: Vec<&str> = stdout_alias.lines().collect();
        assert_eq!(
            lines_alias, lines_case,
            "Case alias {case_alias} must match --sort=Path"
        );
    }
}
