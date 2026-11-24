// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--completions=SHELL` prints the completion script the repository ships
//! for that shell, so a binary installed on its own (mise, aqua, a bare
//! download) can still load its completions: `lez --completions=fish | source`.

use std::fs;
use std::path::PathBuf;

use crate::common::{TempTestDir, lez_cmd};

/// Every name `--completions` takes, with the file it must print.
const SHELLS: [(&str, &str); 7] = [
    ("bash", "bash/lez"),
    ("zsh", "zsh/_lez"),
    ("fish", "fish/lez.fish"),
    ("nushell", "nush/lez.nu"),
    ("nu", "nush/lez.nu"),
    ("powershell", "pwsh/_lez.ps1"),
    ("pwsh", "pwsh/_lez.ps1"),
];

fn shipped(file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("completions")
        .join(file);
    fs::read(&path).unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()))
}

#[test]
fn each_shell_gets_the_script_the_repository_ships() {
    for (shell, file) in SHELLS {
        for argv in [
            vec![format!("--completions={shell}")],
            vec!["--completions".to_owned(), shell.to_owned()],
        ] {
            let output = lez_cmd().args(&argv).output().expect("run lez");
            assert_eq!(output.status.code(), Some(0), "{argv:?}");
            assert!(output.stderr.is_empty(), "{argv:?}: {:?}", output.stderr);
            assert!(
                output.stdout == shipped(file),
                "{argv:?} did not print completions/{file} byte for byte"
            );
        }
    }
}

/// Like `--version`, it answers before any configuration is read: a listing
/// with a broken `--config` reports the file on stderr, and this says nothing.
#[test]
fn it_answers_before_reading_the_configuration() {
    let dir = TempTestDir::new("completions-config");
    let config = dir.create_file("broken.toml", b"this is [not toml\n");

    let listing = lez_cmd()
        .arg(format!("--config={}", config.display()))
        .arg(dir.path())
        .output()
        .expect("run lez");
    assert!(
        String::from_utf8_lossy(&listing.stderr).contains("Failed to parse config file"),
        "a listing should report the broken file, or this test proves nothing"
    );

    let output = lez_cmd()
        .arg(format!("--config={}", config.display()))
        .arg("--completions=zsh")
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(0));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout == shipped("zsh/_lez"));
}

#[test]
fn paths_given_beside_it_are_not_listed() {
    let output = lez_cmd()
        .args(["--completions=bash", "no-such-path-for-lez"])
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout == shipped("bash/lez"));
}

#[test]
fn an_unknown_shell_is_an_options_error_naming_the_shells() {
    let output = lez_cmd()
        .arg("--completions=tcsh")
        .output()
        .expect("run lez");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("bash, zsh, fish, nushell, powershell"),
        "stderr should list the shells: {stderr}"
    );
}
