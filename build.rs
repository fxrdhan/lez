// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
/// The version string isn’t the simplest: we want to show the version,
/// current Git hash, and compilation date when building *debug* versions, but
/// just the version for *release* versions so the builds are reproducible.
///
/// This script generates the string from the environment variables that Cargo
/// adds (<http://doc.crates.io/environment-variables.html>) and runs `git` to
/// get the SHA1 hash. It then writes the string into a file, which lez then
/// includes at build-time.
///
/// - <https://stackoverflow.com/q/43753491/3484614>
/// - <https://crates.io/crates/vergen>
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The build script entry point.
fn main() -> io::Result<()> {
    // Emit explicit rerun directives so edits to unrelated files (docs, tests,
    // `src/main.rs`) do not re-run `build.rs` and invalidate the `lez` library.
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=PROFILE");
    println!("cargo::rerun-if-env-changed=CARGO_PKG_VERSION");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_GIT");

    let tagline = "A modern, fast, and feature-rich replacement for ls written in Rust";
    let url = "https://github.com/fxrdhan/lez";

    let ver = if is_debug_build() {
        format!(
            "{}\nv{} \\1;31m(pre-release debug build!)\\0m\n\\1;4;34m{}\\0m",
            tagline,
            version_string(),
            url
        )
    } else if is_development_version() {
        emit_git_head_rerun_directives();
        format!(
            "{}\nv{} [{}] built on {} \\1;31m(pre-release!)\\0m\n\\1;4;34m{}\\0m",
            tagline,
            version_string(),
            git_hash(),
            build_date(),
            url
        )
    } else {
        format!("{}\nv{}\n\\1;4;34m{}\\0m", tagline, version_string(), url)
    };

    // We need to create these files in the Cargo output directory.
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by Cargo"));
    let path = out.join("version_string.txt");

    // Write only when the version text actually changed so Cargo does not
    // invalidate `include_str!(...)` consumers on a no-op rerun.
    let content = format!("{}\n", strip_codes(&ver));
    if fs::read_to_string(&path).ok().as_deref() != Some(content.as_str()) {
        fs::write(&path, content)?;
    }

    Ok(())
}

/// Emits `rerun-if-changed` directives for the current Git `HEAD` (and its
/// target ref, if symbolic), handling both regular repositories and worktrees.
fn emit_git_head_rerun_directives() {
    let Some(git_dir) = resolve_git_dir() else {
        return;
    };
    let head_path = git_dir.join("HEAD");
    if head_path.exists() {
        println!("cargo::rerun-if-changed={}", head_path.display());
        if let Ok(head) = fs::read_to_string(&head_path)
            && let Some(reference) = head.trim().strip_prefix("ref: ")
        {
            let ref_path = git_dir.join(reference);
            if ref_path.exists() {
                println!("cargo::rerun-if-changed={}", ref_path.display());
            }
        }
    }
}

fn resolve_git_dir() -> Option<PathBuf> {
    let dot_git = Path::new(".git");
    if dot_git.is_dir() {
        return Some(dot_git.to_path_buf());
    }
    if dot_git.is_file() {
        let text = fs::read_to_string(dot_git).ok()?;
        let raw = text.trim().strip_prefix("gitdir:")?.trim();
        let candidate = PathBuf::from(raw);
        return Some(if candidate.is_absolute() {
            candidate
        } else {
            dot_git
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(candidate)
        });
    }
    None
}

/// Removes escape codes from a string.
fn strip_codes(input: &str) -> String {
    input
        .replace("\\0m", "")
        .replace("\\1;31m", "")
        .replace("\\1;4;34m", "")
}

/// Retrieve the project’s current Git hash, as a string.
fn git_hash() -> String {
    use std::process::Command;

    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

/// Whether we should show pre-release info in the version string.
///
/// Both weekly releases and actual releases are --release releases,
/// but actual releases will have a proper version number.
fn is_development_version() -> bool {
    cargo_version().ends_with("-pre") || env::var("PROFILE").unwrap() == "debug"
}

/// Whether we are building in debug mode.
fn is_debug_build() -> bool {
    env::var("PROFILE").unwrap() == "debug"
}

/// Retrieves the [package] version in Cargo.toml as a string.
fn cargo_version() -> String {
    env::var("CARGO_PKG_VERSION").unwrap()
}

/// Returns the version and build parameters string.
fn version_string() -> String {
    let mut ver = cargo_version();

    let feats = nonstandard_features_string();
    if !feats.is_empty() {
        ver.push_str(&format!(" [{feats}]"));
    }

    ver
}

/// Finds whether a feature is enabled by examining the Cargo variable.
fn feature_enabled(name: &str) -> bool {
    env::var(format!("CARGO_FEATURE_{name}"))
        .map(|e| !e.is_empty())
        .unwrap_or(false)
}

/// A comma-separated list of non-standard feature choices.
fn nonstandard_features_string() -> String {
    let mut s = Vec::new();

    if feature_enabled("GIT") {
        s.push("+git");
    } else {
        s.push("-git");
    }

    s.join(", ")
}

/// Formats the current UTC date as an ISO 8601 string (`YYYY-MM-DD`) without
/// external build-dependencies (Howard Hinnant's civil-from-days algorithm).
fn build_date() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc_date(secs)
}

fn format_utc_date(unix_secs: u64) -> String {
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}
