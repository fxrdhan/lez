// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use nu_ansi_term::Style;
use std::ffi::CStr;

#[cfg(target_os = "netbsd")]
use std::ffi::CString;

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::output::table::FlagsFormat;

#[cfg(not(target_os = "netbsd"))]
unsafe extern "C" {
    fn fflagstostr(flags: libc::c_ulong) -> *const libc::c_char;
}

#[cfg(target_os = "netbsd")]
unsafe extern "C" {
    fn flags_to_string(flags: libc::c_ulong, def: *const libc::c_char) -> *const libc::c_char;
}

/// Wrapper around the C library call fflagstostr or the netbsd equivalent
/// If returned string is NULL or empty a "-" is returned
fn wrapper_flags_to_string(flags: f::flag_t) -> String {
    #[cfg(target_os = "netbsd")]
    let empty_string = CString::new("").expect("This string is always valid");

    // SAFETY: Calling external "C" function
    #[cfg(not(target_os = "netbsd"))]
    let flags_c_str = unsafe { fflagstostr(libc::c_ulong::from(flags)) };

    // SAFETY: Calling external "C" function
    #[cfg(target_os = "netbsd")]
    let flags_c_str = unsafe { flags_to_string(libc::c_ulong::from(flags), empty_string.as_ptr()) };

    if flags_c_str.is_null() {
        "-".to_string()
    } else {
        let flags_str = unsafe { CStr::from_ptr(flags_c_str) };
        let flags = flags_str
            .to_str()
            .map_or("-", |s| if s.is_empty() { "-" } else { s })
            .to_string();

        // SAFETY: Calling external "C" function to free memory allocated by fflagstostr
        unsafe {
            libc::free(flags_c_str.cast_mut().cast());
        }

        flags
    }
}

impl f::Flags {
    pub fn render(self, style: Style, _format: FlagsFormat) -> TextCell {
        TextCell::paint(style, wrapper_flags_to_string(self.0))
    }

    pub fn render_json(self, _format: FlagsFormat) -> Option<String> {
        Some(wrapper_flags_to_string(self.0))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zero_flags_render() {
        let flags = f::Flags(0);
        assert_eq!(flags.render_json(FlagsFormat::Short), Some("-".to_string()));
        assert_eq!(flags.render_json(FlagsFormat::Long), Some("-".to_string()));

        let cell = flags.render(Style::default(), FlagsFormat::Short);
        assert_eq!(*cell.width, 1);
    }

    #[test]
    fn test_bsd_flags_render_and_json() {
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            let nodump = f::Flags(libc::UF_NODUMP);
            let s = nodump.render_json(FlagsFormat::Short).unwrap();
            assert!(s.contains("nodump"), "Expected nodump in string, got: {s}");

            let uchg = f::Flags(libc::UF_IMMUTABLE);
            let s_uchg = uchg.render_json(FlagsFormat::Short).unwrap();
            assert!(
                s_uchg.contains("uchg") || s_uchg.contains("uappnd"),
                "Expected uchg in string, got: {s_uchg}"
            );

            let cell = nodump.render(Style::default(), FlagsFormat::Short);
            assert_eq!(*cell.width, s.len());
        }
    }
}
