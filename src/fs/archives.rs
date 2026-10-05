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

mod zip;

/// The kinds of archive whose entries can be listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tar,
    Zip,
}

impl Kind {
    fn of(name: &str) -> Option<Self> {
        let (_, ext) = name.rsplit_once('.')?;
        if ext.eq_ignore_ascii_case("tar") {
            Some(Self::Tar)
        } else if ext.eq_ignore_ascii_case("zip") {
            Some(Self::Zip)
        } else {
            None
        }
    }
}

/// Whether this file name looks like an archive we can inspect: an
/// uncompressed `.tar`, or a `.zip`. Detection is extension-based for now;
/// content sniffing is future work (see upstream eza#797 discussion).
#[must_use]
pub fn is_archive_name(name: &str) -> bool {
    Kind::of(name).is_some()
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

/// The entries read from an archive, and whether there were more.
#[derive(Debug, Clone)]
pub struct ArchiveListing {
    pub entries: Vec<ArchiveEntry>,
    /// The archive holds more than [`MAX_ENTRIES`]; the rest are not read.
    pub truncated: bool,
}

/// Safety valve so a pathological archive cannot flood the listing.
pub const MAX_ENTRIES: usize = 500;

/// Reads the entries of the archive at `path`, as its extension names it,
/// or as a tar archive when the extension names none.
///
/// Directories are skipped. Reading stops at [`MAX_ENTRIES`], marking the
/// listing truncated; what is left is not counted, which would mean reading
/// on through an archive of any size.
pub fn read_entries(path: &Path) -> io::Result<ArchiveListing> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    match Kind::of(&name) {
        Some(Kind::Zip) => zip::read_entries(std::fs::File::open(path)?),
        Some(Kind::Tar) | None => read_tar_entries(path),
    }
}

fn read_tar_entries(path: &Path) -> io::Result<ArchiveListing> {
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
        // Not `entry.header().size()`: that reads only the header field,
        // which writers leave at zero once a size passes the 8 GiB it can
        // hold and put the real one in a PAX `size` record. `Entry::size`
        // honours that record, and is a GNU sparse file's real length.
        let size = entry.size();
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

    Ok(ArchiveListing {
        entries: out,
        truncated,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn detects_tar_and_zip_by_extension_case_insensitively() {
        assert!(is_archive_name("backup.tar"));
        assert!(is_archive_name("BACKUP.TAR"));
        assert!(is_archive_name("mixed.Tar"));
        assert!(is_archive_name("archive.zip"));
        assert!(is_archive_name("ARCHIVE.ZIP"));
        assert!(!is_archive_name("no-extension"));
        // Compressed tar variants are explicitly out of scope for now.
        assert!(!is_archive_name("archive.tar.gz"));
        assert!(!is_archive_name("archive.tgz"));
        // Formats built on zip go by their own names, and are not opened.
        assert!(!is_archive_name("library.jar"));
    }
}
