// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "lez_cfg_test_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp test dir");
        Self { path }
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_explicit_config_file_cli_flag() {
    let temp = TempTestDir::new("explicit_cli");
    let test_file = temp.path.join("file_a.txt");
    fs::write(&test_file, b"content").unwrap();

    let config_path = temp.path.join("my_custom_config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true

[icons]
icons = "never"
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with --config");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via --config: {stdout}"
    );
}

#[test]
fn test_no_config_flag_ignores_config_file() {
    let temp = TempTestDir::new("no_config");
    let test_file = temp.path.join("file_a.txt");
    fs::write(&test_file, b"content").unwrap();

    let config_dir = temp.path.join("config_dir");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    // With LEZ_CONFIG_DIR pointing to config_dir, but with --no-config
    let output = crate::common::lez_cmd()
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .arg("--no-config")
        .arg(&temp.path)
        .output()
        .expect("run lez with --no-config");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Permissions") && !stdout.contains("Size") && !stdout.contains("Name"),
        "Header should NOT be displayed when --no-config is passed: {stdout}"
    );
}

#[test]
fn test_global_config_dir_discovery() {
    let temp = TempTestDir::new("global_discovery");
    let test_file = temp.path.join("sample.txt");
    fs::write(&test_file, b"sample").unwrap();

    let config_dir = temp.path.join("lez_config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with LEZ_CONFIG_DIR");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via LEZ_CONFIG_DIR config.toml: {stdout}"
    );
}

#[test]
fn test_local_directory_lez_toml_overrides_global() {
    let temp = TempTestDir::new("local_override");
    let workdir = temp.path.join("project");
    fs::create_dir_all(&workdir).unwrap();
    fs::write(workdir.join("a.txt"), b"1").unwrap();
    fs::write(workdir.join("b.txt"), b"2").unwrap();

    // Global config: header = false
    let config_dir = temp.path.join("lez_global");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
[display]
header = false
"#,
    )
    .unwrap();

    // Local .lez.toml in workdir: header = true
    fs::write(
        workdir.join(".lez.toml"),
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .current_dir(&workdir)
        .env("LEZ_CONFIG_DIR", &config_dir)
        .arg("-l")
        .output()
        .expect("run lez with local .lez.toml");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Local .lez.toml should enable header overriding global config: {stdout}"
    );
}

#[test]
fn test_cli_argument_overrides_config_file() {
    let temp = TempTestDir::new("cli_precedence");
    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let file_a = temp.path.join("file_a.txt");
    fs::write(&file_a, b"test").unwrap();

    // Config enables header, but CLI explicitly runs without long table (e.g. oneline)
    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg("--oneline")
        .arg(&temp.path)
        .output()
        .expect("run lez with --oneline overriding header");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Permissions") && !stdout.contains("Size"),
        "CLI --oneline should take precedence over config header: {stdout}"
    );
}

#[test]
fn test_env_var_lez_config_file() {
    let temp = TempTestDir::new("env_config_file");
    let config_path = temp.path.join("special_config.toml");
    fs::write(
        &config_path,
        r#"
[display]
header = true
"#,
    )
    .unwrap();

    let file_a = temp.path.join("file.txt");
    fs::write(&file_a, b"test").unwrap();

    let output = crate::common::lez_cmd()
        .env("LEZ_CONFIG_FILE", &config_path)
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("run lez with LEZ_CONFIG_FILE");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Permissions") || stdout.contains("Size") || stdout.contains("Name"),
        "Header should be displayed via LEZ_CONFIG_FILE: {stdout}"
    );
}

#[test]
fn test_malformed_config_file_handled_gracefully() {
    let temp = TempTestDir::new("malformed_config");
    let config_path = temp.path.join("broken_config.toml");
    fs::write(&config_path, b"invalid = toml [ broken syntax").unwrap();

    let file_a = temp.path.join("file.txt");
    fs::write(&file_a, b"test").unwrap();

    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg(&temp.path)
        .output()
        .expect("run lez with malformed config");

    assert!(
        output.status.success(),
        "lez should exit 0 even if config syntax is invalid"
    );
}

#[test]
fn test_config_file_sort_and_quotes_and_absolute() {
    let temp = TempTestDir::new("cfg_sort_quotes_abs");
    let test_file = temp.path.join("spaced file.txt");
    fs::write(&test_file, b"content").unwrap();
    let test_b = temp.path.join("a_file.txt");
    fs::write(&test_b, b"content").unwrap();

    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[filter]
sort = "extension"

[display]
quotes = "always"
absolute = "on"
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg(&temp.path)
        .output()
        .expect("run lez with config");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('\''),
        "Quotes should be enabled via display.quotes = 'always': {stdout}"
    );
}

#[test]
fn test_config_display_mode_tree_recurses() {
    let temp = TempTestDir::new("mode_tree");
    let sub = temp.path.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let nested = sub.join("nested.txt");
    fs::write(&nested, b"nested content").unwrap();

    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
mode = "tree"
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg(&temp.path)
        .output()
        .expect("run lez with config mode=tree");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("nested.txt"),
        "mode = 'tree' in config must recurse into subdirectories: {stdout}"
    );
}

#[test]
fn test_config_display_mode_code_activates() {
    let temp = TempTestDir::new("mode_code");
    let test_rs = temp.path.join("main.rs");
    fs::write(&test_rs, b"fn main() {}\n").unwrap();

    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[display]
mode = "code"
"#,
    )
    .unwrap();

    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg(&temp.path)
        .output()
        .expect("run lez with config mode=code");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Rust") && stdout.contains("Code %"),
        "mode = 'code' in config must produce language code statistics table: {stdout}"
    );
}

#[test]
fn test_cli_color_auto_overrides_config_color_always() {
    let temp = TempTestDir::new("color_precedence");
    let test_file = temp.path.join("file.txt");
    fs::write(&test_file, b"content").unwrap();

    let config_path = temp.path.join("config.toml");
    fs::write(
        &config_path,
        r#"
[theme]
color = "always"
"#,
    )
    .unwrap();

    // Under Command::output(), stdout is a pipe (non-TTY).
    // With --color=auto, colors must be suppressed because stdout is not a TTY,
    // overriding the config file's color = "always".
    let output = crate::common::lez_cmd()
        .arg("--config")
        .arg(&config_path)
        .arg("-l")
        .arg("--color=auto")
        .arg(&temp.path)
        .output()
        .expect("run lez with --color=auto");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\x1b["),
        "--color=auto on non-TTY should not emit ANSI colors, even if config has color='always': {stdout:?}"
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
        assert!(
            stderr.starts_with(&format!(
                "lez: Failed to parse config file {path:?}: TOML parse error at line 3"
            )),
            "{stderr}"
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
