// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Guards for the shell completions around flags that require an equals sign.
//!
//! Nine flags take an optional value and only accept it attached with `=`.
//! A completion that offers the value after a space builds a command line the
//! parser reads differently from what the user meant: the value lands as a
//! path and the flag falls back to its default. The set is read from the clap
//! command here, so adding `require_equals` to a tenth flag fails these tests
//! until every backend is taught about it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `(directory, primary, compatibility copy)` for every backend we ship.
const BACKENDS: [(&str, &str, &str); 5] = [
    ("bash", "lez", "eza"),
    ("zsh", "_lez", "_eza"),
    ("fish", "lez.fish", "eza.fish"),
    ("nush", "lez.nu", "eza.nu"),
    ("pwsh", "_lez.ps1", "_eza.ps1"),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn completion(dir: &str, file: &str) -> String {
    let path = repo_root().join("completions").join(dir).join(file);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()))
}

/// Every long name, aliases included, that clap will only accept with `=`.
fn flags_requiring_equals() -> Vec<String> {
    let mut names = Vec::new();
    for arg in lez::options::parser::get_command().get_arguments() {
        if !arg.is_require_equals_set() {
            continue;
        }
        if let Some(long) = arg.get_long() {
            names.push(long.to_owned());
        }
        for alias in arg.get_all_aliases().unwrap_or_default() {
            names.push(alias.to_owned());
        }
    }
    names.sort();
    assert!(
        names.len() >= 11,
        "expected at least the eleven known equals-only long names, found {names:?}"
    );
    names
}

/// The body of a `case … in … esac`, so a flag can be looked for in the arm
/// that completes after `=` without matching the one that completes after a
/// space.
fn case_body<'a>(script: &'a str, header: &str) -> &'a str {
    let after = script
        .split_once(header)
        .unwrap_or_else(|| panic!("bash completion should contain `{header}`"))
        .1;
    after
        .split_once("\n    esac")
        .unwrap_or_else(|| panic!("`{header}` should be closed by an esac"))
        .0
}

/// A case arm names a flag exactly, not as a prefix: `--color-scale-mode)` is
/// not an arm for `--color`.
fn names_a_case_arm(body: &str, flag: &str) -> bool {
    body.contains(&format!("--{flag})")) || body.contains(&format!("--{flag}|"))
}

#[test]
fn bash_moves_the_value_lists_behind_the_equals_sign() {
    let script = completion("bash", "lez");
    let equals_arms = case_body(&script, "case \"$eq_opt\" in");
    let space_arms = case_body(&script, "case \"$prev\" in");

    for flag in flags_requiring_equals() {
        assert!(
            names_a_case_arm(equals_arms, &flag),
            "bash offers no values for --{flag} after an equals sign"
        );
        assert!(
            !names_a_case_arm(space_arms, &flag),
            "bash still offers values for --{flag} after a space"
        );
    }
}

/// fish `complete` statements continue across lines with a trailing backslash.
fn fish_statements(script: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    for line in script.lines() {
        if let Some(head) = line.strip_suffix('\\') {
            current.push_str(head);
            continue;
        }
        current.push_str(line);
        statements.push(std::mem::take(&mut current));
    }
    if !current.is_empty() {
        statements.push(current);
    }
    statements
}

#[test]
fn fish_gates_the_value_lists_on_an_equals_sign() {
    let script = completion("fish", "lez.fish");
    let condition = "__lez_value_follows_an_equals_sign";
    assert!(
        script.contains(&format!("function {condition}")),
        "fish completion should define {condition}"
    );

    for flag in flags_requiring_equals() {
        let offering: Vec<String> = fish_statements(&script)
            .into_iter()
            .filter(|statement| {
                statement.contains(&format!("-l {flag} ")) && statement.contains(" -a ")
            })
            .collect();
        assert!(
            !offering.is_empty(),
            "fish offers no values at all for --{flag}"
        );
        for statement in offering {
            assert!(
                statement.contains(condition),
                "fish offers values for --{flag} without requiring an equals sign: {statement}"
            );
        }
    }
}

#[test]
fn nushell_leaves_the_equals_flags_undeclared() {
    let script = completion("nush", "lez.nu");
    for flag in flags_requiring_equals() {
        for line in script.lines() {
            let declared = line.trim_start();
            let name = declared.split([' ', '(', ':']).next().unwrap_or_default();
            assert!(
                name != format!("--{flag}"),
                "nushell declares --{flag}. It rewrites `--flag=value` into two \
                 arguments for anything it declares, and rejects it outright for a \
                 flag declared without a type, so declaring it is what breaks it: \
                 `{declared}`"
            );
        }
    }
}

/// Anything nushell *does* declare must agree with the parser: a flag that
/// takes a value has to be declared as taking one, or `--flag=value` becomes a
/// parse error inside nushell before the binary ever sees it.
#[test]
fn nushell_declares_a_type_for_every_flag_that_takes_a_value() {
    let script = completion("nush", "lez.nu");
    let command = lez::options::parser::get_command();

    for arg in command.get_arguments() {
        let takes_value = !matches!(
            arg.get_action(),
            clap::ArgAction::SetTrue
                | clap::ArgAction::SetFalse
                | clap::ArgAction::Count
                | clap::ArgAction::Help
                | clap::ArgAction::HelpShort
                | clap::ArgAction::HelpLong
                | clap::ArgAction::Version
        );
        let Some(long) = arg.get_long() else { continue };
        if !takes_value || arg.is_require_equals_set() {
            continue;
        }
        for line in script.lines() {
            let declared = line.trim_start();
            if !declared.starts_with(&format!("--{long}")) {
                continue;
            }
            let head = declared.split_whitespace().next().unwrap_or_default();
            let name = head.split(['(', ':']).next().unwrap_or_default();
            if name != format!("--{long}") {
                continue;
            }
            assert!(
                declared.contains(':'),
                "--{long} takes a value, but nushell declares it as a switch, which \
                 makes `--{long}=value` a parse error: `{declared}`"
            );
        }
    }
}

#[test]
fn powershell_completes_the_equals_flags_as_whole_words() {
    let script = completion("pwsh", "_lez.ps1");
    assert!(
        script.contains("$wordToComplete -like '*=*'"),
        "the PowerShell completer should serve the equals form before the switch"
    );

    for flag in flags_requiring_equals() {
        assert!(
            script.contains(&format!("'--{flag}'")),
            "PowerShell has no value list for --{flag} in $EqualsFlags"
        );
        assert!(
            !script.contains(&format!("'*;--{flag}'")),
            "PowerShell still offers values for --{flag} after a space"
        );
    }
}

/// The `eza`-named copies are the primary files with the command name
/// rewritten, nothing else. Checking the rewrite exactly — rather than
/// comparing with both names normalised away — also catches a copy that was
/// edited directly instead of regenerated.
#[test]
fn the_compat_copies_are_the_primary_files_with_the_name_rewritten() {
    for (dir, primary, compat) in BACKENDS {
        assert_eq!(
            completion(dir, primary).replace("lez", "eza"),
            completion(dir, compat),
            "completions/{dir}/{compat} must be completions/{dir}/{primary} with \
             `lez` rewritten to `eza`; regenerate it rather than editing it"
        );
    }
}

/// Locate a bash that can actually run a completion function: `compgen` and
/// `complete` are only present in builds with programmable completion, and
/// `mapfile` needs bash 4.
fn usable_bash() -> Option<PathBuf> {
    // Windows runners carry a Git bash, but it reads Unix paths and a PATH
    // separated by colons, which a `C:\\…` binary directory is not. Nothing is
    // lost by leaving it out: this completion is not installed there.
    if cfg!(windows) {
        return None;
    }
    for candidate in [
        "bash",
        "/bin/bash",
        "/usr/local/bin/bash",
        "/opt/homebrew/bin/bash",
    ] {
        let Ok(output) = Command::new(candidate)
            .args(["-c", r#"echo "${BASH_VERSINFO[0]} $(type -t compgen)""#])
            .output()
        else {
            continue;
        };
        let reported = String::from_utf8_lossy(&output.stdout);
        let mut fields = reported.split_whitespace();
        let major: u32 = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        if major >= 4 && fields.next() == Some("builtin") {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// Drive `_lez` the way readline does — COMP_WORDS split on COMP_WORDBREAKS,
/// which is why `--absolute=on` arrives as three words — and read back what it
/// would insert.
fn bash_completions_for(bash: &Path, words: &[&str], cword: usize) -> (Vec<String>, bool) {
    let root = repo_root();
    let binary_dir = Path::new(env!("CARGO_BIN_EXE_lez"))
        .parent()
        .expect("the test binary should live in a directory")
        .to_owned();
    let quoted: Vec<String> = words.iter().map(|word| format!("'{word}'")).collect();
    let script = format!(
        r#"
_filedir() {{ COMPREPLY=(FILEDIR); }}
compopt() {{ NOSPACE=yes; }}
PATH={binary_dir:?}:$PATH
source {source:?}
COMP_WORDS=({words})
COMP_CWORD={cword}
COMPREPLY=()
NOSPACE=no
_lez
printf '%s\n' "${{COMPREPLY[@]}}"
printf 'NOSPACE=%s\n' "$NOSPACE"
"#,
        binary_dir = binary_dir,
        source = root.join("completions").join("bash").join("lez"),
        words = quoted.join(" "),
        cword = cword,
    );

    let output = Command::new(bash)
        .arg("-c")
        .arg(script)
        .current_dir(&root)
        .output()
        .expect("bash should run");
    assert!(
        output.status.success(),
        "bash exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut replies: Vec<String> = stdout.lines().map(str::to_owned).collect();
    let nospace = replies.pop().as_deref() == Some("NOSPACE=yes");
    replies.retain(|reply| !reply.is_empty());
    (replies, nospace)
}

#[test]
fn bash_really_offers_the_values_only_after_an_equals_sign() {
    let Some(bash) = usable_bash() else {
        // nixpkgs builds its plain `bash` without programmable completion, so
        // the flake's own check cannot run this even though it runs on Linux.
        // The ubuntu job outside the sandbox is what holds the requirement.
        let inside_a_nix_build = std::env::var_os("NIX_BUILD_TOP").is_some();
        assert!(
            !cfg!(target_os = "linux") || inside_a_nix_build,
            "no bash with programmable completion was found, and Linux outside \
             the Nix sandbox is where this test is expected to run"
        );
        eprintln!("skipped: no bash with compgen and mapfile on this machine");
        return;
    };

    // `--absolute=` — the value replaces the `=`, so each candidate carries it.
    let (replies, _) = bash_completions_for(&bash, &["lez", "--absolute", "="], 2);
    assert_eq!(replies, ["=on", "=follow", "=off"]);

    // `--absolute=o` — the value replaces the partial word on its own.
    let (replies, _) = bash_completions_for(&bash, &["lez", "--absolute", "=", "o"], 3);
    assert_eq!(replies, ["on", "off"]);

    // A space is a new word, and a new word here is a path.
    let (replies, _) = bash_completions_for(&bash, &["lez", "--absolute", ""], 2);
    assert_eq!(
        replies,
        ["FILEDIR"],
        "a value after a space would be read as a path"
    );

    // The short form requires the sign too.
    let (replies, _) = bash_completions_for(&bash, &["lez", "-F", "="], 2);
    assert_eq!(replies, ["=always", "=automatic", "=auto", "=never"]);

    // --color-scale takes a comma-separated list; the fields already typed stay.
    let (replies, _) = bash_completions_for(&bash, &["lez", "--color-scale", "=", "age,s"], 3);
    assert_eq!(replies, ["age,size"]);

    // The flag itself completes to the equals sign, with the space held back.
    let (replies, nospace) = bash_completions_for(&bash, &["lez", "--abso"], 1);
    assert_eq!(replies, ["--absolute="]);
    assert!(
        nospace,
        "bash would end the word with a space after `--absolute=`"
    );

    // Flags that take their value after a space are left alone.
    let (replies, _) = bash_completions_for(&bash, &["lez", "--color-scale-mode", ""], 2);
    assert_eq!(replies, ["fixed", "gradient", "--"]);
}

#[test]
fn all_clap_flags_are_present_in_completions() {
    let command = lez::options::parser::get_command();
    let mut clap_longs = Vec::new();
    for arg in command.get_arguments() {
        if let Some(long) = arg.get_long()
            && long != "help"
            && long != "version"
        {
            clap_longs.push(long.to_owned());
        }
    }
    assert!(!clap_longs.is_empty());

    for (dir, primary, _) in BACKENDS {
        let script = completion(dir, primary);
        if dir == "bash" {
            // Bash completions use dynamic `lez --help` runtime parsing
            assert!(
                script.contains("lez --help"),
                "completions/bash/{primary} must contain dynamic lez --help parser"
            );
        } else {
            for flag in &clap_longs {
                let zsh_expanded = flag.replace("color", "colo{,u}r");
                assert!(
                    script.contains(flag) || (dir == "zsh" && script.contains(&zsh_expanded)),
                    "completions/{dir}/{primary} is missing CLI flag --{flag}"
                );
            }
        }
    }
}
