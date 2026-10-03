// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Where lez finds its config file and which one wins. A file given with
//! `--config`, or else `LEZ_CONFIG_FILE` (then `EZA_`, `EXA_`), is the only
//! one read. Otherwise the first of `config.toml`, `lez.toml`,
//! `config.yaml` and `config.yml` in the config directory is laid under the
//! first of `.lez.toml`, `.lez.yaml`, `.lez.yml`, `.eza.toml` and
//! `.eza.yaml` in the working directory. `--no-config` reads none, and the
//! command line beats them all. What each key does is checked at the end.

use std::path::Path;

/// `a.txt` and `b.md`, which sort one way by name and the other by
/// extension.
fn work_dir(dir: &crate::common::TempTestDir) -> std::path::PathBuf {
    dir.create_file("work/a.txt", b"");
    dir.create_file("work/b.md", b"");
    dir.path().join("work")
}

const HEADER: &str = "[display]\nheader = true\n";
const NO_HEADER: &str = "[display]\nheader = false\n";
const BY_EXTENSION: &str = "[filter]\nsort = \"extension\"\n";
const PLAIN: &str = "a.txt\nb.md\n";
const HEADED: &str = "Name\na.txt\nb.md\n";

/// The listing of `work` in the long view's name column, where a config
/// with `header = true` shows a `Name` line.
fn listing(work: &Path, config_dir: &Path, args: &[&str], envs: &[(&str, &Path)]) -> String {
    let mut cmd = crate::common::lez_in(work);
    cmd.env("LEZ_CONFIG_DIR", config_dir);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    crate::common::success_stdout(cmd.args(crate::common::NAME_COLUMN_ONLY).args(args))
}

#[test]
fn every_place_lez_looks_for_a_config_file() {
    let dir = crate::common::TempTestDir::new("config_places");
    let work = work_dir(&dir);
    let nowhere = dir.path().join("nowhere");
    assert_eq!(listing(&work, &nowhere, &[], &[]), PLAIN);

    for name in ["config.toml", "lez.toml", "config.yaml", "config.yml"] {
        let global = dir.path().join(format!("global_{name}"));
        let yaml = name.ends_with(".yaml") || name.ends_with(".yml");
        let contents = if yaml {
            "display:\n  header: true\n"
        } else {
            HEADER
        };
        std::fs::create_dir(&global).expect("create the config directory");
        std::fs::write(global.join(name), contents).expect("write the config");
        assert_eq!(listing(&work, &global, &[], &[]), HEADED, "{name}");
        assert_eq!(
            listing(&work, &global, &["--no-config"], &[]),
            PLAIN,
            "{name}"
        );
    }

    for name in [
        ".lez.toml",
        ".lez.yaml",
        ".lez.yml",
        ".eza.toml",
        ".eza.yaml",
    ] {
        let local = dir.path().join(format!("local_{name}"));
        let yaml = name.ends_with(".yaml") || name.ends_with(".yml");
        let contents = if yaml {
            "display:\n  header: true\n"
        } else {
            HEADER
        };
        std::fs::create_dir(&local).expect("create the working directory");
        std::fs::write(local.join("a.txt"), "").expect("write a.txt");
        std::fs::write(local.join("b.md"), "").expect("write b.md");
        std::fs::write(local.join(name), contents).expect("write the config");
        assert_eq!(listing(&local, &nowhere, &[], &[]), HEADED, "{name}");
        assert_eq!(
            listing(&local, &nowhere, &["--no-config"], &[]),
            PLAIN,
            "{name}"
        );
    }

    let explicit = dir.create_file("explicit.toml", HEADER.as_bytes());
    assert_eq!(
        listing(
            &work,
            &nowhere,
            &["--config", explicit.to_str().expect("UTF-8")],
            &[]
        ),
        HEADED
    );
    for var in ["LEZ_CONFIG_FILE", "EZA_CONFIG_FILE", "EXA_CONFIG_FILE"] {
        assert_eq!(
            listing(&work, &nowhere, &[], &[(var, &explicit)]),
            HEADED,
            "{var}"
        );
        assert_eq!(
            listing(&work, &nowhere, &["--no-config"], &[(var, &explicit)]),
            PLAIN,
            "{var}"
        );
    }
}

#[test]
fn which_config_file_wins() {
    let dir = crate::common::TempTestDir::new("config_wins");
    let work = work_dir(&dir);
    let global = dir.path().join("global");
    std::fs::create_dir(&global).expect("create the config directory");
    let by_extension_headed = "Name\nb.md\na.txt\n";

    // The first global name there wins over the next.
    std::fs::write(
        global.join("config.toml"),
        format!("{HEADER}{BY_EXTENSION}"),
    )
    .expect("write the config");
    std::fs::write(global.join("lez.toml"), NO_HEADER).expect("write the config");
    assert_eq!(listing(&work, &global, &[], &[]), by_extension_headed);

    // The local file is laid over the global one, key by key.
    std::fs::write(work.join(".lez.toml"), NO_HEADER).expect("write the config");
    std::fs::write(work.join(".eza.toml"), HEADER).expect("write the config");
    assert_eq!(listing(&work, &global, &[], &[]), "b.md\na.txt\n");

    // An explicit file is the only one read, and `--config` beats the
    // variables, which go `LEZ_`, `EZA_`, `EXA_`.
    let headed = dir.create_file("headed.toml", HEADER.as_bytes());
    let sorted = dir.create_file("sorted.toml", BY_EXTENSION.as_bytes());
    let sorted_arg = sorted.to_str().expect("UTF-8");
    assert_eq!(
        listing(&work, &global, &["--config", sorted_arg], &[]),
        "b.md\na.txt\n"
    );
    assert_eq!(
        listing(
            &work,
            &global,
            &["--config", sorted_arg],
            &[("LEZ_CONFIG_FILE", &headed)]
        ),
        "b.md\na.txt\n"
    );
    for (first, second) in [
        ("LEZ_CONFIG_FILE", "EZA_CONFIG_FILE"),
        ("EZA_CONFIG_FILE", "EXA_CONFIG_FILE"),
    ] {
        assert_eq!(
            listing(&work, &global, &[], &[(first, &headed), (second, &sorted)]),
            HEADED,
            "{first} before {second}"
        );
        // An empty variable names no file: the next one is read, and with
        // none the files are discovered as usual, without a word.
        let empty = Path::new("");
        assert_eq!(
            listing(&work, &global, &[], &[(first, empty), (second, &headed)]),
            HEADED,
            "{first} empty"
        );
        assert_eq!(
            listing(&work, &global, &[], &[(first, empty)]),
            "b.md\na.txt\n",
            "{first} empty"
        );
    }

    // The command line beats every file.
    std::fs::write(
        &sorted,
        format!("{BY_EXTENSION}[theme]\ncolor = \"always\"\n"),
    )
    .expect("write the config");
    for flags in [
        &["-s", "name", "--color=never"][..],
        &["-s", "name", "--color=auto"],
    ] {
        assert_eq!(
            listing(
                &work,
                &global,
                &[&["--config", sorted_arg][..], flags].concat(),
                &[]
            ),
            PLAIN,
            "{flags:?}"
        );
    }
}

/// A file given explicitly that cannot be read or parsed is reported, and
/// the listing goes on without it.
#[test]
fn an_explicit_config_that_cannot_be_used_is_reported() {
    let dir = crate::common::TempTestDir::new("config_broken");
    let work = work_dir(&dir);
    let broken = dir.create_file("broken.toml", b"invalid = toml [ broken syntax");
    let missing = dir.path().join("missing.toml");
    let missing_error = std::fs::read_to_string(&missing).expect_err("missing file");
    let run = |config: &Path| {
        let output = crate::common::lez_in(&work)
            .args(crate::common::NAME_COLUMN_ONLY)
            .arg("--config")
            .arg(config)
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&output.stdout), PLAIN);
        String::from_utf8(output.stderr).expect("UTF-8 stderr")
    };
    assert_eq!(
        run(&broken),
        format!(
            "lez: Failed to parse config file {broken:?}: TOML parse error at line 1, column 16\n  \
             |\n\
             1 | invalid = toml [ broken syntax\n  \
             |                ^\n\
             unexpected key or value, expected newline, `#`\n"
        )
    );
    assert_eq!(
        run(&missing),
        format!("lez: Failed to read config file {missing:?}: {missing_error}\n")
    );
}

/// A config file lez finds on its own, in the config directory or the
/// current one, is reported when it does not parse, in the words used for
/// the same file given with `--config`, and the listing goes on without it.
/// Both used to be dropped without a word, the valid setting beside the bad
/// one with them. A directory with a config file's name is passed over.
#[test]
fn a_discovered_config_that_does_not_parse_is_reported() {
    let dir = crate::common::TempTestDir::new("broken_discovered");
    let broken = b"[display]\nheader = true\nsize_digits = 300\n";
    dir.create_file("global/config.toml", broken);
    // Joined a part at a time, as lez joins it, so the separators match on
    // Windows.
    let global = dir.path().join("global").join("config.toml");
    dir.create_file("work/.lez.toml", broken);
    dir.create_file("work/file.txt", b"x");
    dir.create_dir("other/.lez.toml");
    dir.create_file("other/file.txt", b"x");
    let local = std::path::Path::new(".").join(".lez.toml");

    let run = |cwd: &str, config_dir: &std::path::Path, explicit: Option<&std::path::Path>| {
        let mut cmd = crate::common::lez_in(&dir.path().join(cwd));
        cmd.env("LEZ_CONFIG_DIR", config_dir);
        if let Some(path) = explicit {
            cmd.arg("--config").arg(path);
        }
        let output = cmd
            .args([
                "-l",
                "--no-permissions",
                "--no-user",
                "--no-time",
                "file.txt",
            ])
            .output()
            .expect("run lez");
        assert_eq!(output.status.code(), Some(0), "{cwd}");
        (
            String::from_utf8(output.stdout).expect("UTF-8 stdout"),
            String::from_utf8(output.stderr).expect("UTF-8 stderr"),
        )
    };
    let nowhere = dir.path().join("nowhere");

    for (cwd, config_dir, path) in [
        ("other", dir.path().join("global"), global.as_path()),
        ("work", nowhere.clone(), local.as_path()),
    ] {
        let (stdout, stderr) = run(cwd, &config_dir, None);
        assert_eq!(
            (stdout.clone(), stderr.clone()),
            run(cwd, &nowhere, Some(path)),
            "{cwd}"
        );
        assert_eq!(stdout, "1 file.txt\n", "{cwd}");
        assert_eq!(
            stderr,
            format!(
                "lez: Failed to parse config file {path:?}: TOML parse error at line 3, column 15\n  \
                 |\n\
                 3 | size_digits = 300\n  \
                 |               ^^^\n\
                 invalid value: integer `300`, expected u8\n"
            ),
            "{cwd}"
        );
    }

    assert_eq!(
        run("other", &nowhere, None),
        ("1 file.txt\n".to_owned(), String::new())
    );
}

/// Each key of the config file does what its flag does: the listing with
/// the key set is the listing with the flag given, and differs from the
/// one with neither, so a key that is read but goes nowhere is caught.
/// `absolute` was such a key: `--absolute` has a default of its own, which
/// stood in front of the config file's. `icons.spacing` has no flag and is
/// compared with `LEZ_ICON_SPACING`, and `loc.sub_files` with its other
/// values. Unix only: several keys need a link, a mode or a group.
#[cfg(unix)]
#[test]
fn every_config_key_does_what_its_flag_does() {
    crate::common::require_git();
    let repo = crate::common::TempGitRepo::new("config_keys");
    repo.create_file("a.rs", b"fn main() {}\n");
    repo.create_file("big.bin", &[0; 3000]);
    repo.create_file("b.txt", b"x\n");
    repo.create_file(".hidden", b"h\n");
    repo.create_file("sub/inner.txt", b"i\n");
    repo.create_file("space name.txt", b"");
    repo.create_file(".gitignore", b"b.txt\n");
    repo.create_file("notes.md", b"# Notes\n\n```rust\nfn x() {}\n```\n");
    repo.create_symlink("b.txt", "link");
    repo.git(&["add", "a.rs"]);
    let config = crate::common::TempTestDir::new("config_keys_file");
    let path = config.path().join("config.toml");
    let path_arg = path.to_str().expect("UTF-8 path");

    let run = |args: &[&str], envs: &[(&str, &str)]| {
        let mut cmd = crate::common::lez_in(repo.path());
        for (key, value) in envs {
            cmd.env(key, value);
        }
        crate::common::success_stdout(cmd.args(args))
    };
    let long = [
        "-l",
        "--no-permissions",
        "--no-filesize",
        "--no-user",
        "--no-time",
    ];
    let sized = ["-l", "--no-permissions", "--no-user", "--no-time"];
    let coloured = [
        "-l",
        "--no-permissions",
        "--no-user",
        "--no-time",
        "--color=always",
    ];
    let scaled = [&coloured[..], &["--color-scale=size"]].concat();
    let long_loc = [&long[..], &["--loc"]].concat();
    let long_percent = [&long[..], &["--loc=percent"]].concat();
    let long_git = [&long[..], &["--git"]].concat();
    let long_repos = [&long[..], &["-d", "sub", "."]].concat();
    let numeric = ["-l", "--no-permissions", "--no-filesize", "--no-time"];
    let octal = ["-l", "--no-filesize", "--no-user", "--no-time"];
    let timed = ["-l", "--no-permissions", "--no-filesize", "--no-user"];

    let cases: [(&str, &[&str], &[&str]); 40] = [
        ("[display]\nmode = \"tree\"", &[], &["--tree"]),
        ("[display]\nmode = \"long\"", &[], &["-l"]),
        ("[display]\nmode = \"lines\"", &["--width=200"], &["-1"]),
        ("[display]\nmode = \"grid\"", &[], &["--grid"]),
        ("[display]\nmode = \"code\"", &[], &["--code"]),
        ("[display]\nheader = true", &long, &["-h"]),
        ("[display]\ngroup = true", &long, &["-g"]),
        ("[display]\nnumeric = true", &numeric, &["-n"]),
        ("[display]\nlinks = true", &long, &["-H"]),
        ("[display]\ninode = true", &long, &["-i"]),
        ("[display]\nblocksize = true", &long, &["-S"]),
        ("[display]\nblocks = true", &long, &["--blocks"]),
        ("[display]\ntotal_size = true", &sized, &["--total-size"]),
        ("[display]\nsize_digits = 4", &sized, &["--size-digits=4"]),
        (
            "[display]\ntime_style = \"long-iso\"",
            &timed,
            &["--time-style=long-iso"],
        ),
        ("[display]\noctal_permissions = true", &octal, &["-o"]),
        ("[display]\ndereference = true", &long, &["-X"]),
        ("[display]\nfile_flags = true", &long, &["-O"]),
        ("[display]\nsmart_group = true", &long, &["--smart-group"]),
        ("[display]\nabsolute = \"on\"", &["-1"], &["--absolute=on"]),
        (
            "[display]\nhyperlink = \"always\"",
            &["-1"],
            &["--hyperlink=always"],
        ),
        (
            "[display]\nquotes = \"never\"",
            &["-1"],
            &["--quotes=never"],
        ),
        ("[display]\nlanguage = false", &long_loc, &["--no-language"]),
        ("[filter]\nall = true", &["-1"], &["-a"]),
        ("[filter]\nalmost_all = true", &["-1"], &["-A"]),
        ("[filter]\nonly_dirs = true", &["-1"], &["-D"]),
        ("[filter]\nonly_files = true", &["-1"], &["-f"]),
        (
            "[filter]\nshow_dotfiles = true",
            &["-1"],
            &["--show-dotfiles"],
        ),
        (
            "[filter]\nignore_globs = [\"*.txt\"]",
            &["-1"],
            &["-I", "*.txt"],
        ),
        ("[filter]\ngit_ignore = true", &["-1"], &["--git-ignore"]),
        ("[filter]\nsort = \"size\"", &["-1"], &["-s", "size"]),
        ("[filter]\nreverse = true", &["-1"], &["-r"]),
        ("[filter]\nlevel = 1", &["-T"], &["-L1"]),
        ("[git]\ngit = true", &long, &["--git"]),
        ("[git]\ngit_glyphs = true", &long_git, &["--git-glyphs"]),
        ("[git]\ngit_repos = true", &long_repos, &["--git-repos"]),
        (
            "[git]\ngit_repos_no_status = true",
            &long_repos,
            &["--git-repos-no-status"],
        ),
        ("[icons]\nicons = \"always\"", &["-1"], &["--icons=always"]),
        ("[theme]\ncolor = \"always\"", &["-1"], &["--color=always"]),
        (
            "[theme]\ncolor_scale = \"size\"",
            &coloured,
            &["--color-scale=size"],
        ),
    ];
    let more: [(&str, &[&str], &[&str]); 2] = [
        (
            "[theme]\ncolor_scale_mode = \"fixed\"",
            &scaled,
            &["--color-scale-mode=fixed"],
        ),
        (
            "[loc]\npercent_digits = 3",
            &long_percent,
            &["--percent-digits=3"],
        ),
    ];
    // The security context column exists on Linux alone.
    #[cfg(target_os = "linux")]
    let platform: &[(&str, &[&str], &[&str])] =
        &[("[display]\nsecurity_context = true", &long, &["-Z"])];
    #[cfg(not(target_os = "linux"))]
    let platform: &[(&str, &[&str], &[&str])] = &[];
    for (toml, base, flags) in cases
        .into_iter()
        .chain(more)
        .chain(platform.iter().copied())
    {
        std::fs::write(&path, format!("{toml}\n")).expect("write the config");
        let from_flags = run(&[base, flags].concat(), &[]);
        assert_eq!(
            run(&[&["--config", path_arg][..], base].concat(), &[]),
            from_flags,
            "{toml}"
        );
        assert_ne!(from_flags, run(base, &[]), "{toml} changes nothing");
    }

    // `extended` needs an attribute to show; set last, so it cannot touch
    // the cases above.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if crate::common::set_xattr(&repo.path().join("a.rs"), b"v") {
        std::fs::write(&path, "[display]\nextended = true\n").expect("write the config");
        let from_flag = run(&[&octal[..], &["-@"]].concat(), &[]);
        assert_eq!(
            run(&[&["--config", path_arg][..], &octal].concat(), &[]),
            from_flag
        );
        assert_ne!(from_flag, run(&octal, &[]));
    }

    std::fs::write(&path, "[icons]\nspacing = 3\n").expect("write the config");
    let spaced = run(&["-1", "--icons=always"], &[("LEZ_ICON_SPACING", "3")]);
    assert_eq!(
        run(&["--config", path_arg, "-1", "--icons=always"], &[]),
        spaced
    );
    assert_ne!(run(&["-1", "--icons=always"], &[]), spaced);

    let sub_files: Vec<String> = ["symbol", "count", "blank"]
        .iter()
        .map(|mode| {
            std::fs::write(&path, format!("[loc]\nsub_files = \"{mode}\"\n"))
                .expect("write the config");
            run(&["--config", path_arg, "--code=lines"], &[])
        })
        .collect();
    assert_eq!(
        sub_files[0],
        run(&["--code=lines"], &[]),
        "symbol is the default"
    );
    assert_ne!(sub_files[0], sub_files[1]);
    assert_ne!(sub_files[1], sub_files[2]);
    assert_ne!(sub_files[0], sub_files[2]);
}
