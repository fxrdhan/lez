// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Guards for `powertest.yaml`, the generator input behind `tests/ptests`.
//!
//! `just regen` deletes `tests/ptests` and rebuilds it from this file, so a
//! mistake here is not visible until someone regenerates and finds the suite
//! rewritten. Two mistakes had already accumulated that way: the file named a
//! binary this project does not build, and it declared flags in a form the
//! generator renders with a space, which is not a form those flags accept.
//!
//! The same goes for `devtools/generate-trycmd-test.sh`, which writes cases
//! into `tests/cmd` by hand. It carried the same stale binary name for longer,
//! and nothing noticed, because the twenty cases already committed there were
//! written or corrected by hand.

use std::fs;
use std::path::Path;

fn workspace_file(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()))
}

/// The name under `[[bin]]` — the only command this project installs.
fn binary_name() -> String {
    let manifest = workspace_file("Cargo.toml");
    let after = manifest
        .split_once("[[bin]]")
        .expect("Cargo.toml must declare a [[bin]]")
        .1;
    let line = after
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .expect("[[bin]] must give a name");
    line.split('"')
        .nth(1)
        .expect("the [[bin]] name should be quoted")
        .to_owned()
}

/// Every long name, aliases included, that clap will only accept with `=`,
/// plus the short forms of the same arguments.
fn flags_requiring_equals() -> Vec<String> {
    let mut names = Vec::new();
    for arg in lsr::options::parser::get_command().get_arguments() {
        if !arg.is_require_equals_set() {
            continue;
        }
        if let Some(long) = arg.get_long() {
            names.push(format!("--{long}"));
        }
        for alias in arg.get_all_aliases().unwrap_or_default() {
            names.push(format!("--{alias}"));
        }
        if let Some(short) = arg.get_short() {
            names.push(format!("-{short}"));
        }
    }
    names.sort();
    names
}

/// The flag tokens `powertest.yaml` uses as map keys, paired with the line
/// number they sit on. A key is one or two list items under a `?`.
fn declared_flags(config: &str) -> Vec<(usize, String)> {
    let mut flags = Vec::new();
    for (index, line) in config.lines().enumerate() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("? - ")
            .or_else(|| trimmed.strip_prefix("- "))
        else {
            continue;
        };
        // Drop the trailing `# section heading` comments the file carries.
        let token = rest.split(" #").next().unwrap_or(rest).trim();
        if token.is_empty() || token == "null" || !token.starts_with('-') {
            continue;
        }
        flags.push((index + 1, token.to_owned()));
    }
    flags
}

#[test]
fn the_generator_targets_the_binary_this_project_builds() {
    let config = workspace_file("powertest.yaml");
    let binary = binary_name();
    // Matched line by line: a Windows checkout can carry CRLF, and `lines`
    // strips the carriage return that a raw `contains` would trip over.
    let declares = |setting: &str| config.lines().any(|line| line.trim_end() == setting);
    assert!(
        declares(&format!("binary: {binary}")),
        "powertest.yaml must set `binary: {binary}`; every generated case \
         carries it as bin.name, so a stale value points the whole suite at a \
         command that is not installed"
    );
    assert!(
        declares(&format!("gen_binary: target/debug/{binary}")),
        "powertest.yaml must set `gen_binary: target/debug/{binary}`"
    );
}

/// populate_set renders a key and its value as `format!("{} {}", flag, value)`
/// — a space, always. So a flag that requires an equals sign cannot carry a
/// `values:` list; the value has to be written into the key itself.
#[test]
fn equals_only_flags_are_spelled_out_in_the_key() {
    let config = workspace_file("powertest.yaml");
    let equals_only = flags_requiring_equals();

    for (line_number, token) in declared_flags(&config) {
        let name = token.split('=').next().unwrap_or(&token);
        if !equals_only.iter().any(|flag| flag == name) {
            continue;
        }
        assert!(
            token.contains('='),
            "powertest.yaml:{line_number} declares `{token}`, which the \
             generator renders as `{token} <value>`. That flag only accepts a \
             value attached with an equals sign, so write the pair out as \
             `{token}=<value>` and drop the `values:` list"
        );
    }
}

/// A key that already carries its value must not also carry a `values:` list,
/// or the generator appends a second value after a space.
#[test]
fn a_key_carrying_its_value_has_no_values_list() {
    let config = workspace_file("powertest.yaml");
    let lines: Vec<&str> = config.lines().collect();

    for (line_number, token) in declared_flags(&config) {
        if !token.contains('=') {
            continue;
        }
        let mapping = lines[line_number..]
            .iter()
            .find(|line| line.trim_start().starts_with(':'))
            .unwrap_or_else(|| panic!("the key on line {line_number} should be closed by a `:`"));
        assert_eq!(
            mapping.trim(),
            ":",
            "powertest.yaml:{line_number} gives `{token}` a values list; the \
             generator would render `{token} <value>`"
        );
    }
}

/// The other generator. It writes `bin.name` into a `tests/cmd` case and runs
/// the built binary to record the output, so both spellings have to be the
/// name this project actually builds. This one went unnoticed longer than the
/// `powertest.yaml` mistake: every case already in `tests/cmd` says `lsr`,
/// while the script that claims to produce them said `eza`.
#[test]
fn the_trycmd_generator_names_the_binary_this_project_builds() {
    let script = workspace_file("devtools/generate-trycmd-test.sh");
    let binary = binary_name();

    assert!(
        script.contains(&format!("bin.name = \"{binary}\"")),
        "generate-trycmd-test.sh must write `bin.name = \"{binary}\"` into the case \
         it generates, or every case made with it names a command that is not built"
    );
    assert!(
        script.contains(&format!("/debug/{binary}")),
        "generate-trycmd-test.sh must run the {binary} binary to record the output"
    );
    assert!(
        !script.contains("eza"),
        "generate-trycmd-test.sh still mentions eza somewhere"
    );
}

/// And what it claims to generate has to match what is already there, which is
/// the check that would have caught the drift at any point in the last year.
#[test]
fn the_committed_cmd_cases_agree_with_the_generator() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cmd");
    let binary = binary_name();
    let expected = format!("bin.name = \"{binary}\"");

    let mut checked = 0;
    for entry in fs::read_dir(&dir).expect("tests/cmd should be readable") {
        let path = entry.expect("the entry should be readable").path();
        if path.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let case = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} should be readable: {e}", path.display()));
        assert!(
            case.contains(&expected),
            "{} should name the {binary} binary",
            path.display()
        );
        checked += 1;
    }
    assert!(checked > 0, "tests/cmd should hold some cases");
}
