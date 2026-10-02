// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! `-A` and `-aa` override each other, the last one given winning; and in
//! strict mode the newer long-view flags, and `--follow-symlinks` without
//! recursion, are options errors rather than silently doing nothing.

use crate::common::{TempTestDir, lez_in, success_stdout};

fn fixture() -> TempTestDir {
    let dir = TempTestDir::new("options_hardening");
    dir.create_file("regular.txt", b"");
    dir.create_file(".hidden.txt", b"");
    dir
}

#[test]
fn the_last_of_almost_all_and_all_all_wins() {
    let dir = fixture();
    let with_dots = ".\n..\n.hidden.txt\nregular.txt\n";
    let without_dots = ".hidden.txt\nregular.txt\n";
    for (flags, expected) in [
        (&["-a"][..], without_dots),
        (&["-a", "-a"], with_dots),
        (&["-A"], without_dots),
        (&["-A", "-a"], without_dots),
        (&["-A", "-a", "-a"], with_dots),
        (&["-a", "-a", "-A"], without_dots),
    ] {
        assert_eq!(
            success_stdout(lez_in(dir.path()).arg("-1").args(flags)),
            expected,
            "{flags:?}"
        );
    }
}

#[test]
fn strict_mode_refuses_flags_their_view_ignores() {
    let dir = fixture();
    for (flag, needs, accepted_with) in [
        ("--color-scale=size", "option long", "-l"),
        ("--color-scale-mode=gradient", "option long", "-l"),
        ("--no-symlink-targets", "option long", "-l"),
        ("--follow-symlinks", "options recurse or tree", "-R"),
        ("--follow-symlinks", "options recurse or tree", "-T"),
    ] {
        let strict = |extra: &[&str]| {
            lez_in(dir.path())
                .env("LEZ_STRICT", "1")
                .args(extra)
                .args([flag, "regular.txt"])
                .output()
                .expect("run lez")
        };
        let refused = strict(&[]);
        assert_eq!(refused.status.code(), Some(3), "{flag}");
        assert!(refused.stdout.is_empty(), "{flag}");
        let name = flag
            .trim_start_matches("--")
            .split('=')
            .next()
            .unwrap_or_default();
        assert_eq!(
            String::from_utf8_lossy(&refused.stderr),
            format!("lez: Option {name} is useless without {needs}\n")
        );

        let accepted = strict(&[accepted_with]);
        assert_eq!(accepted.status.code(), Some(0), "{flag} {accepted_with}");
        assert!(accepted.stderr.is_empty(), "{flag} {accepted_with}");
    }
}
