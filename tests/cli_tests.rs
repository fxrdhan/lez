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
    assert!(
        std::path::Path::new("tests/test_dir").exists(),
        "the `{feature}` feature runs the generated suites, which need \
         tests/test_dir. It is built by the Nix flake (`nix build ./#trycmd`) \
         and cannot be created on macOS, where devtools/dir-generator.sh needs \
         groupadd. Drop the feature to leave these suites out."
    );
}

#[test]
#[cfg(feature = "powertest")]
fn cli_powertest_tests() {
    require_generated_fixture("powertest");
    trycmd::TestCases::new().case("tests/ptests/*.toml");
}

#[test]
#[cfg(feature = "nix")]
fn cli_nix_generated_tests() {
    require_generated_fixture("nix");
    trycmd::TestCases::new().case("tests/gen/*.toml");
}
