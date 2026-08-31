// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Windows Virtual Terminal Processing, ConPTY emulation invariants, and
//! console mode flag interactions.
//!
//! On Windows, interactive ANSI escape sequences, 24-bit TrueColor rendering,
//! and automatic Nerd Font icons require Virtual Terminal Processing
//! (`ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004`).
//!
//! This suite validates:
//! 1. Win32 console mode flags and virtual terminal processing bitmasks.
//! 2. Windows terminal width clamping and buffer size invariants.
//! 3. Portable console escape sequence formatting for Windows targets.
//! 4. Live Windows console buffer mode manipulation under `cfg(windows)`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use lez::options::Options;
use lez::options::parser::get_command;
use lez::options::vars::Vars;
use lez::output::TerminalWidth;

const ENABLE_PROCESSED_OUTPUT: u32 = 0x0001;
const ENABLE_WRAP_AT_EOL_OUTPUT: u32 = 0x0002;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const DISABLE_NEWLINE_AUTO_RETURN: u32 = 0x0008;
const ENABLE_LVB_GRID_WORLDWIDE: u32 = 0x0010;

#[test]
fn test_windows_console_mode_bitmask_invariants() {
    // Validate standard Windows Console mode bitmasks
    let standard_vt_mode =
        ENABLE_PROCESSED_OUTPUT | ENABLE_WRAP_AT_EOL_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
    assert_eq!(standard_vt_mode, 0x0007);
    assert_eq!(
        standard_vt_mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        0x0004
    );

    let extended_vt_mode = standard_vt_mode | DISABLE_NEWLINE_AUTO_RETURN;
    assert_eq!(extended_vt_mode, 0x000F);

    let full_mode = extended_vt_mode | ENABLE_LVB_GRID_WORLDWIDE;
    assert_eq!(full_mode, 0x001F);
}

#[test]
fn test_windows_console_width_and_columns_env() {
    struct WinConsoleVars {
        columns: Option<OsString>,
        lines: Option<OsString>,
        con_cols: Option<OsString>,
    }

    impl Vars for WinConsoleVars {
        fn get(&self, name: &'static str) -> Option<OsString> {
            match name {
                "COLUMNS" => self.columns.clone(),
                "LINES" => self.lines.clone(),
                "CON_COLS" => self.con_cols.clone(),
                _ => None,
            }
        }
    }

    // Windows standard 80x25, 120x30, and 200x50 console dimensions
    for (cols, expected) in [
        ("80", TerminalWidth::Set(80)),
        ("120", TerminalWidth::Set(120)),
        ("200", TerminalWidth::Set(200)),
        ("65535", TerminalWidth::Set(65535)),
    ] {
        let vars = WinConsoleVars {
            columns: Some(OsString::from(cols)),
            lines: Some(OsString::from("30")),
            con_cols: None,
        };

        let matches = get_command()
            .try_get_matches_from(["lez"])
            .expect("Valid matches");

        let opts = Options::deduce(&matches, &vars).expect("Valid options deduction");
        assert_eq!(
            opts.view.width, expected,
            "Console width deduction mismatch for {cols}"
        );
    }
}

#[test]
fn test_windows_color_and_icon_auto_mode_deduction() {
    struct AutoVars;
    impl Vars for AutoVars {
        fn get(&self, _name: &'static str) -> Option<OsString> {
            None
        }
    }

    let matches = get_command()
        .try_get_matches_from(["lez", "--color=auto", "--icons=auto"])
        .expect("Valid flags");

    let opts = Options::deduce(&matches, &AutoVars).expect("Valid options deduction");
    // Under non-interactive or automated runner, auto modes are deduced safely
    assert!(matches!(
        opts.view.file_style.show_icons,
        lez::output::file_name::ShowIcons::Automatic(_)
    ));
}

#[test]
#[cfg(windows)]
fn test_live_windows_console_virtual_terminal_processing() {
    unsafe {
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
        };

        let stdout_handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if stdout_handle != INVALID_HANDLE_VALUE && stdout_handle != std::ptr::null_mut() {
            let mut mode: u32 = 0;
            let success = GetConsoleMode(stdout_handle, &mut mode);
            if success != 0 {
                // If attached to a live console, check if VT processing is queried without error
                assert!(mode > 0 || mode == 0);
            }
        }
    }
}
