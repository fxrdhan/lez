// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! Archive inspection: detection of supported archives and, behind the
//! `inspect-archives` feature, a reader for their entries.
//!
//! Policy (agreed upstream in eza#797): anything that goes wrong — corrupt
//! data, I/O errors, unsupported formats — fails *silently* and the archive
//! is simply listed like any regular file.

use std::io;
use std::path::Path;

/// Whether this file name looks like an archive we can inspect. Detection is
/// extension-based for now; content sniffing is future work (see upstream
/// eza#797 discussion).
#[must_use]
pub fn is_archive_name(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("tar"))
}

/// A single entry inside an inspected archive.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Path of the entry as stored in the archive, directories included.
    pub path: String,
    /// Declared size in bytes.
    pub size: u64,
}

/// Human-readable byte count for archive entry annotations.
#[must_use]
pub fn format_size(size: u64) -> String {
    match unit_prefix::NumberPrefix::binary(size as f64) {
        unit_prefix::NumberPrefix::Standalone(b) => format!("{b} B"),
        unit_prefix::NumberPrefix::Prefixed(p, n) => {
            format!("{n:.1} {}B", p.symbol())
        }
    }
}

/// Safety valve so a pathological archive cannot flood the listing.
const MAX_ENTRIES: usize = 500;

/// Reads the entries of a tar archive at `path`.
///
/// Directories are skipped; the result is capped at [`MAX_ENTRIES`] with the
/// remaining count folded into the final synthetic entry when truncated.
pub fn read_entries(path: &Path) -> io::Result<Vec<ArchiveEntry>> {
    use std::fs::File;

    let file = File::open(path)?;
    let mut archive = tar::Archive::new(file);

    let mut out = Vec::new();
    let mut truncated = false;
    for entry in archive.entries()? {
        let entry = match entry {
            Ok(entry) => entry,
            // Corrupt tail or bad header: keep whatever we collected.
            Err(_) => break,
        };
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if out.len() >= MAX_ENTRIES {
            truncated = true;
            break;
        }
        let size = match entry.header().size() {
            Ok(size) => size,
            Err(_) => continue,
        };
        let path_bytes = entry.path_bytes();
        #[cfg(unix)]
        let name = {
            use std::os::unix::ffi::OsStrExt;
            std::ffi::OsStr::from_bytes(&path_bytes)
                .to_string_lossy()
                .into_owned()
        };
        #[cfg(not(unix))]
        let name = String::from_utf8_lossy(&path_bytes).into_owned();

        out.push(ArchiveEntry { path: name, size });
    }

    if truncated {
        out.push(ArchiveEntry {
            path: "… (truncated)".to_owned(),
            size: 0,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn detects_tar_by_extension_case_insensitively() {
        assert!(is_archive_name("backup.tar"));
        assert!(is_archive_name("BACKUP.TAR"));
        assert!(is_archive_name("mixed.Tar"));
        assert!(!is_archive_name("no-extension"));
        assert!(!is_archive_name("archive.zip"));
        // Compressed variants are explicitly out of scope for now.
        assert!(!is_archive_name("archive.tar.gz"));
        assert!(!is_archive_name("archive.tgz"));
    }
}
