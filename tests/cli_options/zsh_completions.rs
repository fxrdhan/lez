// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The completion `lez` itself ships and that `#compdef lez` binds.
fn get_zsh_completion_path() -> PathBuf {
    get_repo_root().join("completions").join("zsh").join("_lez")
}

/// The compatibility copy installed for anyone still invoking the binary as
/// `eza`. It is generated from the primary file, so it has to stay in step.
fn get_zsh_compat_completion_path() -> PathBuf {
    get_repo_root().join("completions").join("zsh").join("_eza")
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lez")
}

#[test]
fn test_zsh_completion_file_exists() {
    let path = get_zsh_completion_path();
    assert!(
        path.exists(),
        "Zsh completion file must exist at completions/zsh/_lez"
    );

    let compat = get_zsh_compat_completion_path();
    assert!(
        compat.exists(),
        "Zsh compatibility completion must exist at completions/zsh/_eza"
    );
}

/// Two files installed into the same site-functions directory must not both
/// claim `eza`, or which one zsh loads comes down to order.
#[test]
fn test_zsh_completions_claim_distinct_commands() {
    let primary = fs::read_to_string(get_zsh_completion_path()).expect("_lez should be readable");
    let compat =
        fs::read_to_string(get_zsh_compat_completion_path()).expect("_eza should be readable");

    assert_eq!(primary.lines().next(), Some("#compdef lez"));
    assert_eq!(compat.lines().next(), Some("#compdef eza"));
}

#[test]
fn test_zsh_completion_f_flag_separate_from_classify() {
    let path = get_zsh_completion_path();
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {}", path.display(), err));

    // Ensure the old combined form {-F,--classify} is no longer present
    assert!(
        !content.contains("{-F,--classify}"),
        "Zsh completion must not group -F and --classify together in {{-F,--classify}}"
    );

    // Verify separate -F flag definition
    let has_separate_f = content.lines().any(|line| {
        let trimmed = line.trim().strip_prefix("\\*").unwrap_or(line.trim());
        trimmed.starts_with("-F\"[Display type indicator by file names")
            && !trimmed.contains(":(when):")
    });
    assert!(
        has_separate_f,
        "Zsh completion must define -F without requiring an argument parameter"
    );
}

#[test]
fn test_zsh_completion_classify_with_equals_and_when_values() {
    let path = get_zsh_completion_path();
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {}", path.display(), err));

    // Verify --classify= option with equals and allowed values
    let has_classify_equals = content.lines().any(|line| {
        let trimmed = line.trim().strip_prefix("\\*").unwrap_or(line.trim());
        trimmed.starts_with("--classify=\"[Display type indicator by file names]")
            && trimmed.contains(":(when):(always auto automatic never)")
    });
    assert!(
        has_classify_equals,
        "Zsh completion must define --classify= with equal sign and optional when values"
    );
}

#[test]
fn test_zsh_completion_syntax_check() {
    let path = get_zsh_completion_path();
    let which_zsh = Command::new("which").arg("zsh").output();

    if let Ok(which_out) = which_zsh
        && which_out.status.success()
    {
        let output = Command::new("zsh")
            .arg("-n")
            .arg(&path)
            .output()
            .expect("Failed to execute zsh syntax check");

        assert!(
            output.status.success(),
            "zsh -n syntax check failed for {}:\nstdout: {}\nstderr: {}",
            path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_cli_f_flag_and_classify_option_parity() {
    // Verify that the CLI itself accepts -F with no args
    let output_short = Command::new(bin_path())
        .arg("-F")
        .output()
        .expect("Failed to run lez -F");
    assert!(output_short.status.success());

    // Verify that the CLI accepts --classify=always
    let output_long = Command::new(bin_path())
        .arg("--classify=always")
        .output()
        .expect("Failed to run lez --classify=always");
    assert!(output_long.status.success());

    // Verify that the CLI accepts --classify=never
    let output_never = Command::new(bin_path())
        .arg("--classify=never")
        .output()
        .expect("Failed to run lez --classify=never");
    assert!(output_never.status.success());
}
