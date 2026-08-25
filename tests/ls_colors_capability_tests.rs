// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `LS_COLORS` has a `ca` entry for files carrying Linux capabilities, and GNU
//! `ls` honours it. Answering whether a file has them costs a `getxattr`, so
//! the lookup only happens when a style was actually asked for — these hold
//! both halves of that.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_capability_{tag}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the fixture directory should be creatable");
        Self { path }
    }

    fn file(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::write(&p, b"#!/bin/sh\n").unwrap();
        p
    }

    fn lez(&self, ls_colors: &str, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_lez"))
            .args(["--color=always"])
            .args(args)
            .current_dir(&self.path)
            .env("LS_COLORS", ls_colors)
            .env_remove("EZA_COLORS")
            .env_remove("LEZ_COLORS")
            .output()
            .expect("lez should run");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Give the file a capability, or say why not. Needs `setcap` and the
/// privilege to use it, which a CI runner has and a developer machine may not.
#[cfg(target_os = "linux")]
fn grant_capability(path: &std::path::Path) -> bool {
    Command::new("sudo")
        .args(["-n", "setcap", "cap_net_raw+ep"])
        .arg(path)
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A file with capabilities takes the `ca` colour rather than the plain one.
#[test]
#[cfg(target_os = "linux")]
fn a_file_with_capabilities_takes_the_ca_colour() {
    let dir = Fixture::new("granted");
    let file = dir.file("with_caps");
    if !grant_capability(&file) {
        // A skipped test looks exactly like a passing one, so it is only
        // allowed off CI. On a runner setcap is available, and if it stops
        // being so this should say so rather than quietly prove nothing.
        assert!(
            std::env::var_os("CI").is_none(),
            "setcap should be available on CI; without it this test proves nothing"
        );
        eprintln!("skipped: setcap is not available to this account");
        return;
    }

    let listing = dir.lez("ca=38;5;17", &["with_caps"]);
    assert!(
        listing.contains("38;5;17"),
        "the ca style should have been used: {listing:?}"
    );
}

/// And the colour is not handed out to files that do not have any.
#[test]
#[cfg(target_os = "linux")]
fn a_file_without_capabilities_does_not() {
    let dir = Fixture::new("plain");
    dir.file("no_caps");

    let listing = dir.lez("ca=38;5;17", &["no_caps"]);
    assert!(
        !listing.contains("38;5;17"),
        "a file without capabilities should not borrow the ca style: {listing:?}"
    );
}

/// Nothing looks for the attribute unless a style was set for it. This holds
/// the guard rather than the colour, so it is worth running everywhere: the
/// listing must be unchanged by a `ca` entry that is not there.
#[test]
fn without_a_ca_entry_the_listing_is_untouched() {
    let dir = Fixture::new("unset");
    dir.file("some_file");

    let with_ca = dir.lez("ca=38;5;17:fi=0", &["some_file"]);
    let without = dir.lez("fi=0", &["some_file"]);
    assert_eq!(
        with_ca, without,
        "a file with no capabilities should look the same either way"
    );
}

/// `ca` must not leak into the colour of a directory, which cannot carry
/// capabilities.
#[test]
fn a_directory_never_takes_the_ca_colour() {
    let dir = Fixture::new("dir");
    fs::create_dir(dir.path.join("subdir")).unwrap();

    let listing = dir.lez("ca=38;5;17:di=34", &["-d", "subdir"]);
    assert!(
        !listing.contains("38;5;17"),
        "a directory should keep its own colour: {listing:?}"
    );
}
