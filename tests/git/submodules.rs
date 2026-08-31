// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `--ignore-submodule-contents`: recursion must not descend into Git
//! submodule working trees. The submodule entry itself stays listed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new(prefix: &str) -> Option<Self> {
        if !git_available() {
            eprintln!("git not available; skipping");
            return None;
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lez_submodule_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp repo root");
        Some(Self { path })
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.path.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    fn git(&self, rel: &str, args: &[&str]) -> bool {
        let cwd = self.path.join(rel);
        let output = Command::new("git")
            .args(
                ["-c", "user.name=t", "-c", "user.email=t@example.com"]
                    .iter()
                    .chain(args.iter()),
            )
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to spawn git");
        if !output.status.success() {
            eprintln!("git {:?} failed: {:?}", args, output.stderr);
        }
        output.status.success()
    }

    /// Parent repo containing a committed file plus one submodule (`mod`)
    /// whose own tree holds `inner.txt` and `deep/nested.txt`.
    fn with_submodule(&self) -> bool {
        // child repository
        self.write("child/inner.txt", "inner");
        self.write("child/deep/nested.txt", "nested");
        if !(self.git("child", &["init", "-q"])
            && self.git("child", &["add", "."])
            && self.git("child", &["commit", "-q", "-m", "init"]))
        {
            return false;
        }

        // parent repository
        self.write("outer.txt", "outer");
        if !(self.git("", &["init", "-q"]) && self.git("", &["add", "."])) {
            return false;
        }
        let child_abs = self.path.join("child");
        let _ = fs::remove_dir_all(self.path.join(".git")); // no-op safety
        if !self.git(
            "",
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                child_abs.to_str().unwrap(),
                "sub",
            ],
        ) || !self.git("", &["commit", "-q", "-m", "add submodule"])
        {
            return false;
        }
        self.path.join("sub").join("inner.txt").exists()
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn run_lez(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_lez"))
        .args(args)
        .output()
        .expect("Failed to execute lez binary");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn ignore_submodule_contents_prunes_recursion() {
    let Some(repo) = TempRepo::new("ignore") else {
        return;
    };
    if !repo.with_submodule() {
        eprintln!("could not build submodule fixture; skipping");
        return;
    }
    let sub_inner = Path::new("sub").join("inner.txt");
    let nested = Path::new("sub").join("deep").join("nested.txt");

    // Without the flag recursion descends into the submodule.
    let stdout = run_lez(&[
        "-R",
        "--color=never",
        repo.path.join("sub").to_str().unwrap(),
    ]);
    assert!(stdout.contains("inner.txt"), "{stdout}");

    // With it, the submodule's own tree is pruned but the entry remains.
    let stdout = run_lez(&[
        "-R",
        "--color=never",
        "--ignore-submodule-contents",
        repo.path.to_str().unwrap(),
    ]);
    assert!(stdout.contains("outer.txt"), "{stdout}");
    assert!(
        stdout.lines().any(|l| l.trim().ends_with("sub")),
        "the submodule directory itself must stay listed: {stdout}"
    );
    assert!(
        !stdout.contains(sub_inner.to_str().unwrap()) && !stdout.contains("inner.txt"),
        "submodule contents must not be listed: {stdout}"
    );
    assert!(!stdout.contains(nested.file_name().unwrap().to_str().unwrap()));
}
