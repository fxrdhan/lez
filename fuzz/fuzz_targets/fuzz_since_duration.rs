// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let cmd = lez::options::parser::get_command();
        let _ = cmd.try_get_matches_from(["lez", "--since", s]);
    }
});
