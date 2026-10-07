// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `packaging/install.sh`, the `curl | bash` installer, run offline against
//! stand-ins for `uname`, `curl` and `cargo`. A platform it recognises either
//! downloads the release's prebuilt binary or, where the release has none,
//! builds that release with Cargo; anything else is refused.

#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use crate::common::TempTestDir;

const TAG: &str = "v9.9.9";

/// What a download installs: a `lez` that only names itself.
const STAND_IN_LEZ: &str = "#!/bin/sh\necho stand-in lez\n";

/// Prints the platform the test names.
const UNAME: &str = r#"#!/bin/sh
case "$1" in
    -s) echo "$STUB_OS" ;;
    -m) echo "$STUB_ARCH" ;;
    *) exit 1 ;;
esac
"#;

/// Answers the API with the release below and serves the assets it lists.
/// Any other URL fails the way `curl -f` fails on a 404.
const CURL: &str = r#"#!/bin/sh
url=""
out=""
while [ $# -gt 0 ]; do
    case "$1" in
        -o) out="$2"; shift 2 ;;
        -*) shift ;;
        *) url="$1"; shift ;;
    esac
done
echo "curl $url" >> "$STUB_LOG"
case "$url" in
    https://api.github.com/*) cat "$STUB_RELEASE" ;;
    *)
        if grep -qF "\"$url\"" "$STUB_RELEASE"; then
            cp "$STUB_ARCHIVE" "$out"
        else
            echo "curl: (22) The requested URL returned error: 404" >&2
            exit 22
        fi
        ;;
esac
"#;

const CARGO: &str = r#"#!/bin/sh
echo "cargo $*" >> "$STUB_LOG"
"#;

/// The latest release as the GitHub API prints it, cut down to what the
/// installer reads. Like every release so far, it has prebuilt binaries for
/// Linux on x86_64 and for macOS, and none for Linux on ARM64.
fn release_json() -> String {
    let assets = [
        "lez_x86_64-unknown-linux-gnu.tar.gz",
        "lez_aarch64-apple-darwin.tar.gz",
        "lez_x86_64-apple-darwin.tar.gz",
        "lez_x86_64-pc-windows-msvc.zip",
    ]
    .map(|name| {
        format!(
            "    {{\n      \"name\": \"{name}\",\n      \"browser_download_url\": \
             \"https://github.com/fxrdhan/lez/releases/download/{TAG}/{name}\"\n    }}"
        )
    })
    .join(",\n");
    format!("{{\n  \"tag_name\": \"{TAG}\",\n  \"assets\": [\n{assets}\n  ]\n}}\n")
}

struct Run {
    output: Output,
    /// Each URL the installer fetched and each `cargo` command it ran.
    log: String,
    /// The `lez` it put in `INSTALL_DIR`, if any.
    installed: Option<String>,
}

impl Run {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    fn ran_cargo(&self) -> bool {
        self.log.lines().any(|line| line.starts_with("cargo "))
    }
}

fn run_installer(os: &str, arch: &str, with_cargo: bool) -> Run {
    let dir = TempTestDir::new("install_sh");
    let bin = dir.create_dir("bin");
    let stand_ins = [("uname", UNAME), ("curl", CURL)]
        .into_iter()
        .chain(with_cargo.then_some(("cargo", CARGO)));
    for (name, script) in stand_ins {
        let path = bin.join(name);
        fs::write(&path, script).expect("failed to write a stand-in");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("failed to make a stand-in executable");
    }

    let release = dir.create_file("release.json", release_json().as_bytes());
    let staged = dir.create_file("staged/lez", STAND_IN_LEZ.as_bytes());
    let archive = dir.path().join("archive.tar.gz");
    let tar = Command::new("tar")
        .arg("czf")
        .arg(&archive)
        .arg("-C")
        .arg(staged.parent().expect("the staged lez has a parent"))
        .arg("lez")
        .status()
        .expect("failed to run tar");
    assert!(tar.success(), "tar could not build the release archive");

    // The stand-ins come first, and a directory with a real `cargo` is left
    // out, so that only the stand-in, if there is one, can be found.
    let path = env::join_paths(
        std::iter::once(bin).chain(
            env::split_paths(&env::var_os("PATH").unwrap_or_default())
                .filter(|dir| !dir.join("cargo").exists()),
        ),
    )
    .expect("PATH should join");

    let log = dir.path().join("log");
    let install_dir = dir.path().join("install");
    let output = Command::new("bash")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/install.sh"))
        .env("PATH", path)
        .env("STUB_OS", os)
        .env("STUB_ARCH", arch)
        .env("STUB_LOG", &log)
        .env("STUB_RELEASE", &release)
        .env("STUB_ARCHIVE", &archive)
        .env("INSTALL_DIR", &install_dir)
        .output()
        .expect("failed to run bash");

    Run {
        output,
        log: fs::read_to_string(&log).unwrap_or_default(),
        installed: fs::read_to_string(install_dir.join("lez")).ok(),
    }
}

/// Linux on x86_64 and macOS on either architecture install the release's
/// binary, whichever name `uname -m` gives the architecture.
#[test]
fn a_platform_with_a_prebuilt_binary_downloads_it() {
    for (os, arch, target) in [
        ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ("Linux", "amd64", "x86_64-unknown-linux-gnu"),
        ("Darwin", "arm64", "aarch64-apple-darwin"),
        ("Darwin", "x86_64", "x86_64-apple-darwin"),
    ] {
        let run = run_installer(os, arch, true);
        assert!(run.output.status.success(), "{os} {arch}: {}", run.stderr());
        let download = format!(
            "curl https://github.com/fxrdhan/lez/releases/download/{TAG}/lez_{target}.tar.gz"
        );
        assert!(run.log.contains(&download), "{os} {arch}: {}", run.log);
        assert!(!run.ran_cargo(), "{os} {arch}: {}", run.log);
        assert_eq!(run.installed.as_deref(), Some(STAND_IN_LEZ), "{os} {arch}");
    }
}

/// No release has a binary for Linux on ARM64. The installer used to ask for
/// one anyway and stop at the 404; it builds the release's tag with Cargo.
#[test]
fn linux_on_arm64_builds_the_release_with_cargo() {
    for arch in ["aarch64", "arm64"] {
        let run = run_installer("Linux", arch, true);
        assert!(run.output.status.success(), "{arch}: {}", run.stderr());
        let fetched: Vec<&str> = run
            .log
            .lines()
            .filter(|line| line.starts_with("curl "))
            .collect();
        assert_eq!(
            fetched,
            ["curl https://api.github.com/repos/fxrdhan/lez/releases/latest"],
            "{arch}: only the release should be looked up"
        );
        let build =
            format!("cargo install --git https://github.com/fxrdhan/lez.git --tag {TAG} --locked");
        assert!(run.log.contains(&build), "{arch}: {}", run.log);
        assert_eq!(run.installed, None, "{arch}");
    }
}

/// Without Cargo it cannot build one either, and says what to do instead.
#[test]
fn linux_on_arm64_without_cargo_says_how_to_install() {
    let run = run_installer("Linux", "aarch64", false);
    assert_eq!(run.output.status.code(), Some(1), "{}", run.stderr());
    let stderr = run.stderr();
    assert!(
        stderr.contains(&format!(
            "{TAG} has no prebuilt binary for aarch64-unknown-linux-gnu"
        )),
        "{stderr}"
    );
    assert!(stderr.contains("https://rustup.rs"), "{stderr}");
    assert!(stderr.contains("INSTALL.md"), "{stderr}");
    assert!(!stderr.contains("404"), "{stderr}");
}

/// A system or an architecture it has no name for is refused before
/// anything is fetched.
#[test]
fn an_unknown_platform_is_refused() {
    for (os, arch, message) in [
        ("FreeBSD", "x86_64", "Unsupported operating system: FreeBSD"),
        ("Linux", "riscv64", "Unsupported CPU architecture: riscv64"),
    ] {
        let run = run_installer(os, arch, true);
        assert_eq!(run.output.status.code(), Some(1), "{os} {arch}");
        assert!(
            run.stderr().contains(message),
            "{os} {arch}: {}",
            run.stderr()
        );
        assert!(run.log.is_empty(), "{os} {arch}: {}", run.log);
    }
}
