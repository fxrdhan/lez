// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! How the user's locale writes a number: what separates groups of digits,
//! and what comes before a fraction.
//!
//! The `locale` crate asks the C library on Linux. On macOS it reads a file
//! per locale under `/usr/share/locale`, which `C` and `POSIX` do not have,
//! so it grouped digits the English way where the C library groups none;
//! on Windows it reads nothing at all. Both ask the system here instead.
//! A locale the system does not have falls back to English, as on Linux.

use locale::Numeric;

/// The user's way of writing numbers, or English where it cannot be read.
#[must_use]
pub fn user_numeric() -> Numeric {
    system_numeric().unwrap_or_else(Numeric::english)
}

#[cfg(target_os = "macos")]
fn system_numeric() -> Option<Numeric> {
    use std::ffi::{CStr, c_char};

    /// A separator, as long as it is text.
    ///
    /// # Safety
    ///
    /// `p` is null or points to a NUL-terminated string.
    unsafe fn text(p: *const c_char) -> Option<String> {
        if p.is_null() {
            return None;
        }
        // SAFETY: as the caller promises.
        let text = unsafe { CStr::from_ptr(p) };
        text.to_str().ok().map(str::to_owned)
    }

    // SAFETY: an empty name asks for the locale the environment sets, and a
    // null base makes a new locale object, freed below.
    let locale =
        unsafe { libc::newlocale(libc::LC_NUMERIC_MASK, c"".as_ptr(), std::ptr::null_mut()) };
    if locale.is_null() {
        return None;
    }
    // SAFETY: `locale` is valid until it is freed, and the strings it
    // describes live as long as it does; they are copied out before then.
    let numeric = unsafe {
        libc::localeconv_l(locale).as_ref().and_then(|conv| {
            Some(Numeric {
                decimal_sep: text(conv.decimal_point)?,
                thousands_sep: text(conv.thousands_sep)?,
            })
        })
    };
    // SAFETY: made by `newlocale` above and not used after this.
    unsafe { libc::freelocale(locale) };
    numeric
}

#[cfg(windows)]
fn system_numeric() -> Option<Numeric> {
    use windows_sys::Win32::Globalization::{GetLocaleInfoEx, LOCALE_SDECIMAL, LOCALE_STHOUSAND};

    let info = |kind: u32| {
        // Windows allows these at most three characters and a NUL.
        let mut buffer = [0_u16; 16];
        // SAFETY: a null name asks for the user's locale, and the buffer's
        // length is the one given.
        let written = unsafe { GetLocaleInfoEx(std::ptr::null(), kind, buffer.as_mut_ptr(), 16) };
        let written = usize::try_from(written).ok().filter(|&n| n > 0)?;
        String::from_utf16(buffer.get(..written - 1)?).ok()
    };
    Some(Numeric {
        decimal_sep: info(LOCALE_SDECIMAL)?,
        thousands_sep: info(LOCALE_STHOUSAND)?,
    })
}

#[cfg(not(any(target_os = "macos", windows)))]
fn system_numeric() -> Option<Numeric> {
    Numeric::load_user_locale().ok()
}
