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
        let path =
            std::env::temp_dir().join(format!("lez_ada_{prefix}_{}_{}", std::process::id(), nanos));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(if cfg!(windows) { "lez.exe" } else { "lez" })
}

#[test]
fn test_ada_icons_cli() {
    let temp = TempTestDir::new("icons");
    temp.create_file("main.adb", b"procedure Main is begin null; end Main;\n");
    temp.create_file("spec.ads", b"package Spec is end Spec;\n");
    temp.create_file("legacy.ada", b"procedure Legacy is begin null; end;\n");
    temp.create_file("project.gpr", b"project Project is end Project;\n");

    let output = Command::new(bin_path())
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez with icons");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let ada_glyph = '\u{e6b5}'.to_string();

    assert!(stdout.contains(&ada_glyph), "Output missing Ada icon");
    assert!(stdout.contains("main.adb"));
    assert!(stdout.contains("spec.ads"));
    assert!(stdout.contains("legacy.ada"));
    assert!(stdout.contains("project.gpr"));
}

#[test]
fn test_ada_code_summary_cli() {
    let temp = TempTestDir::new("code");
    temp.create_file(
        "main.adb",
        b"-- Body implementation\nwith Ada.Text_IO;\n\nprocedure Main is\nbegin\n    Ada.Text_IO.Put_Line(\"Hello\");\nend Main;\n",
    );
    temp.create_file(
        "spec.ads",
        b"-- Package specification\npackage Spec is\n    function Get_Val return Integer;\nend Spec;\n",
    );
    temp.create_file(
        "build.gpr",
        b"-- GNAT Project\nproject Build is\n    for Source_Dirs use (\"src\");\nend Build;\n",
    );

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez --code");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Ada"),
        "Output missing Ada language in --code summary: {}",
        stdout
    );
    // Exact column counts for 3 Ada files (15 lines, 11 code, 3 comments, 1 blank):
    let ada_row = stdout
        .lines()
        .find(|line| line.contains("Ada"))
        .expect("Ada row must exist in summary table");

    assert!(
        ada_row.contains('3'),
        "Files column should show 3: {ada_row}"
    );
    assert!(
        ada_row.contains("15"),
        "Lines column should show 15: {ada_row}"
    );
    assert!(
        ada_row.contains("11"),
        "Code column should show 11: {ada_row}"
    );
    assert!(
        ada_row.contains('3'),
        "Comments column should show 3: {ada_row}"
    );
    assert!(
        ada_row.contains('1'),
        "Blanks column should show 1: {ada_row}"
    );
    assert!(
        ada_row.contains("100.0%"),
        "Code % should show 100.0%: {ada_row}"
    );
}

#[test]
fn test_ada_code_summary_with_icons_cli() {
    let temp = TempTestDir::new("code_icons");
    temp.create_file(
        "example.adb",
        b"procedure Example is\nbegin\n    null;\nend Example;\n",
    );

    let output = Command::new(bin_path())
        .arg("--code")
        .arg("--icons=always")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez --code --icons=always");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let ada_glyph = '\u{e6b5}'.to_string();
    assert!(
        stdout.contains(&ada_glyph),
        "Output missing Ada icon in --code --icons summary: {}",
        stdout
    );
    assert!(stdout.contains("Ada"));
}

#[test]
fn test_ada_comment_counting_edge_cases() {
    let temp = TempTestDir::new("edge_cases");
    // String with comment marker, trailing comment, empty lines
    let content = b"-- First comment line\n-- Second comment line\nwith Ada.Text_IO;\n\nprocedure Edge is\n   S : String := \"-- not a comment\";\nbegin\n   Ada.Text_IO.Put_Line (S); -- trailing comment\nend Edge;\n";
    temp.create_file("edge.adb", content);

    let output = Command::new(bin_path())
        .arg("--code")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lez --code");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Ada"));
    let ada_row = stdout
        .lines()
        .find(|line| line.contains("Ada"))
        .expect("Ada row must exist in summary table");

    // 1 file, 9 lines, 6 code, 2 comments, 1 blank, 100.0% share
    assert!(
        ada_row.contains('1'),
        "Files column should show 1: {ada_row}"
    );
    assert!(
        ada_row.contains('9'),
        "Lines column should show 9: {ada_row}"
    );
    assert!(
        ada_row.contains('6'),
        "Code column should show 6: {ada_row}"
    );
    assert!(
        ada_row.contains('2'),
        "Comments column should show 2: {ada_row}"
    );
    assert!(
        ada_row.contains('1'),
        "Blanks column should show 1: {ada_row}"
    );
    assert!(
        ada_row.contains("100.0%"),
        "Code % should show 100.0%: {ada_row}"
    );
}
