// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

use std::fs::{self, File as StdFile};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsr_chal_m3_{prefix}_{}_{}",
            std::process::id(),
            nanos
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn create_file(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.path.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = StdFile::create(&p).unwrap();
        f.write_all(content).unwrap();
        p
    }

    fn create_dir(&self, name: &str) -> PathBuf {
        let p = self.path.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_lsr")
}

// -----------------------------------------------------------------------------
// 1. Alias Simulation Tests
// -----------------------------------------------------------------------------

#[test]
fn test_alias_sim_ll_with_smart_group() {
    // Alias simulation: user aliases `ll="lsr -l"` and runs `ll --smart-group`
    let temp = TempTestDir::new("alias_ll");
    temp.create_file("sample.txt", b"content");

    let output = Command::new(bin_path())
        .arg("-l") // from alias
        .arg("--smart-group") // user argument
        .arg(&temp.path)
        .output()
        .expect("Failed to execute lsr");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sample.txt"));
}

#[test]
fn test_alias_sim_lsg_with_flag_overrides() {
    // Alias simulation: user aliases `lsg="lsr -l --smart-group"`
    // and tests appending various overriding/modifying flags
    let temp = TempTestDir::new("alias_lsg");
    temp.create_file("alpha.txt", b"alpha data");
    temp.create_file("beta.doc", b"beta data");

    // Case 1: lsg -g (explicit group added to smart-group)
    let out_g = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("-g")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_g.status.success());

    // Case 2: lsg --no-user (suppress owner user column while smart-group is active)
    let out_no_user = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("--no-user")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_no_user.status.success());
    let stdout_no_user = String::from_utf8_lossy(&out_no_user.stdout);
    assert!(stdout_no_user.contains("alpha.txt"));

    // Case 3: lsg --no-permissions
    let out_no_perm = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("--no-permissions")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_no_perm.status.success());

    // Case 4: lsg --no-filesize
    let out_no_sz = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("--no-filesize")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_no_sz.status.success());

    // Case 5: lsg --no-time
    let out_no_time = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("--no-time")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_no_time.status.success());

    // Case 6: lsg -1 (one-line mode overrides details mode)
    let out_oneline = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("-1")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_oneline.status.success());
    let stdout_oneline = String::from_utf8_lossy(&out_oneline.stdout);
    assert!(stdout_oneline.contains("alpha.txt"));
    assert!(stdout_oneline.contains("beta.doc"));

    // Case 7: lsg --grid (grid mode overrides details mode)
    let out_grid = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg("--grid")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_grid.status.success());
}

#[test]
fn test_alias_sim_ls_bare_smart_group() {
    // Alias simulation: user aliases `ls="lsr --smart-group"`
    let temp = TempTestDir::new("alias_ls");
    temp.create_file("one.txt", b"one");
    temp.create_file("two.txt", b"two");

    // Case 1: bare `ls` (default grid mode without -l)
    let out_bare = Command::new(bin_path())
        .arg("--smart-group")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_bare.status.success());
    let stdout_bare = String::from_utf8_lossy(&out_bare.stdout);
    assert!(stdout_bare.contains("one.txt"));
    assert!(stdout_bare.contains("two.txt"));

    // Case 2: `ls -l` (long mode added dynamically)
    let out_long = Command::new(bin_path())
        .arg("--smart-group")
        .arg("-l")
        .arg(&temp.path)
        .output()
        .expect("Failed to execute");
    assert!(out_long.status.success());
    let stdout_long = String::from_utf8_lossy(&out_long.stdout);
    assert!(stdout_long.contains("one.txt"));
}

// -----------------------------------------------------------------------------
// 2. Flag Precedence, Combinations, and Order Permutations
// -----------------------------------------------------------------------------

#[test]
fn test_flag_order_permutations_smart_group_and_g() {
    let temp = TempTestDir::new("order_perm");
    temp.create_file("test.txt", b"data");

    // Permutation 1: -l -g --smart-group
    let out1 = Command::new(bin_path())
        .args(["-l", "-g", "--smart-group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out1.status.success());

    // Permutation 2: -l --smart-group -g
    let out2 = Command::new(bin_path())
        .args(["-l", "--smart-group", "-g"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out2.status.success());

    // Permutation 3: -l --group --smart-group
    let out3 = Command::new(bin_path())
        .args(["-l", "--group", "--smart-group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out3.status.success());

    // Permutation 4: -l --smart-group --group
    let out4 = Command::new(bin_path())
        .args(["-l", "--smart-group", "--group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out4.status.success());

    // Outputs of all 4 should succeed and produce valid listings
    assert_eq!(out1.status.code(), Some(0));
    assert_eq!(out2.status.code(), Some(0));
    assert_eq!(out3.status.code(), Some(0));
    assert_eq!(out4.status.code(), Some(0));
}

#[test]
fn test_smart_group_with_numeric_and_octal() {
    let temp = TempTestDir::new("num_oct");
    temp.create_file("item.bin", b"\x00\x01\x02");

    let out = Command::new(bin_path())
        .args(["-l", "--smart-group", "--numeric", "--octal-permissions"])
        .arg(&temp.path)
        .output()
        .expect("Failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("item.bin"));
}

#[test]
fn test_smart_group_with_header_alignment() {
    let temp = TempTestDir::new("header_align");
    temp.create_file("file_a.txt", b"a");
    temp.create_file("file_b.txt", b"bb");
    temp.create_file("file_c.txt", b"ccc");

    // Case 1: -l -h (without smart group)
    let out_no_sg = Command::new(bin_path())
        .args(["-l", "--header", "--color=never", "--icons=never"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out_no_sg.status.success());
    let _stdout_no_sg = String::from_utf8_lossy(&out_no_sg.stdout);

    // Case 2: -l -h --smart-group
    let out_sg = Command::new(bin_path())
        .args([
            "-l",
            "--header",
            "--smart-group",
            "--color=never",
            "--icons=never",
        ])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out_sg.status.success());
    let _stdout_sg = String::from_utf8_lossy(&out_sg.stdout);

    // Header in smart-group mode must contain "Group"
    #[cfg(unix)]
    {
        assert!(
            _stdout_sg.contains("Group"),
            "Header must contain 'Group' when --smart-group is active:\n{}",
            _stdout_sg
        );
        // Header without -g / --smart-group must NOT contain "Group"
        let first_line_no_sg = _stdout_no_sg.lines().next().unwrap_or_default();
        assert!(
            !first_line_no_sg.contains("Group"),
            "Plain -l --header must NOT contain 'Group' header column:\n{}",
            first_line_no_sg
        );

        // Verify column count alignment between header and data rows
        let lines: Vec<&str> = _stdout_sg.lines().collect();
        assert!(lines.len() >= 4, "Expected at least header + 3 files");
        let header_cols: Vec<&str> = lines[0].split_whitespace().collect();
        assert!(
            header_cols.contains(&"Group"),
            "Header columns must include Group: {:?}",
            header_cols
        );

        for line in &lines[1..] {
            let data_cols: Vec<&str> = line.split_whitespace().collect();
            // Permissions, Size, User, Group, Date, Time/Year, Filename
            // Number of whitespace-separated tokens should match expected long columns
            assert!(
                data_cols.len() >= 6,
                "Data line has too few columns: {:?}",
                line
            );
        }
    }
}

#[test]
fn test_smart_group_with_time_styles() {
    let temp = TempTestDir::new("time_styles");
    temp.create_file("clock.txt", b"tick");

    for style in &[
        "default",
        "iso",
        "long-iso",
        "full-iso",
        "relative",
        "relative-recent",
    ] {
        let out = Command::new(bin_path())
            .args(["-l", "--smart-group", &format!("--time-style={style}")])
            .arg(&temp.path)
            .output()
            .expect("Failed");

        assert!(out.status.success(), "Failed for time-style={style}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("clock.txt"));
    }
}

#[test]
fn test_smart_group_with_tree_and_level() {
    let temp = TempTestDir::new("tree_sg");
    temp.create_file("root.txt", b"root");
    let sub = temp.create_dir("subdir");
    let mut f = StdFile::create(sub.join("child.txt")).unwrap();
    f.write_all(b"child").unwrap();

    let out = Command::new(bin_path())
        .args(["-l", "--smart-group", "--tree", "--level=2"])
        .arg(&temp.path)
        .output()
        .expect("Failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("root.txt"));
    assert!(stdout.contains("subdir"));
    assert!(stdout.contains("child.txt"));
}

#[test]
fn test_smart_group_json_consistency() {
    let temp = TempTestDir::new("json_sg");
    temp.create_file("entry1.txt", b"data1");
    temp.create_file("entry2.txt", b"data2");

    let out = Command::new(bin_path())
        .args(["-l", "--smart-group", "--json"])
        .arg(&temp.path)
        .output()
        .expect("Failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output must be valid JSON");
    let _map = parsed.as_object().expect("Expected JSON map");

    #[cfg(unix)]
    {
        for (filename, val) in _map {
            let meta = val.as_object().expect("File metadata object");
            assert!(
                meta.contains_key("Group"),
                "File {filename} missing 'Group' field under --smart-group --json"
            );
        }
    }
}

#[test]
fn test_smart_group_empty_directory() {
    let temp = TempTestDir::new("empty_dir");

    let out = Command::new(bin_path())
        .args(["-l", "--smart-group", "--header"])
        .arg(&temp.path)
        .output()
        .expect("Failed");

    assert!(out.status.success());
}

#[test]
fn test_smart_group_special_filenames() {
    let temp = TempTestDir::new("special_names");
    temp.create_file("file with spaces.txt", b"spaces");
    temp.create_file("file-with-dashes.log", b"dashes");
    temp.create_file("file_with_underscores.rs", b"code");
    temp.create_file("ümlaut-fïle.txt", b"unicode");

    let out = Command::new(bin_path())
        .args(["-l", "--smart-group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("file with spaces.txt"));
    assert!(stdout.contains("file-with-dashes.log"));
    assert!(stdout.contains("file_with_underscores.rs"));
    assert!(stdout.contains("ümlaut-fïle.txt"));
}

#[test]
fn test_smart_group_strict_mode_env() {
    let temp = TempTestDir::new("strict_mode");
    temp.create_file("file.txt", b"test");

    // EZA_STRICT=1 with -l --smart-group should succeed
    let out_eza = Command::new(bin_path())
        .env("EZA_STRICT", "1")
        .args(["-l", "--smart-group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out_eza.status.success());

    // EXA_STRICT=1 with -l --smart-group should succeed
    let out_exa = Command::new(bin_path())
        .env("EXA_STRICT", "1")
        .args(["-l", "--smart-group"])
        .arg(&temp.path)
        .output()
        .expect("Failed");
    assert!(out_exa.status.success());
}

#[test]
fn test_smart_group_symlinks_and_dereference() {
    let temp = TempTestDir::new("symlinks");
    let _target = temp.create_file("target.txt", b"target payload");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_path = temp.path.join("link_to_target");
        let _ = symlink(&_target, &link_path);

        // Run with -l --smart-group
        let out_sym = Command::new(bin_path())
            .args(["-l", "--smart-group"])
            .arg(&temp.path)
            .output()
            .expect("Failed");
        assert!(out_sym.status.success());
        let stdout_sym = String::from_utf8_lossy(&out_sym.stdout);
        assert!(stdout_sym.contains("link_to_target"));

        // Run with -l --smart-group --dereference (-L)
        let out_deref = Command::new(bin_path())
            .args(["-l", "--smart-group", "--dereference"])
            .arg(&temp.path)
            .output()
            .expect("Failed");
        assert!(out_deref.status.success());
        let stdout_deref = String::from_utf8_lossy(&out_deref.stdout);
        assert!(stdout_deref.contains("link_to_target"));
    }
}

#[test]
fn test_smart_group_with_git_integration() {
    let temp = TempTestDir::new("git_smart_group");
    // Initialize git repo in temp dir
    let init_out = Command::new("git")
        .arg("init")
        .arg(&temp.path)
        .output()
        .expect("Failed to git init");
    if init_out.status.success() {
        temp.create_file("tracked.rs", b"pub fn hello() {}");
        let _ = Command::new("git")
            .current_dir(&temp.path)
            .args(["add", "tracked.rs"])
            .output();

        let out = Command::new(bin_path())
            .args(["-l", "--smart-group", "--git"])
            .arg(&temp.path)
            .output()
            .expect("Failed");
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("tracked.rs"));
    }
}
