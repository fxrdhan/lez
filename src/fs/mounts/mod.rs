// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
use linux::mounts;
#[cfg(target_os = "macos")]
use macos::mounts;

/// Details of a mounted filesystem.
#[derive(Clone)]
pub struct MountedFs {
    pub dest: PathBuf,
    pub fstype: String,
    pub source: String,
}

#[derive(Debug)]
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub enum Error {
    #[cfg(target_os = "macos")]
    GetFSStatError(i32),
    #[cfg(target_os = "linux")]
    IOError(std::io::Error),
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl std::error::Error for Error {}

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Allow unreachable_patterns for windows build
        match self {
            #[cfg(target_os = "macos")]
            Error::GetFSStatError(err) => write!(f, "getfsstat failed: {err}"),
            #[cfg(target_os = "linux")]
            Error::IOError(err) => write!(f, "failed to read /proc/mounts: {err}"),
        }
    }
}

// A lazily initialised static map of all mounted file systems.
//
// The map contains a mapping from the mounted directory path to the
// corresponding mount information. If there's an error retrieving the mount
// list or if we're not running on Linux or Mac, the map will be empty.
//
// Initialise this at application start so we don't have to look the details
// up for every directory. Ideally this would only be done if the --mounts
// option is specified which will be significantly easier once the move
// to `clap` is complete.
/// The last path component of every mount point, as a set.
///
/// A directory can only be a mount point if its name is the name of one, and
/// checking that costs nothing. Working it out the other way round — turning
/// the directory into an absolute path so it can be looked up — costs a
/// `canonicalize` for every directory in every listing, which is a syscall
/// round trip each on filesystems where that is expensive.
///
/// Canonicalising never renames the last component; it only resolves symlinks
/// among the ancestors, and a symlink is not reported as a directory in the
/// first place. So a name that is not in here cannot be a mount point.
pub(super) fn mount_point_names() -> &'static HashSet<OsString> {
    static NAMES: OnceLock<HashSet<OsString>> = OnceLock::new();

    NAMES.get_or_init(|| {
        all_mounts()
            .keys()
            .filter_map(|path| path.file_name().map(std::ffi::OsStr::to_os_string))
            .collect()
    })
}

pub(super) fn all_mounts() -> &'static HashMap<PathBuf, MountedFs> {
    static ALL_MOUNTS: OnceLock<HashMap<PathBuf, MountedFs>> = OnceLock::new();

    ALL_MOUNTS.get_or_init(|| {
        // Allow unused_mut for windows build
        #[allow(unused_mut)]
        let mut mount_map: HashMap<PathBuf, MountedFs> = HashMap::new();

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Ok(mounts) = mounts() {
            for mount in mounts {
                mount_map.insert(mount.dest.clone(), mount);
            }
        }

        mount_map
    })
}
