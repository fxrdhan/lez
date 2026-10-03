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

/// `zsh -n` parses both files without running them. zsh is the default
/// shell on macOS and CI installs it on Linux, so there it is required;
/// anywhere else without it the check is skipped, saying so. It used to
/// pass without a word wherever `which zsh` found nothing.
#[test]
fn zsh_parses_both_completion_files() {
    let available = Command::new("zsh")
        .args(["-c", "true"])
        .output()
        .is_ok_and(|output| output.status.success());
    if !available {
        let required = cfg!(target_os = "macos")
            || (cfg!(target_os = "linux")
                && std::env::var_os("CI").is_some()
                && std::env::var_os("NIX_BUILD_TOP").is_none());
        assert!(
            !required,
            "zsh is needed here to check the completion files"
        );
        eprintln!("skipped: no zsh on this machine");
        return;
    }
    for path in [get_zsh_completion_path(), get_zsh_compat_completion_path()] {
        let output = Command::new("zsh")
            .arg("-n")
            .arg(&path)
            .output()
            .expect("run zsh -n");
        assert_eq!(output.status.code(), Some(0), "{}", path.display());
        assert!(
            output.stderr.is_empty(),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
