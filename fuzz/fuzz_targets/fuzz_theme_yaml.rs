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
    let temp_dir = std::env::temp_dir().join(format!("lez_fuzz_yaml_{}_{}", std::process::id(), nanos));
    if fs::create_dir_all(&temp_dir).is_ok() {
        let yaml_path = temp_dir.join("theme.yml");
        if let Ok(mut f) = StdFile::create(&yaml_path) {
            let _ = f.write_all(data);
            let _ = f.flush();
            drop(f);

            // Fuzz the YAML theme parser
            let config = lez::options::config::ThemeConfig::from_path(yaml_path);
            let _ = config.to_theme();
        }
        let _ = fs::remove_dir_all(&temp_dir);
    }
});
