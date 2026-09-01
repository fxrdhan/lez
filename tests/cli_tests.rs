#[test]
fn cli_all_tests() {
    trycmd::TestCases::new().case("tests/cmd/*_all.toml");
}

#[test]
#[cfg(unix)]
fn cli_unix_tests() {
    trycmd::TestCases::new().case("tests/cmd/*_unix.toml");
}

#[test]
#[cfg(windows)]
fn cli_windows_tests() {
    trycmd::TestCases::new().case("tests/cmd/*_windows.toml");
}

#[test]
#[cfg(feature = "nix-local")]
fn cli_nix_local_tests() {
    trycmd::TestCases::new().case("tests/cmd/*_nix_local.toml");
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
        if status.as_ref().map_or(false, |s| s.success()) {
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
    trycmd::TestCases::new()
        .env("LS_COLORS", "")
        .env("LEZ_COLORS", "")
        .env("EZA_COLORS", "")
        .env("EXA_COLORS", "")
        .case("tests/ptests/*.toml");
}

#[test]
#[cfg(feature = "nix")]
fn cli_nix_generated_tests() {
    require_generated_fixture("nix");
    trycmd::TestCases::new().case("tests/gen/*.toml");
}
