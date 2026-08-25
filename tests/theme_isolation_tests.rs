// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_test_theme_iso_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_builtin_indicators_in_ls_colors_not_applied_to_filenames() {
    let temp = TempTestDir::new("indicators_ls_colors");
    temp.create_file("su", b"binary");
    temp.create_file("ca", b"certificate");
    temp.create_file("do", b"script");
    temp.create_file("tw", b"sticky");
    temp.create_file("ow", b"other_writable");
    temp.create_file("st", b"sticky_dir");
    temp.create_file("mi", b"missing");
    temp.create_file("rs", b"reset");
    temp.create_file("no", b"normal");
    temp.create_file("mh", b"multihardlink");
    temp.create_file("sg", b"setgid");
    temp.create_file("file.txt", b"plain text");
    temp.create_file("code.rs", b"fn main() {}");

    // Define standard LS_COLORS with distinctive indicator color styles
    // su: 37;41 (white on red bg)
    // ca: 30;41 (black on red bg)
    // do: 01;35 (bold magenta)
    // tw: 30;42 (black on green bg)
    // ow: 34;43 (blue on yellow bg)
    // st: 37;44 (white on blue bg)
    let ls_colors = "su=37;41:ca=30;41:do=01;35:tw=30;42:ow=34;43:st=37;44:mi=05;37;41:rs=0:no=0:mh=00:sg=30;43:*.txt=31:*.rs=32";

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--color=always")
        .arg(&temp.path)
        .env("LS_COLORS", ls_colors)
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // *.txt should be red (31) and *.rs should be green (32)
    assert!(
        stdout.contains("\x1b[31m") || stdout.contains("\x1b[0;31m"),
        "stdout should contain red color for *.txt, got: {stdout:?}"
    );
    assert!(
        stdout.contains("\x1b[32m") || stdout.contains("\x1b[0;32m"),
        "stdout should contain green color for *.rs, got: {stdout:?}"
    );

    // Indicator styles should NOT leak onto files named after the indicators
    assert!(
        !stdout.contains("\x1b[37;41m") && !stdout.contains("\x1b[41;37m"),
        "stdout should not contain su indicator style (37;41), got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[30;41m") && !stdout.contains("\x1b[41;30m"),
        "stdout should not contain ca indicator style (30;41), got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[1;35m") && !stdout.contains("\x1b[01;35m"),
        "stdout should not contain do indicator style (01;35), got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[30;42m") && !stdout.contains("\x1b[42;30m"),
        "stdout should not contain tw indicator style (30;42), got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[34;43m") && !stdout.contains("\x1b[43;34m"),
        "stdout should not contain ow indicator style (34;43), got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[37;44m") && !stdout.contains("\x1b[44;37m"),
        "stdout should not contain st indicator style (37;44), got: {stdout:?}"
    );
}

#[test]
fn test_extension_globs_work_properly_alongside_ls_colors_indicators() {
    let temp = TempTestDir::new("globs_alongside_indicators");
    temp.create_file("document.txt", b"text file");
    temp.create_file("script.py", b"python script");
    temp.create_file("archive.zip", b"zip archive");
    temp.create_file("ca", b"ca cert");
    temp.create_file("su", b"su binary");

    let ls_colors = "di=34:su=37;41:ca=30;41:*.txt=31:*.py=32:*.zip=33";

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--color=always")
        .arg(&temp.path)
        .env("LS_COLORS", ls_colors)
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check glob extensions
    assert!(
        stdout.contains("\x1b[31m") || stdout.contains("\x1b[0;31m"),
        "stdout should format document.txt with red (31), got: {stdout:?}"
    );
    assert!(
        stdout.contains("\x1b[32m") || stdout.contains("\x1b[0;32m"),
        "stdout should format script.py with green (32), got: {stdout:?}"
    );
    assert!(
        stdout.contains("\x1b[33m") || stdout.contains("\x1b[0;33m"),
        "stdout should format archive.zip with yellow (33), got: {stdout:?}"
    );

    // Indicator styles should not appear
    assert!(
        !stdout.contains("\x1b[37;41m") && !stdout.contains("\x1b[41;37m"),
        "stdout should not format su with indicator style, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[30;41m") && !stdout.contains("\x1b[41;30m"),
        "stdout should not format ca with indicator style, got: {stdout:?}"
    );
}

#[test]
fn test_non_builtin_keys_in_ls_colors_still_treated_as_globs() {
    let temp = TempTestDir::new("non_builtin_ls_colors");
    temp.create_file("sf", b"sf file");
    temp.create_file("uu", b"uu file");

    // sf and uu are not among the 23 standard GNU dircolors keys
    let ls_colors = "sf=38;5;121:uu=38;5;117";

    let output = Command::new(bin_path())
        .arg("-1")
        .arg("--color=always")
        .arg(&temp.path)
        .env("LS_COLORS", ls_colors)
        .env_remove("EZA_COLORS")
        .env_remove("EXA_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute lez");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("\x1b[38;5;121m"),
        "stdout should format non-builtin key sf as glob with 38;5;121, got: {stdout:?}"
    );
    assert!(
        stdout.contains("\x1b[38;5;117m"),
        "stdout should format non-builtin key uu as glob with 38;5;117, got: {stdout:?}"
    );
}
