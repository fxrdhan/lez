mod common;

/// Runs the cases matching `pattern`. Each case file sets
/// `env.inherit = false`, so lez sees none of the environment the tests run
/// in (`LS_COLORS`, `TZ`, `COLUMNS`, a `LEZ_*` setting, the user's config)
/// beyond what is added here and in the case itself.
fn run_isolated(pattern: &str) {
    let cases = trycmd::TestCases::new();
    cases.env(
        "LEZ_CONFIG_DIR",
        common::no_config_dir().to_string_lossy().into_owned(),
    );
    // Windows programs expect these, as `common::lez_cmd` notes.
    #[cfg(windows)]
    for name in ["SystemRoot", "SYSTEMROOT", "USERPROFILE", "ComSpec"] {
        if let Ok(value) = std::env::var(name) {
            cases.env(name, value);
        }
    }
    cases.case(pattern);
}

#[test]
fn cli_all_tests() {
    run_isolated("tests/cmd/*_all.toml");
}

#[test]
#[cfg(unix)]
fn cli_unix_tests() {
    run_isolated("tests/cmd/*_unix.toml");
}

#[test]
#[cfg(windows)]
fn cli_windows_tests() {
    run_isolated("tests/cmd/*_windows.toml");
}

#[test]
#[cfg(feature = "nix-local")]
fn cli_nix_local_tests() {
    run_isolated("tests/cmd/*_nix_local.toml");
}

/// The generated suites need a fixture that only the Nix build produces, so
/// they used to skip themselves whenever it was absent. Asking for the feature
/// and silently getting nothing is how nine stale cases reached CI unnoticed;
/// the two derivations that turn these features on both build the fixture
/// first, so a missing one means something is wrong rather than merely absent.
#[cfg(any(feature = "powertest", feature = "nix"))]
fn require_generated_fixture(feature: &str) {
    let fixture = std::path::Path::new("tests/test_dir");
    if !fixture.exists() {
        let status = std::process::Command::new("bash")
            .arg("devtools/dir-generator.sh")
            .arg("tests/test_dir")
            .status();
        if status.as_ref().is_ok_and(|s| s.success()) {
            let _ = std::process::Command::new("bash")
                .arg("devtools/generate-timestamp-test-dir.sh")
                .arg("tests/timestamp_test_dir")
                .status();
        }
    }
    assert!(
        fixture.exists(),
        "the `{feature}` feature runs the generated suites, which need \
         tests/test_dir. Run `bash devtools/dir-generator.sh tests/test_dir` \
         or `just gen_test_dir` to create it."
    );
}

#[test]
#[cfg(feature = "powertest")]
fn cli_powertest_tests() {
    require_generated_fixture("powertest");
    run_isolated("tests/ptests/*.toml");
}

#[test]
#[cfg(feature = "nix")]
fn cli_nix_generated_tests() {
    require_generated_fixture("nix");
    run_isolated("tests/gen/*.toml");
}
