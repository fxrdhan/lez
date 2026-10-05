// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2
#![no_main]

use libfuzzer_sys::fuzz_target;
use std::fs::{self, File as StdFile};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

fuzz_target!(|data: &[u8]| {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("lez_fuzz_zip_{}_{}", std::process::id(), nanos));
    if fs::create_dir_all(&temp_dir).is_ok() {
        let zip_path = temp_dir.join("input.zip");
        if let Ok(mut f) = StdFile::create(&zip_path) {
            let _ = f.write_all(data);
            let _ = f.flush();
            drop(f);

            // Fuzz the zip central directory reader with arbitrary corrupted bytes
            let _ = lez::fs::archives::read_entries(&zip_path);
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }
});
