// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::process::Command;

use crate::common::{TempTestDir, bin_path};

#[test]
#[cfg(unix)]
fn test_theme_symlink_foreground_target_with_italic() {
    let config_dir = TempTestDir::new("theme_target_italic_cfg");
    let theme_path = config_dir.path().join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filekinds:\n  directory:\n    foreground: Blue\n    is_bold: false\n  symlink:\n    foreground: target\n    is_italic: true"
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("theme_target_italic_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", config_dir.path())
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Should borrow Blue (34) from target directory AND have italic (3)
    assert!(
        stdout.contains("\x1b[3;34m"),
        "Expected symlink to borrow target directory Blue (34) and apply italic (3), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_theme_symlink_attributes_combined_with_lez_colors_ln_target() {
    let config_dir = TempTestDir::new("theme_attr_combined_cfg");
    let theme_path = config_dir.path().join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filekinds:\n  directory:\n    foreground: Blue\n    is_bold: false\n  symlink:\n    is_italic: true"
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("theme_attr_combined_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", config_dir.path())
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "ln=target")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[3;34m"),
        "Expected combined LEZ_COLORS='ln=target' with theme italic to have Blue and italic, got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_lez_colors_ln_target_with_italic_code() {
    let work_dir = TempTestDir::new("env_target_italic_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "di=34:ln=target;3")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[3;34m"),
        "Expected LEZ_COLORS='ln=target;3' to borrow Blue (34) and apply italic (3), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_lez_colors_ln_target_with_bold_underline_codes() {
    let work_dir = TempTestDir::new("env_target_bold_under_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "di=34:ln=target;1;4")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Must contain exact escape \x1b[1;4;34m
    assert!(
        stdout.contains("\x1b[1;4;34m"),
        "Expected LEZ_COLORS='ln=target;1;4' to borrow Blue (34) with bold and underline, got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_broken_symlink_keeps_or_color_unaffected() {
    let config_dir = TempTestDir::new("broken_symlink_unaffected_cfg");
    let theme_path = config_dir.path().join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filekinds:\n  symlink:\n    foreground: target\n    is_italic: true"
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("broken_symlink_unaffected_work");
    let broken_link = work_dir.path().join("broken_orphan_link");
    std::os::unix::fs::symlink("/nonexistent_path_target_does_not_exist", &broken_link)
        .expect("create broken symlink");

    // Test with explicit or=31 (Red) in LEZ_COLORS
    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", config_dir.path())
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "or=31")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&broken_link)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Broken symlink must retain or= color (31), and NOT have italic (3)
    assert!(
        stdout.contains("\x1b[31m"),
        "Expected broken symlink to retain or=31 Red color, got: {stdout:?}"
    );
    assert!(
        !stdout.contains("\x1b[3;") && !stdout.contains(";3m") && !stdout.contains("\x1b[3m"),
        "Broken symlink must remain unaffected by target italic styling, but got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_theme_symlink_bare_target_string() {
    let config_dir = TempTestDir::new("theme_bare_target_cfg");
    let theme_path = config_dir.path().join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filekinds:\n  directory:\n    foreground: Blue\n    is_bold: false\n  symlink: target"
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("theme_bare_target_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", config_dir.path())
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[34m"),
        "Expected symlink with 'symlink: target' to borrow Blue (34), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_theme_symlink_target_with_ansi_string() {
    let config_dir = TempTestDir::new("theme_ansi_target_cfg");
    let theme_path = config_dir.path().join("theme.yml");
    let mut f = StdFile::create(&theme_path).expect("create theme.yml");
    writeln!(
        f,
        "filekinds:\n  directory:\n    foreground: Blue\n    is_bold: false\n  symlink: \"target;3\""
    )
    .expect("write theme.yml");

    let work_dir = TempTestDir::new("theme_ansi_target_work");
    let target_dir = work_dir.path().join("my_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_path = work_dir.path().join("link_to_dir");
    std::os::unix::fs::symlink(&target_dir, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .env("LEZ_CONFIG_DIR", config_dir.path())
        .env_remove("EZA_CONFIG_DIR")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LS_COLORS")
        .env_remove("LEZ_COLORS")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[3;34m"),
        "Expected symlink with 'symlink: \"target;3\"' to borrow Blue (34) and have italic (3), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_symlink_to_file_inherits_extension_color_and_applies_style() {
    let work_dir = TempTestDir::new("symlink_to_file_work");
    let target_file = work_dir.path().join("lib.rs");
    fs::write(&target_file, "// rust code\n").expect("write target file");
    let link_path = work_dir.path().join("link_to_rs");
    std::os::unix::fs::symlink(&target_file, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "*.rs=35:ln=target;3")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[3;35m"),
        "Expected symlink to borrow Purple (35) from *.rs and apply italic (3), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_symlink_to_executable_inherits_exec_color_and_applies_style() {
    use std::os::unix::fs::PermissionsExt;
    let work_dir = TempTestDir::new("symlink_to_exec_work");
    let target_file = work_dir.path().join("script.sh");
    fs::write(&target_file, "#!/bin/sh\n").expect("write target script");
    fs::set_permissions(&target_file, fs::Permissions::from_mode(0o755)).expect("chmod +x");
    let link_path = work_dir.path().join("link_to_exec");
    std::os::unix::fs::symlink(&target_file, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "ex=32:ln=target;4")
        .env_remove("EXA_COLORS")
        .arg("-d")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[4;32m"),
        "Expected symlink to borrow Green (32) from executable and apply underline (4), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_multihop_symlink_inherits_target_color_and_applies_style() {
    let work_dir = TempTestDir::new("multihop_symlink_work");
    let target_dir = work_dir.path().join("final_target_dir");
    fs::create_dir_all(&target_dir).expect("create target dir");
    let link_b = work_dir.path().join("link_b");
    std::os::unix::fs::symlink(&target_dir, &link_b).expect("create link_b");
    let link_a = work_dir.path().join("link_a");
    std::os::unix::fs::symlink(&link_b, &link_a).expect("create link_a");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "di=34:ln=target;3")
        .env_remove("EXA_COLORS")
        .arg("-ld")
        .arg("--color=always")
        .arg(&link_a)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\x1b[3;34mlink_a\x1b[0m"),
        "Expected multihop symlink link_a to borrow Blue (34) and have italic (3), got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_long_details_renders_styled_type_indicator_and_link() {
    let work_dir = TempTestDir::new("long_details_work");
    let target_file = work_dir.path().join("doc_data");
    fs::write(&target_file, "text\n").expect("write target");
    let link_path = work_dir.path().join("link_to_doc");
    std::os::unix::fs::symlink(&target_file, &link_path).expect("create symlink");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "fi=33:ln=target;3")
        .env_remove("EXA_COLORS")
        .arg("-l")
        .arg("--color=always")
        .arg(&link_path)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Indicator "l" should borrow normal fi colour (33) and be italic (\x1b[3;33ml\x1b[0m)
    assert!(
        stdout.contains("\x1b[3;33ml\x1b[0m"),
        "Expected filetype indicator 'l' to borrow normal colour and be italic in long details, got: {stdout:?}"
    );
    // Link name should be yellow and italic (\x1b[3;33mlink_to_doc\x1b[0m)
    assert!(
        stdout.contains("\x1b[3;33mlink_to_doc\x1b[0m"),
        "Expected symlink name to be yellow (33) and italic (3), got: {stdout:?}"
    );
    // Target after -> should be yellow without italic (\x1b[33mdoc_data\x1b[0m)
    assert!(
        stdout.contains("\x1b[33mdoc_data\x1b[0m"),
        "Expected target name to be yellow without symlink italic style, got: {stdout:?}"
    );
}

#[test]
#[cfg(unix)]
fn test_symlink_loop_cycle_falls_back_to_broken_symlink_color() {
    let work_dir = TempTestDir::new("loop_cycle_work");
    let link_1 = work_dir.path().join("loop_link_1");
    let link_2 = work_dir.path().join("loop_link_2");
    std::os::unix::fs::symlink(&link_2, &link_1).expect("create link_1");
    std::os::unix::fs::symlink(&link_1, &link_2).expect("create link_2");

    let out = Command::new(bin_path())
        .arg("--no-config")
        .env_remove("LS_COLORS")
        .env("LEZ_COLORS", "or=31:ln=target;3")
        .env_remove("EXA_COLORS")
        .arg("-ld")
        .arg("--color=always")
        .arg(&link_1)
        .output()
        .expect("run lez");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Loop cycle must not infinite-loop or panic, and should fall back to or=31
    assert!(
        stdout.contains("\x1b[31mloop_link_1\x1b[0m"),
        "Expected cyclic symlink to fall back to broken symlink colour Red (31), got: {stdout:?}"
    );
}
