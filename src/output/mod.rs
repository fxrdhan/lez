// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
#[cfg(target_os = "windows")]
pub use self::cell::TextCell;
pub use self::escape::escape;

pub mod code;
pub mod color_scale;
pub mod details;
pub mod file_name;
pub mod grid;
pub mod grid_details;
pub mod icons;
pub mod lines;
pub mod render;
pub mod table;
pub mod time;

mod cell;
mod escape;
mod tree;

/// The **view** contains all information about how to format output.
#[derive(Debug)]
pub struct View {
    pub mode: Mode,
    pub width: TerminalWidth,
    pub file_style: file_name::Options,
    pub deref_links: bool,
    pub follow_links: bool,
    pub total_size: bool,
}

/// The **mode** is the “type” of output.
#[derive(PartialEq, Eq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Mode {
    Grid(grid::Options),
    Details(details::Options),
    GridDetails(grid_details::Options),
    Lines,
    /// The `--code` lines-of-code summary, which lists languages rather than
    /// files.
    Code(code::Options),
}

/// The width of the terminal requested by the user.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum TerminalWidth {
    /// The user requested this specific number of columns.
    Set(usize),

    /// Look up the terminal size at runtime.
    Automatic,
}

impl TerminalWidth {
    #[must_use]
    pub fn actual_terminal_width(self) -> Option<usize> {
        // All of stdin, stdout, and stderr could not be connected to a
        // terminal, but we’re only interested in stdout because it’s
        // where the output goes.

        #[cfg(unix)]
        let stdout_term_width = {
            terminal_size::terminal_size_of(std::io::stdout())
                .map(|(w, _h)| (w.0 as usize).clamp(1, u16::MAX as usize))
        };
        #[cfg(windows)]
        let stdout_term_width = {
            use std::os::windows::io::BorrowedHandle;
            use windows_sys::Win32::System::Console::{GetStdHandle, STD_OUTPUT_HANDLE};
            terminal_size::terminal_size_of(unsafe {
                BorrowedHandle::borrow_raw(GetStdHandle(STD_OUTPUT_HANDLE))
            })
            .map(|(w, _h)| (w.0 as usize).clamp(1, u16::MAX as usize))
        };

        match self {
            Self::Set(width) => Some(width.clamp(1, u16::MAX as usize)),
            Self::Automatic => stdout_term_width,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn actual_terminal_width_set_normal() {
        assert_eq!(TerminalWidth::Set(80).actual_terminal_width(), Some(80));
    }

    #[test]
    fn actual_terminal_width_set_clamped_min() {
        assert_eq!(TerminalWidth::Set(0).actual_terminal_width(), Some(1));
    }

    #[test]
    fn actual_terminal_width_set_clamped_max() {
        assert_eq!(
            TerminalWidth::Set(100_000).actual_terminal_width(),
            Some(u16::MAX as usize)
        );
        assert_eq!(
            TerminalWidth::Set(usize::MAX).actual_terminal_width(),
            Some(u16::MAX as usize)
        );
    }
}
