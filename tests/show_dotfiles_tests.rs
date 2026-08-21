// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};
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
            "lsr_show_dotfiles_test_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp test directory");
        Self { path }
    }

    fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
        let file_path = self.path.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_lsr(args: &[&str]) -> Output {
    let bin_path = env!("CARGO_BIN_EXE_lsr");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("Failed to execute lsr binary")
}

fn listed_names(args: &[&str]) -> Vec<String> {
    let output = run_lsr(args);
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn test_show_dotfiles_lists_dot_prefixed_entries_by_default_hidden() {
    let temp = TempTestDir::new("basic");
    temp.create_file(".dotfile", b"dot");
    temp.create_file("regular.txt", b"regular");

    let dir_arg = temp.path.to_str().unwrap();

    // Default: dot-prefixed entries are hidden.
    let names = listed_names(&["-1", "--color=never", dir_arg]);
    assert_eq!(names, vec!["regular.txt".to_string()]);

    // --show-dotfiles reveals them without needing --all.
    let names = listed_names(&["-1", "--color=never", "--show-dotfiles", dir_arg]);
    assert_eq!(
        names,
        vec![".dotfile".to_string(), "regular.txt".to_string()]
    );
}

#[test]
fn test_show_dotfiles_does_not_reveal_dot_directories() {
    let temp = TempTestDir::new("no_dots");
    temp.create_file(".dotfile", b"dot");
    temp.create_file("regular.txt", b"regular");

    let dir_arg = temp.path.to_str().unwrap();

    // Unlike a double --all, --show-dotfiles never lists '.' and '..'.
    let names = listed_names(&["-1", "--color=never", "--show-dotfiles", dir_arg]);
    assert!(!names.iter().any(|n| n == "." || n == ".."));

    // A double --all still shows them.
    let names = listed_names(&["-1", "--color=never", "-aa", dir_arg]);
    assert!(names.iter().any(|n| n == "."));
    assert!(names.iter().any(|n| n == ".."));
}

#[test]
fn test_almost_all_takes_precedence_over_show_dotfiles() {
    let temp = TempTestDir::new("precedence");
    temp.create_file(".dotfile", b"dot");
    temp.create_file("regular.txt", b"regular");

    let dir_arg = temp.path.to_str().unwrap();

    // --almost-all binds stronger: dotfiles shown, '.'/'..' still hidden,
    // identical to plain --almost-all output.
    let combined = listed_names(&["-1", "--color=never", "--show-dotfiles", "-A", dir_arg]);
    let almost_all = listed_names(&["-1", "--color=never", "-A", dir_arg]);
    assert_eq!(combined, almost_all);
    assert!(combined.contains(&".dotfile".to_string()));
}
