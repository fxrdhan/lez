// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Files, and methods and fields to access their metadata.

use std::collections::{HashMap, HashSet};
use std::fs::FileType;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::str;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{self, AtomicU8};
use std::time::SystemTime;

use chrono::prelude::*;

use log::{debug, error, trace};
use std::sync::LazyLock;

use crate::fs::dir::Dir;
use crate::fs::feature::xattr;
use crate::fs::feature::xattr::{Attribute, FileAttributes};
use crate::fs::fields as f;
use crate::fs::fields::SecurityContextType;
use crate::fs::recursive_size::RecursiveSize;

use super::mounts::MountedFs;
use super::mounts::{all_mounts, mount_point_names};

// Maps a (file handle, shows_dotfiles) => (size_in_bytes, size_in_blocks)
// For windows, size_in_blocks is always 0
// Mutex::new is const but HashMap::new is not const requiring us to use lazy
// initialization.
#[allow(clippy::type_complexity)]
static DIRECTORY_SIZE_CACHE: LazyLock<
    Mutex<HashMap<(Option<same_file::Handle>, bool), (u64, u64)>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// A **File** is a wrapper around one of Rust’s `PathBuf` values, along with
/// associated data about the file.
///
/// Each file is definitely going to have its filename displayed at least
/// once, have its file extension extracted at least once, and have its metadata
/// information queried at least once, so it makes sense to do all this at the
/// start and hold on to all the information.
/// `points_to_dir` has not been worked out yet.
const POINTS_TO_DIR_UNKNOWN: u8 = 0;
/// `points_to_dir` was worked out and the answer was no.
const POINTS_TO_DIR_NO: u8 = 1;
/// `points_to_dir` was worked out and the answer was yes.
const POINTS_TO_DIR_YES: u8 = 2;

pub struct File<'dir> {
    /// The filename portion of this file’s path, including the extension.
    ///
    /// This is used to compare against certain filenames (such as checking if
    /// it’s “Makefile” or something) and to highlight only the filename in
    /// colour when displaying the path.
    pub name: String,

    /// The file’s name’s extension, if present, extracted from the name.
    ///
    /// This is queried many times over, so it’s worth caching it.
    pub ext: Option<String>,

    /// The path that begat this file.
    ///
    /// Even though the file’s name is extracted, the path needs to be kept
    /// around, as certain operations involve looking up the file’s absolute
    /// location (such as searching for compiled files) or using its original
    /// path (following a symlink).
    pub path: PathBuf,

    /// The cached filetype for this file
    pub filetype: OnceLock<Option<std::fs::FileType>>,

    /// A cached `metadata` (`stat`) call for this file.
    ///
    /// This too is queried multiple times, and is *not* cached by the OS, as
    /// it could easily change between invocations — but exa is so short-lived
    /// it’s better to just cache it.
    pub metadata: OnceLock<io::Result<std::fs::Metadata>>,

    /// A reference to the directory that contains this file, if any.
    ///
    /// Filenames that get passed in on the command-line directly will have no
    /// parent directory reference — although they technically have one on the
    /// filesystem, we’ll never need to look at it, so it’ll be `None`.
    /// However, *directories* that get passed in will produce files that
    /// contain a reference to it, which is used in certain operations (such
    /// as looking up compiled files).
    pub parent_dir: Option<&'dir Dir>,

    /// Whether this is one of the two `--all all` directories, `.` and `..`.
    ///
    /// Unlike all other entries, these are not returned as part of the
    /// directory’s children, and are in fact added specifically by exa; this
    /// means that they should be skipped when recursing.
    pub is_all_all: bool,

    /// Whether to dereference symbolic links when querying for information.
    ///
    /// For instance, when querying the size of a symbolic link, if
    /// dereferencing is enabled, the size of the target will be displayed
    /// instead.
    pub deref_links: bool,

    /// Whether to determine MIME types for styling decisions.
    pub mime_read_contents: bool,

    /// The active dotfile filter used for recursive directory size traversal.
    pub dot_filter: Option<super::DotFilter>,

    /// The recursive directory size when `total_size` is used.
    recursive_size: RecursiveSize,

    /// Whether this file — or, for a symlink, the file it points to — is a
    /// directory.
    ///
    /// Answering this for a symlink costs a `readlink` plus a `stat` on the
    /// target, and the directory-grouping sort asks it twice per comparison,
    /// so O(n log n) times over a listing. Caching turns that back into one
    /// lookup per file.
    ///
    /// A tri-state byte rather than a `OnceLock<bool>`, which is sixteen bytes
    /// and would grow every entry in a listing to save the same lookup. Two
    /// threads racing here both compute the same answer, so the store needs no
    /// ordering beyond `Relaxed`.
    points_to_dir: AtomicU8,

    /// The extended attributes of this file.
    extended_attributes: OnceLock<Vec<Attribute>>,

    /// The absolute value of this path, used to look up mount points.
    absolute_path: OnceLock<Option<PathBuf>>,

    /// The MIME type of this file.
    mimetype: OnceLock<Option<&'static str>>,
}

/// Windows has no `.` or `..` entry on disk: a path ending in one is resolved
/// by parsing the path, not by asking the filesystem. So `symlink_metadata(".")`
/// describes whatever the path collapses to — and when the working directory
/// was reached through a directory symlink, that is the link, not the directory
/// it points at. Listing then stops at a single `. -> target` row instead of
/// showing the contents. Following the link is the only way to describe where
/// we actually are.
///
/// Every other platform has a real entry to stat, and a link named `.` cannot
/// exist, so nothing changes there.
fn follow_instead_of_stating_the_link(path: &std::path::Path) -> bool {
    if !cfg!(windows) {
        return false;
    }
    matches!(
        path.components().next_back(),
        Some(std::path::Component::CurDir | std::path::Component::ParentDir)
    )
}

impl<'dir> File<'dir> {
    pub fn from_args<PD, FN>(
        path: PathBuf,
        parent_dir: PD,
        filename: FN,
        deref_links: bool,
        total_size: bool,
        mime_read_contents: bool,
        filetype: Option<std::fs::FileType>,
    ) -> File<'dir>
    where
        PD: Into<Option<&'dir Dir>>,
        FN: Into<Option<String>>,
    {
        Self::from_args_with_filter(
            path,
            parent_dir,
            filename,
            deref_links,
            total_size,
            mime_read_contents,
            filetype,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_args_with_filter<PD, FN>(
        path: PathBuf,
        parent_dir: PD,
        filename: FN,
        deref_links: bool,
        total_size: bool,
        mime_read_contents: bool,
        filetype: Option<std::fs::FileType>,
        dot_filter: Option<super::DotFilter>,
    ) -> File<'dir>
    where
        PD: Into<Option<&'dir Dir>>,
        FN: Into<Option<String>>,
    {
        let parent_dir = parent_dir.into();
        let name = filename.into().unwrap_or_else(|| File::filename(&path));
        let ext = File::ext_from_name(&name);

        let is_all_all = false;
        let recursive_size = if total_size {
            RecursiveSize::Unknown
        } else {
            RecursiveSize::None
        };

        debug!("deref_links {deref_links}");

        let filetype = match filetype {
            Some(f) => OnceLock::from(Some(f)),
            None => OnceLock::new(),
        };

        debug!("deref_links {deref_links}");

        let mut file = File {
            name,
            ext,
            path,
            parent_dir,
            is_all_all,
            deref_links,
            recursive_size,
            filetype,
            mime_read_contents,
            dot_filter,
            metadata: OnceLock::new(),
            points_to_dir: AtomicU8::new(POINTS_TO_DIR_UNKNOWN),
            extended_attributes: OnceLock::new(),
            absolute_path: OnceLock::new(),
            mimetype: OnceLock::new(),
        };

        if total_size {
            file.recursive_size = file.recursive_directory_size();
        }

        file
    }

    fn new_aa(
        path: PathBuf,
        parent_dir: &'dir Dir,
        name: &'static str,
        total_size: bool,
        mime_read_contents: bool,
        dot_filter: Option<super::DotFilter>,
    ) -> File<'dir> {
        let ext = File::ext_from_name(name);

        let is_all_all = true;
        let parent_dir = Some(parent_dir);
        let recursive_size = if total_size {
            RecursiveSize::Unknown
        } else {
            RecursiveSize::None
        };

        let mut file = File {
            name: name.into(),
            ext,
            path,
            parent_dir,
            is_all_all,
            deref_links: false,
            recursive_size,
            mime_read_contents,
            dot_filter,
            metadata: OnceLock::new(),
            points_to_dir: AtomicU8::new(POINTS_TO_DIR_UNKNOWN),
            absolute_path: OnceLock::new(),
            extended_attributes: OnceLock::new(),
            filetype: OnceLock::new(),
            mimetype: OnceLock::new(),
        };

        if total_size {
            file.recursive_size = file.recursive_directory_size();
        }

        file
    }

    #[must_use]
    pub fn new_aa_current(
        parent_dir: &'dir Dir,
        total_size: bool,
        mime_read_contents: bool,
        dot_filter: Option<super::DotFilter>,
    ) -> File<'dir> {
        File::new_aa(
            parent_dir.path.clone(),
            parent_dir,
            ".",
            total_size,
            mime_read_contents,
            dot_filter,
        )
    }

    #[must_use]
    pub fn new_aa_parent(
        path: PathBuf,
        parent_dir: &'dir Dir,
        _total_size: bool,
        mime_read_contents: bool,
        dot_filter: Option<super::DotFilter>,
    ) -> File<'dir> {
        File::new_aa(
            path,
            parent_dir,
            "..",
            false,
            mime_read_contents,
            dot_filter,
        )
    }

    /// A file’s name is derived from its string. This needs to handle directories
    /// such as `/` or `..`, which have no `file_name` component. So instead, just
    /// use the last component as the name.
    #[must_use]
    pub fn filename(path: &Path) -> String {
        if let Some(back) = path.components().next_back() {
            back.as_os_str().to_string_lossy().to_string()
        } else {
            // use the path as fallback
            error!("Path {path:?} has no last component");
            path.display().to_string()
        }
    }

    /// Extract an extension from a file path, if one is present, in lowercase.
    ///
    /// The extension is the series of characters after the last dot. This
    /// deliberately counts dotfiles, so the “.git” folder has the extension “git”.
    ///
    /// ASCII lowercasing is used because these extensions are only compared
    /// against a pre-compiled list of extensions which are known to only exist
    /// within ASCII, so it’s alright.
    fn ext_from_name(name: &str) -> Option<String> {
        name.rfind('.').map(|p| name[p + 1..].to_ascii_lowercase())
    }

    fn ext(path: &Path) -> Option<String> {
        let name = path.file_name()?;
        Self::ext_from_name(&name.to_string_lossy())
    }

    /// Read the extended attributes of a file path.
    fn gather_extended_attributes(&self) -> Vec<Attribute> {
        if xattr::ENABLED {
            let attributes = if self.deref_links {
                self.path.attributes()
            } else {
                self.path.symlink_attributes()
            };
            match attributes {
                Ok(xattrs) => xattrs,
                Err(e) => {
                    error!(
                        "Error looking up extended attributes for {}: {}",
                        self.path.display(),
                        e
                    );
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    fn filetype(&self) -> Option<&std::fs::FileType> {
        self.filetype
            .get_or_init(|| self.metadata().as_ref().ok().map(|md| md.file_type()))
            .as_ref()
    }

    pub fn mimetype(&self) -> Option<&'static str> {
        *self.mimetype.get_or_init(|| {
            if let Some(filetype) = self.filetype()
                && filetype.is_file()
                && self.mime_read_contents
            {
                debug!("Mimetype reading file {:?}", self.path);
                return tree_magic_mini::from_filepath(&self.path).inspect(|mimetype| {
                    debug!("Mimetype {:?} file {:?}", mimetype, self.path);
                });
            }
            None
        })
    }

    pub fn metadata(&self) -> Result<&std::fs::Metadata, &io::Error> {
        self.metadata
            .get_or_init(|| {
                debug!("Statting file {:?}", self.path);
                if follow_instead_of_stating_the_link(&self.path) {
                    return std::fs::metadata(&self.path);
                }
                std::fs::symlink_metadata(&self.path)
            })
            .as_ref()
    }

    /// Get the extended attributes of a file path on demand.
    pub fn extended_attributes(&self) -> &Vec<Attribute> {
        self.extended_attributes
            .get_or_init(|| self.gather_extended_attributes())
    }

    /// Whether this file has any extended attributes without fetching their full values.
    pub fn has_xattrs(&self) -> bool {
        use crate::fs::feature::xattr::FileAttributes;
        if !xattr::ENABLED {
            return false;
        }
        if let Some(xattrs) = self.extended_attributes.get() {
            return !xattrs.is_empty();
        }
        if self.deref_links {
            self.path.has_attributes()
        } else {
            self.path.has_symlink_attributes()
        }
    }

    /// Whether this file is a directory on the filesystem.
    pub fn is_directory(&self) -> bool {
        self.filetype().is_some_and(std::fs::FileType::is_dir)
    }

    /// Whether this file is a `.git` directory.
    #[must_use]
    pub fn is_git_dir(&self) -> bool {
        self.name == ".git"
    }

    /// Whether this file is a directory, or a symlink pointing to a directory.
    pub fn points_to_directory(&self) -> bool {
        // Both of these are already answered by the cached `filetype`, so
        // they cost nothing and settle the question for everything that is
        // not a symlink -- which is nearly every entry in a listing. Reaching
        // for the cache first would tax them for a saving they never collect.
        if self.is_directory() {
            return true;
        }
        if !self.is_link() {
            return false;
        }

        // Only symlinks get this far, and only they pay a `readlink` plus a
        // `stat` on the target, so only they carry the cache.
        match self.points_to_dir.load(atomic::Ordering::Relaxed) {
            POINTS_TO_DIR_NO => return false,
            POINTS_TO_DIR_YES => return true,
            _ => {}
        }

        let answer = match self.link_target() {
            FileTarget::Ok(target) => target.points_to_directory(),
            _ => false,
        };

        self.points_to_dir.store(
            if answer {
                POINTS_TO_DIR_YES
            } else {
                POINTS_TO_DIR_NO
            },
            atomic::Ordering::Relaxed,
        );
        answer
    }

    /// Initializes a new `Dir` object using the `PathBuf` of
    /// the current file. It does not perform any validation to check if the
    /// file is actually a directory. To verify that, use `is_directory()`.
    pub fn to_dir(&self) -> Dir {
        trace!("read_dir: initializing dir from path");
        Dir::new(self.path.clone())
    }

    /// If this file is a directory on the filesystem, then clone its
    /// `PathBuf` for use in one of our own `Dir` values, and read a list of
    /// its contents.
    ///
    /// Returns an IO error upon failure, but this shouldn’t be used to check
    /// if a `File` is a directory or not! For that, just use `is_directory()`.
    pub fn read_dir(&self) -> io::Result<Dir> {
        trace!("read_dir: reading dir");
        Dir::read_dir(self.path.clone())
    }

    /// Whether this file is a regular file on the filesystem — that is, not a
    /// directory, a link, or anything else treated specially.
    pub fn is_file(&self) -> bool {
        self.filetype().is_some_and(std::fs::FileType::is_file)
    }

    /// The programming language this file is written in, worked out from its
    /// name or extension, if eza recognises it.
    pub fn language(&self) -> Option<&'static crate::loc::Language> {
        crate::loc::language_for(&self.name, self.ext.as_deref())
    }

    /// Count this file’s lines of code. Returns `None` for anything that
    /// isn’t a readable, regular file in a recognised language — that is,
    /// directories, links, unknown extensions, and binaries.
    pub fn loc(&self) -> Option<crate::loc::LocCounts> {
        if !self.is_file() {
            return None;
        }
        let lang = self.language()?;
        crate::loc::LocCounts::from_path(&self.path, lang)
            .ok()
            .flatten()
    }

    /// Whether this file is both a regular file *and* executable for the
    /// current user. An executable file has a different purpose from an
    /// executable directory, so they should be highlighted differently.
    #[cfg(unix)]
    pub fn is_executable_file(&self) -> bool {
        let bit = modes::USER_EXECUTE;
        if !self.is_file() {
            return false;
        }
        let Ok(md) = self.metadata() else {
            return false;
        };
        (md.permissions().mode() & bit) == bit
    }

    /// Windows edition: a regular file is “executable” when its extension
    /// appears in the `PATHEXT` environment variable (defaulting to the
    /// usual `.COM;.EXE;.BAT;…` list when unset).
    #[cfg(windows)]
    pub fn is_executable_file(&self) -> bool {
        use std::collections::HashSet;
        use std::sync::LazyLock;

        static PATHEXT: LazyLock<HashSet<String>> = LazyLock::new(|| {
            std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
                .split(';')
                .filter_map(|s| s.strip_prefix('.').map(|s| s.to_ascii_uppercase()))
                .collect()
        });

        if !self.is_file() {
            return false;
        }
        match self.ext.as_ref() {
            Some(ext) => PATHEXT.contains(&ext.to_ascii_uppercase()),
            None => false,
        }
    }

    /// Whether this directory is a Btrfs subvolume: subvolumes always carry
    /// inode number 256 (BTRFS_FIRST_FREE_OBJECTID) and live on a btrfs
    /// filesystem. Non-Linux platforms never report one.
    #[cfg(target_os = "linux")]
    pub fn is_btrfs_subvolume(&self) -> bool {
        const BTRFS_FIRST_FREE_OBJECTID: u64 = 256;
        self.is_directory() && self.is_btrfs() && self.inode().0 == BTRFS_FIRST_FREE_OBJECTID
    }

    #[cfg(not(target_os = "linux"))]
    pub fn is_btrfs_subvolume(&self) -> bool {
        false
    }

    /// Walks up the ancestor chain against the mount table, then falls back
    /// to `statfs`, to decide whether this path lives on btrfs.
    #[cfg(target_os = "linux")]
    fn is_btrfs(&self) -> bool {
        use std::os::unix::ffi::OsStrExt;

        const BTRFS_FSTYPE_NAME: &str = "btrfs";

        let start = self.absolute_path().unwrap_or(&self.path);
        for part in start.ancestors() {
            if let Some(mount) = all_mounts().get(part) {
                return mount.fstype == BTRFS_FSTYPE_NAME;
            }
        }

        let mut out = std::mem::MaybeUninit::<libc::statfs>::uninit();
        let path = match std::ffi::CString::new(self.path.as_os_str().as_bytes()) {
            Ok(path) => path,
            Err(_) => return false,
        };
        // SAFETY: `out` is a valid, correctly-sized location for statfs to
        // initialise; errno is the only error channel.
        let result = unsafe { libc::statfs(path.as_ptr(), out.as_mut_ptr()) };
        result == 0 && unsafe { out.assume_init() }.f_type == libc::BTRFS_SUPER_MAGIC
    }

    /// Whether this file carries Linux file capabilities, which `LS_COLORS`
    /// colours through its `ca` entry. Only asked when a style was set for it,
    /// because answering costs a syscall.
    #[must_use]
    pub fn has_capabilities(&self) -> bool {
        xattr::has_capabilities(&self.path)
    }

    /// Whether this file is a symlink on the filesystem.
    pub fn is_link(&self) -> bool {
        self.filetype().is_some_and(FileType::is_symlink)
    }

    /// Whether this file is a named pipe on the filesystem.
    #[cfg(unix)]
    pub fn is_pipe(&self) -> bool {
        self.filetype().is_some_and(FileTypeExt::is_fifo)
    }

    /// Whether this file is a char device on the filesystem.
    #[cfg(unix)]
    pub fn is_char_device(&self) -> bool {
        self.filetype().is_some_and(FileTypeExt::is_char_device)
    }

    /// Whether this file is a block device on the filesystem.
    #[cfg(unix)]
    pub fn is_block_device(&self) -> bool {
        self.filetype().is_some_and(FileTypeExt::is_block_device)
    }

    /// Whether this file is a socket on the filesystem.
    #[cfg(unix)]
    pub fn is_socket(&self) -> bool {
        self.filetype().is_some_and(FileTypeExt::is_socket)
    }

    /// Determine the full path resolving all symbolic links on demand.
    pub fn absolute_path(&self) -> Option<&PathBuf> {
        self.absolute_path
            .get_or_init(|| {
                if self.is_link() && self.link_target().is_broken() {
                    // workaround for broken symlinks to get absolute path for parent and then
                    // append name of file; std::fs::canonicalize requires all path components
                    // (including the last one) to exist
                    self.path
                        .parent()
                        .and_then(|parent| std::fs::canonicalize(parent).ok())
                        .map(|p| p.join(self.name.clone()))
                } else {
                    std::fs::canonicalize(&self.path).ok()
                }
            })
            .as_ref()
    }

    /// Whether this file is a mount point
    pub fn is_mount_point(&self) -> bool {
        if cfg!(not(any(target_os = "linux", target_os = "macos"))) || !self.is_directory() {
            return false;
        }
        let all_mounts = all_mounts();
        if !all_mounts.is_empty() {
            // Checked before turning the path into an absolute one, because
            // that costs a `canonicalize` per directory and this is the first
            // arm every file is matched against. A directory whose name is not
            // the name of any mount point cannot be one. A path with no last
            // component is the root, which the table may well hold, so that
            // one goes through to the lookup below.
            if let Some(name) = self.path.file_name()
                && !mount_point_names().contains(name)
            {
                return false;
            }
            return self
                .absolute_path()
                .is_some_and(|p| all_mounts.contains_key(p));
        }
        #[cfg(unix)]
        if let Ok(x) = std::fs::metadata(&self.path)
            && let Ok(y) = std::fs::metadata(self.path.join(".."))
        {
            // .dev() is the traditional fallback used by mountpoint(1). Misses bind mounts.
            // .ino() detects the root directory, which parents itself and is always a mount
            return x.dev() != y.dev() || x.ino() == y.ino();
        }
        false
    }

    /// The filesystem device and type for a mount point
    pub fn mount_point_info(&self) -> Option<&MountedFs> {
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            return self.absolute_path().and_then(|p| all_mounts().get(p));
        }
        None
    }

    /// Re-prefixes the path pointed to by this file, if it’s a symlink, to
    /// make it an absolute path that can be accessed from whichever
    /// directory exa is being run from.
    fn reorient_target_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(dir) = self.parent_dir {
            dir.join(path)
        } else if let Some(parent) = self.path.parent() {
            parent.join(path)
        } else {
            self.path.join(path)
        }
    }

    /// Again assuming this file is a symlink, follows that link and returns
    /// the result of following it.
    ///
    /// For a working symlink that the user is allowed to follow,
    /// this will be the `File` object at the other end, which can then have
    /// its name, colour, and other details read.
    ///
    /// For a broken symlink, returns where the file *would* be, if it
    /// existed. If this file cannot be read at all, returns the error that
    /// we got when we tried to read it.
    pub fn link_target(&self) -> FileTarget<'dir> {
        // We need to be careful to treat the path actually pointed to by
        // this file — which could be absolute or relative — to the path
        // we actually look up and turn into a `File` — which needs to be
        // absolute to be accessible from any directory.
        debug!("Reading link {:?}", self.path);
        let path = match std::fs::read_link(&self.path) {
            Ok(p) => p,
            Err(e) => return FileTarget::Err(e),
        };

        // A symlink with an empty target is always broken. We must check
        // this before calling reorient_target_path, because joining an
        // empty path with the parent directory would resolve to the parent
        // itself, incorrectly treating the broken symlink as a directory.
        if path.as_os_str().is_empty() {
            return FileTarget::Broken(path);
        }

        let absolute_path = self.reorient_target_path(&path);

        // Use plain `metadata` instead of `symlink_metadata` - we *want* to
        // follow links.
        match std::fs::metadata(&absolute_path) {
            Ok(metadata) => {
                let ext = File::ext(&path);
                let name = File::filename(&path);
                let extended_attributes = OnceLock::new();
                let absolute_path_cell = OnceLock::from(Some(absolute_path));
                let file = File {
                    parent_dir: None,
                    path,
                    ext,
                    filetype: OnceLock::from(Some(metadata.file_type())),
                    metadata: OnceLock::from(Ok(metadata)),
                    name,
                    is_all_all: false,
                    deref_links: self.deref_links,
                    points_to_dir: AtomicU8::new(POINTS_TO_DIR_UNKNOWN),
                    extended_attributes,
                    absolute_path: absolute_path_cell,
                    recursive_size: RecursiveSize::None,
                    mime_read_contents: self.mime_read_contents,
                    dot_filter: self.dot_filter,
                    mimetype: OnceLock::new(),
                };
                FileTarget::Ok(Box::new(file))
            }
            Err(e) => {
                error!("Error following link {:?}: {:#?}", path, e);
                FileTarget::Broken(path)
            }
        }
    }

    /// Assuming this file is a symlink, follows that link and any further
    /// links recursively, returning the result from following the trail.
    ///
    /// For a working symlink that the user is allowed to follow,
    /// this will be the `File` object at the other end, which can then have
    /// its name, colour, and other details read.
    ///
    /// For a broken symlink, returns where the file *would* be, if it
    /// existed. If this file cannot be read at all, returns the error that
    /// we got when we tried to read it.
    pub fn link_target_recurse(&self) -> FileTarget<'dir> {
        let target = self.link_target();
        if let FileTarget::Ok(f) = target {
            if f.is_link() {
                return f.link_target_recurse();
            }
            return FileTarget::Ok(f);
        }
        target
    }

    /// This file’s number of hard links.
    ///
    /// It also reports whether this is both a regular file, and a file with
    /// multiple links. This is important, because a file with multiple links
    /// is uncommon, while you come across directories and other types
    /// with multiple links much more often. Thus, it should get highlighted
    /// more attentively.
    #[cfg(unix)]
    pub fn links(&self) -> f::Links {
        let count = self.metadata().map_or(0, MetadataExt::nlink);

        f::Links {
            count,
            multiple: self.is_file() && count > 1,
        }
    }

    /// This file’s inode.
    #[cfg(unix)]
    pub fn inode(&self) -> f::Inode {
        f::Inode(self.metadata().map_or(0, MetadataExt::ino))
    }

    /// This actual size the file takes up on disk, in bytes.
    #[cfg(unix)]
    pub fn blocksize(&self) -> f::Blocksize {
        if self.deref_links && self.is_link() {
            match self.link_target() {
                FileTarget::Ok(f) => f.blocksize(),
                _ => f::Blocksize::None,
            }
        } else if self.is_directory() {
            self.recursive_size.map_or(f::Blocksize::None, |_, blocks| {
                f::Blocksize::Some(blocks * 512)
            })
        } else if self.is_file() {
            // Note that metadata.blocks returns the number of blocks
            // for 512 byte blocks according to the POSIX standard
            // even though the physical block size may be different.
            f::Blocksize::Some(self.metadata().map_or(0, |md| md.blocks() * 512))
        } else {
            // directory or symlinks
            f::Blocksize::None
        }
    }

    /// The ID of the user that own this file. If dereferencing links, the links
    /// may be broken, in which case `None` will be returned.
    #[cfg(unix)]
    pub fn user(&self) -> Option<f::User> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.user(),
                _ => None,
            };
        }
        Some(f::User(self.metadata().map_or(0, MetadataExt::uid)))
    }

    /// The ID of the group that owns this file.
    #[cfg(unix)]
    pub fn group(&self) -> Option<f::Group> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.group(),
                _ => None,
            };
        }
        Some(f::Group(self.metadata().map_or(0, MetadataExt::gid)))
    }

    /// This file’s size, if it’s a regular file.
    ///
    /// For directories, the recursive size or no size is given depending on
    /// flags. Although they do have a size on some filesystems, I’ve never
    /// looked at one of those numbers and gained any information from it.
    ///
    /// Block and character devices return their device IDs, because they
    /// usually just have a file size of zero.
    ///
    /// Links will return the size of their target (recursively through other
    /// links) if dereferencing is enabled, otherwise None.
    pub fn size(&self) -> f::Size {
        if self.deref_links && self.is_link() {
            return match self.link_target() {
                FileTarget::Ok(f) => f.size(),
                _ => f::Size::None,
            };
        }

        if self.is_directory() {
            return self
                .recursive_size
                .map_or(f::Size::None, |bytes, _| f::Size::Some(bytes));
        }

        #[cfg(unix)]
        if self.is_char_device() || self.is_block_device() {
            let device_id = self.metadata().map_or(0, MetadataExt::rdev);

            // MacOS and Linux have different arguments and return types for the
            // functions major and minor.  On Linux the try_into().unwrap() and
            // the "as u32" cast are not needed.  We turn off the warning to
            // allow it to compile cleanly on Linux.
            //
            // On illumos and Solaris, major and minor are extern "C" fns and
            // therefore unsafe; on other platforms the functions are defined as
            // macros and copied as const fns in the libc crate.
            #[allow(trivial_numeric_casts, unused_unsafe)]
            #[allow(clippy::unnecessary_cast, clippy::useless_conversion)]
            {
                let device_id = device_id
                    .try_into()
                    .expect("Malformed device major ID when getting filesize");
                return f::Size::DeviceIDs(f::DeviceIDs {
                    major: unsafe { libc::major(device_id) as u32 },
                    minor: unsafe { libc::minor(device_id) as u32 },
                });
            }
        }

        if self.is_file() {
            return f::Size::Some(self.metadata().map_or(0, std::fs::Metadata::len));
        }

        f::Size::None // symlink
    }

    /// Calculate the total directory size recursively.  If not a directory `None`
    /// will be returned.  The directory size is cached for recursive directory
    /// listing.
    fn recursive_directory_size(&self) -> RecursiveSize {
        if !self.is_directory() {
            return RecursiveSize::None;
        }

        let dot_filter = self.dot_filter.unwrap_or(super::DotFilter::Dotfiles);
        let shows_dotfiles = dot_filter.shows_dotfiles();
        let handle = same_file::Handle::from_path(&self.path).ok();
        let cache_key = (handle, shows_dotfiles);

        if let Some(size) = DIRECTORY_SIZE_CACHE.lock().unwrap().get(&cache_key) {
            return RecursiveSize::Some(size.0, size.1);
        }

        let mut visited = HashSet::new();
        #[cfg(unix)]
        if let Ok(md) = self.metadata() {
            visited.insert((md.dev(), md.ino()));
        }
        #[cfg(not(unix))]
        {
            visited.insert(self.path.clone());
        }

        let Ok(dir) = Dir::read_dir(self.path.clone()) else {
            return RecursiveSize::Unknown;
        };

        let (size, blocks) =
            dir.calculate_recursive_size(&mut visited, dot_filter, self.mime_read_contents);

        DIRECTORY_SIZE_CACHE
            .lock()
            .unwrap()
            .insert(cache_key, (size, blocks));
        RecursiveSize::Some(size, blocks)
    }

    /// Returns the same value as `self.metadata.len()` or the recursive size
    /// of a directory when `total_size` is used.
    #[inline]
    pub fn length(&self) -> u64 {
        self.recursive_size.unwrap_bytes_or(
            match (self.is_link(), self.deref_links, self.link_target_recurse()) {
                (true, true, FileTarget::Ok(ref f)) => f,
                _ => self,
            }
            .metadata()
            .map_or(0, std::fs::Metadata::len),
        )
    }

    /// Is the file is using recursive size calculation
    #[inline]
    pub fn is_recursive_size(&self) -> bool {
        !self.recursive_size.is_none()
    }

    /// Determines if the directory is empty or not.
    ///
    /// For Unix platforms, this function first checks the link count to quickly
    /// determine non-empty directories. On most UNIX filesystems the link count
    /// is two plus the number of subdirectories. If the link count is less than
    /// or equal to 2, it then checks the directory contents to determine if
    /// it's truly empty. The naive approach used here checks the contents
    /// directly, as certain filesystems make it difficult to infer emptiness
    /// based on directory size alone.
    #[cfg(unix)]
    pub fn is_empty_dir(&self) -> bool {
        if self.is_directory() {
            if self.metadata().map_or(0, MetadataExt::nlink) > 2 {
                // Directories will have a link count of two if they do not have any subdirectories.
                // The '.' entry is a link to itself and the '..' is a link to the parent directory.
                // A subdirectory will have a link to its parent directory increasing the link count
                // above two.  This will avoid the expensive read_dir call below when a directory
                // has subdirectories.
                false
            } else {
                self.is_empty_directory()
            }
        } else {
            false
        }
    }

    /// Determines if the directory is empty or not.
    ///
    /// For Windows platforms, this function checks the directory contents directly
    /// to determine if it's empty. Since certain filesystems on Windows make it
    /// challenging to infer emptiness based on directory size, this approach is used.
    #[cfg(windows)]
    pub fn is_empty_dir(&self) -> bool {
        if self.is_directory() {
            self.is_empty_directory()
        } else {
            false
        }
    }

    /// Checks the contents of the directory to determine if it's empty.
    ///
    /// This function avoids counting '.' and '..' when determining if the directory is
    /// empty. If any other entries are found, it returns `false`.
    ///
    /// The naive approach, as one would think that this info may have been cached.
    /// but as mentioned in the size function comment above, different filesystems
    /// make it difficult to get any info about a dir by it's size, so this may be it.
    fn is_empty_directory(&self) -> bool {
        trace!("is_empty_directory: reading dir");
        // One entry settles it, so read lazily and stop there. Going through
        // `Dir` looked equivalent — it also only takes the first entry — but
        // `Dir::read_dir` collects the whole directory before handing anything
        // back, so answering "is this empty" for a directory of a hundred
        // thousand files read all hundred thousand. That is the cost behind
        // the reports of icons being slow on network and fuse mounts.
        //
        // `read_dir` never yields `.` or `..`, which is the filter `Dir` was
        // being asked for here.
        match std::fs::read_dir(&self.path) {
            Ok(mut entries) => entries.next().is_none(),
            Err(_) => false,
        }
    }

    /// Converts a `SystemTime` to a `NaiveDateTime` without panicking.
    ///
    /// Fixes #655 and #667 in `Self::modified_time`, `Self::accessed_time` and
    /// `Self::created_time`.
    pub fn systemtime_to_naivedatetime(st: SystemTime) -> Option<NaiveDateTime> {
        let (secs, nanos) = match st.duration_since(SystemTime::UNIX_EPOCH) {
            // Time at or after the UNIX epoch.
            Ok(duration) => (
                duration.as_secs().try_into().ok()?,
                (duration.as_nanos() % 1_000_000_000).try_into().ok()?,
            ),
            // Time before the UNIX epoch (#1668): `duration_since` returns an
            // `Err` whose duration is the absolute distance back to the epoch.
            // Negate it, and when there is a sub-second part floor towards
            // negative infinity so the timestamp matches how Unix counts time
            // before 1970 (otherwise these files render their date as `-`).
            Err(err) => {
                let duration = err.duration();
                let mut secs: i64 = -i64::try_from(duration.as_secs()).ok()?;
                let mut nanos = duration.subsec_nanos();
                if nanos > 0 {
                    secs -= 1;
                    nanos = 1_000_000_000 - nanos;
                }
                (secs, nanos)
            }
        };

        DateTime::from_timestamp(secs, nanos).map(|dt| dt.naive_local())
    }

    /// This file’s last modified timestamp, if available on this platform.
    pub fn modified_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.modified_time(),
                _ => None,
            };
        }
        self.metadata()
            .ok()
            .and_then(|md| md.modified().ok())
            .and_then(Self::systemtime_to_naivedatetime)
    }

    /// This file’s last changed timestamp, if available on this platform.
    #[cfg(unix)]
    pub fn changed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.changed_time(),
                _ => None,
            };
        }
        let md = self.metadata();
        DateTime::from_timestamp(
            md.map_or(0, MetadataExt::ctime),
            md.map_or(0, |md| md.ctime_nsec() as u32),
        )
        .map(|dt| dt.naive_local())
    }

    #[cfg(windows)]
    pub fn changed_time(&self) -> Option<NaiveDateTime> {
        self.modified_time()
    }

    /// This file’s last accessed timestamp, if available on this platform.
    pub fn accessed_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.accessed_time(),
                _ => None,
            };
        }
        self.metadata()
            .ok()
            .and_then(|md| md.accessed().ok())
            .and_then(Self::systemtime_to_naivedatetime)
    }

    /// This file’s created timestamp, if available on this platform.
    pub fn created_time(&self) -> Option<NaiveDateTime> {
        if self.is_link() && self.deref_links {
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.created_time(),
                _ => None,
            };
        }
        let btime = self.metadata().ok()?.created().ok()?;
        Self::systemtime_to_naivedatetime(btime)
    }

    /// This file’s ‘type’.
    ///
    /// This is used a the leftmost character of the permissions column.
    /// The file type can usually be guessed from the colour of the file, but
    /// ls puts this character there.
    #[cfg(unix)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else if self.is_pipe() {
            f::Type::Pipe
        } else if self.is_link() {
            f::Type::Link
        } else if self.is_char_device() {
            f::Type::CharDevice
        } else if self.is_block_device() {
            f::Type::BlockDevice
        } else if self.is_socket() {
            f::Type::Socket
        } else {
            f::Type::Special
        }
    }

    #[cfg(windows)]
    pub fn type_char(&self) -> f::Type {
        if self.is_file() {
            f::Type::File
        } else if self.is_directory() {
            f::Type::Directory
        } else {
            f::Type::Special
        }
    }

    /// This file’s permissions, with flags for each bit.
    #[cfg(unix)]
    pub fn permissions(&self) -> Option<f::Permissions> {
        if self.is_link() && self.deref_links {
            // If the chain of links is broken, we instead fall through and
            // return the permissions of the original link, as would have been
            // done if we were not dereferencing.
            return match self.link_target_recurse() {
                FileTarget::Ok(f) => f.permissions(),
                _ => None,
            };
        }
        let bits = self.metadata().map_or(0, MetadataExt::mode);
        let has_bit = |bit| bits & bit == bit;

        Some(f::Permissions {
            user_read: has_bit(modes::USER_READ),
            user_write: has_bit(modes::USER_WRITE),
            user_execute: has_bit(modes::USER_EXECUTE),

            group_read: has_bit(modes::GROUP_READ),
            group_write: has_bit(modes::GROUP_WRITE),
            group_execute: has_bit(modes::GROUP_EXECUTE),

            other_read: has_bit(modes::OTHER_READ),
            other_write: has_bit(modes::OTHER_WRITE),
            other_execute: has_bit(modes::OTHER_EXECUTE),

            sticky: has_bit(modes::STICKY),
            setgid: has_bit(modes::SETGID),
            setuid: has_bit(modes::SETUID),
        })
    }

    #[cfg(windows)]
    pub fn attributes(&self) -> Option<f::Attributes> {
        let bits = self.metadata().ok()?.file_attributes();
        let has_bit = |bit| bits & bit == bit;

        // https://docs.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
        Some(f::Attributes {
            directory: has_bit(0x10),
            archive: has_bit(0x20),
            readonly: has_bit(0x1),
            hidden: has_bit(0x2),
            system: has_bit(0x4),
            reparse_point: has_bit(0x400),
        })
    }

    /// This file’s security context field.
    #[cfg(unix)]
    pub fn security_context(&self) -> f::SecurityContext<'_> {
        let context = match self
            .extended_attributes()
            .iter()
            .find(|a| a.name == "security.selinux")
        {
            Some(attr) => match &attr.value {
                None => SecurityContextType::None,
                Some(value) => match str::from_utf8(value) {
                    Ok(v) => SecurityContextType::SELinux(v.trim_end_matches(char::from(0))),
                    Err(_) => SecurityContextType::None,
                },
            },
            None => SecurityContextType::None,
        };

        f::SecurityContext { context }
    }

    #[cfg(windows)]
    pub fn security_context(&self) -> f::SecurityContext<'_> {
        f::SecurityContext {
            context: SecurityContextType::None,
        }
    }

    /// User file flags.
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly"
    ))]
    pub fn flags(&self) -> f::Flags {
        #[cfg(target_os = "dragonfly")]
        use std::os::dragonfly::fs::MetadataExt;
        #[cfg(target_os = "freebsd")]
        use std::os::freebsd::fs::MetadataExt;
        #[cfg(target_os = "macos")]
        use std::os::macos::fs::MetadataExt;
        #[cfg(target_os = "netbsd")]
        use std::os::netbsd::fs::MetadataExt;
        #[cfg(target_os = "openbsd")]
        use std::os::openbsd::fs::MetadataExt;
        f::Flags(
            self.metadata()
                .map(MetadataExt::st_flags)
                .unwrap_or_default(),
        )
    }

    #[cfg(windows)]
    pub fn flags(&self) -> f::Flags {
        f::Flags(self.metadata().map_or(0, |md| md.file_attributes()))
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_os = "windows"
    )))]
    pub fn flags(&self) -> f::Flags {
        f::Flags(0)
    }

    #[cfg(unix)]
    pub fn permissions_plus(&self, xattrs: bool) -> Option<f::PermissionsPlus> {
        self.permissions().map(|p| f::PermissionsPlus {
            file_type: self.type_char(),
            permissions: p,
            xattrs,
            mount: self.is_mount_point(),
        })
    }

    #[allow(clippy::unnecessary_wraps)] // Needs to match Unix function
    #[cfg(windows)]
    pub fn permissions_plus(&self, xattrs: bool) -> Option<f::PermissionsPlus> {
        Some(f::PermissionsPlus {
            file_type: self.type_char(),
            #[cfg(windows)]
            attributes: self.attributes()?,
            xattrs,
            mount: false,
        })
    }
}

impl<'a> AsRef<File<'a>> for File<'a> {
    fn as_ref(&self) -> &File<'a> {
        self
    }
}

/// The result of following a symlink.
pub enum FileTarget<'dir> {
    /// The symlink pointed at a file that exists.
    Ok(Box<File<'dir>>),

    /// The symlink pointed at a file that does not exist. Holds the path
    /// where the file would be, if it existed.
    Broken(PathBuf),

    /// There was an IO error when following the link. This can happen if the
    /// file isn’t a link to begin with, but also if, say, we don’t have
    /// permission to follow it.
    Err(io::Error),
    // Err is its own variant, instead of having the whole thing be inside an
    // `io::Result`, because being unable to follow a symlink is not a serious
    // error — we just display the error message and move on.
}

impl FileTarget<'_> {
    /// Whether this link doesn’t lead to a file, for whatever reason. This
    /// gets used to determine how to highlight the link in grid views.
    #[must_use]
    pub fn is_broken(&self) -> bool {
        matches!(self, Self::Broken(_) | Self::Err(_))
    }
}

/// More readable aliases for the permission bits exposed by libc.
#[allow(trivial_numeric_casts)]
#[cfg(unix)]
mod modes {

    // The `libc::mode_t` type’s actual type varies, but the value returned
    // from `metadata.permissions().mode()` is always `u32`.
    pub type Mode = u32;

    pub const USER_READ: Mode = libc::S_IRUSR as Mode;
    pub const USER_WRITE: Mode = libc::S_IWUSR as Mode;
    pub const USER_EXECUTE: Mode = libc::S_IXUSR as Mode;

    pub const GROUP_READ: Mode = libc::S_IRGRP as Mode;
    pub const GROUP_WRITE: Mode = libc::S_IWGRP as Mode;
    pub const GROUP_EXECUTE: Mode = libc::S_IXGRP as Mode;

    pub const OTHER_READ: Mode = libc::S_IROTH as Mode;
    pub const OTHER_WRITE: Mode = libc::S_IWOTH as Mode;
    pub const OTHER_EXECUTE: Mode = libc::S_IXOTH as Mode;

    pub const STICKY: Mode = libc::S_ISVTX as Mode;
    pub const SETGID: Mode = libc::S_ISGID as Mode;
    pub const SETUID: Mode = libc::S_ISUID as Mode;
}

#[cfg(test)]
mod ext_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn extension() {
        assert_eq!(Some("dat".to_string()), File::ext(Path::new("fester.dat")));
    }

    #[test]
    fn dotfile() {
        assert_eq!(Some("vimrc".to_string()), File::ext(Path::new(".vimrc")));
    }

    #[test]
    fn no_extension() {
        assert_eq!(None, File::ext(Path::new("jarlsberg")));
    }
}

#[cfg(test)]
mod filename_test {
    use super::File;
    use std::path::Path;

    #[test]
    fn file() {
        assert_eq!("fester.dat", File::filename(Path::new("fester.dat")));
    }

    #[test]
    fn no_path() {
        assert_eq!("foo.wha", File::filename(Path::new("/var/cache/foo.wha")));
    }

    #[test]
    fn here() {
        assert_eq!(".", File::filename(Path::new(".")));
    }

    #[test]
    fn there() {
        assert_eq!("..", File::filename(Path::new("..")));
    }

    #[test]
    fn everywhere() {
        assert_eq!("..", File::filename(Path::new("./..")));
    }

    #[test]
    #[cfg(unix)]
    fn topmost() {
        assert_eq!("/", File::filename(Path::new("/")));
    }
}

#[cfg(test)]
mod is_git_dir_test {
    use super::File;
    use std::path::PathBuf;

    #[test]
    fn test_is_git_dir() {
        let dotgit = File::from_args(PathBuf::from(".git"), None, None, false, false, false, None);
        assert!(dotgit.is_git_dir());

        let dotgithub = File::from_args(
            PathBuf::from(".github"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        assert!(!dotgithub.is_git_dir());

        let regular_file = File::from_args(
            PathBuf::from("main.rs"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        assert!(!regular_file.is_git_dir());

        let nested_dotgit = File::from_args(
            PathBuf::from("repo/.git"),
            None,
            None,
            false,
            false,
            false,
            None,
        );
        assert!(nested_dotgit.is_git_dir());
    }
}

#[cfg(test)]
#[cfg(unix)]
mod length_test {
    use super::File;
    use std::fs::{self, File as StdFile};
    use std::io::Write;

    #[test]
    #[cfg(unix)]
    fn dereference_symlink_length() {
        let temp_dir =
            std::env::temp_dir().join(format!("lez_test_deref_len_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let target_path = temp_dir.join("target.txt");
        let link_path = temp_dir.join("link.txt");

        let mut target_file = StdFile::create(&target_path).unwrap();
        target_file.write_all(b"hello world!").unwrap();
        drop(target_file);

        std::os::unix::fs::symlink(&target_path, &link_path).unwrap();

        let link_no_deref =
            File::from_args(link_path.clone(), None, None, false, false, false, None);
        let link_deref = File::from_args(link_path, None, None, true, false, false, None);

        assert_eq!(
            link_no_deref.length(),
            target_path.to_str().unwrap().len() as u64
        );
        assert_eq!(link_deref.length(), 12);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

#[cfg(test)]
mod systemtime_to_naivedatetime_test {
    use super::File;
    use chrono::Datelike;
    use std::time::{Duration, SystemTime};

    #[test]
    fn post_epoch() {
        // 2001-09-09T01:46:40 UTC == 1_000_000_000 seconds after the epoch.
        let st = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        let dt = File::systemtime_to_naivedatetime(st).expect("post-epoch time");
        assert_eq!(dt.and_utc().timestamp(), 1_000_000_000);
        assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
        assert_eq!(dt.year(), 2001);
    }

    #[test]
    fn epoch() {
        let dt = File::systemtime_to_naivedatetime(SystemTime::UNIX_EPOCH).expect("epoch time");
        assert_eq!(dt.and_utc().timestamp(), 0);
        assert_eq!(dt.year(), 1970);
    }

    #[test]
    fn pre_epoch() {
        // #1668: a time before the UNIX epoch must still yield a real date
        // rather than `None` (which renders as `-`).
        let st = SystemTime::UNIX_EPOCH - Duration::from_secs(2_147_483_648);
        let dt = File::systemtime_to_naivedatetime(st).expect("pre-epoch time");
        assert_eq!(dt.and_utc().timestamp(), -2_147_483_648);
        assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 0);
        assert!(dt.year() < 1970);
        assert_eq!(dt.year(), 1901);
    }

    #[test]
    fn pre_epoch_subsecond() {
        // A pre-epoch time with a sub-second component must floor towards
        // negative infinity so the whole-seconds value and nanos line up.
        let st = SystemTime::UNIX_EPOCH - Duration::new(10, 250_000_000);
        let dt = File::systemtime_to_naivedatetime(st).expect("pre-epoch subsecond time");
        // -10.25s == -11s + 0.75s.
        assert_eq!(dt.and_utc().timestamp(), -11);
        assert_eq!(dt.and_utc().timestamp_subsec_nanos(), 750_000_000);
        assert!(dt.year() < 1970);
    }
}

#[cfg(test)]
#[cfg(unix)]
mod broken_symlink_test {
    use super::*;
    use std::os::unix::fs as unix_fs;
    use std::path::PathBuf;

    fn make_file(path: PathBuf) -> File<'static> {
        File::from_args(path, None, None, false, false, false, None)
    }

    /// A symlink with an empty target should be treated as broken, not as
    /// pointing to a directory. Regression test for
    /// https://github.com/eza-community/eza/issues/1715
    #[test]
    fn empty_target_symlink_is_not_directory() {
        let temp_dir =
            std::env::temp_dir().join(format!("lez_test_empty_symlink_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let link_path = temp_dir.join("empty-link");
        // Some environments (e.g. Nix sandbox) don't allow creating
        // symlinks with empty targets, so skip if that's the case.
        if unix_fs::symlink("", &link_path).is_err() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return;
        }

        let file = make_file(link_path);

        assert!(file.is_link(), "should be recognized as a symlink");
        assert!(
            !file.is_directory(),
            "should not be recognized as a directory"
        );
        assert!(
            !file.points_to_directory(),
            "broken symlink with empty target should not point to a directory"
        );

        let target = file.link_target();
        assert!(
            target.is_broken(),
            "symlink with empty target should be considered broken"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// A symlink whose target has been deleted should not be treated as a
    /// directory either.
    #[test]
    fn deleted_target_symlink_is_not_directory() {
        let temp_dir =
            std::env::temp_dir().join(format!("lez_test_deleted_symlink_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let target_dir = temp_dir.join("target_dir");
        std::fs::create_dir(&target_dir).unwrap();

        let link_path = temp_dir.join("dir-link");
        unix_fs::symlink("target_dir", &link_path).unwrap();

        // Verify it initially points to a directory
        let file = make_file(link_path.clone());
        assert!(
            file.points_to_directory(),
            "should point to directory before deletion"
        );

        // Delete the target directory
        std::fs::remove_dir(&target_dir).unwrap();

        // Re-create File to clear cached state
        let file = make_file(link_path);
        assert!(file.is_link(), "should still be recognized as a symlink");
        assert!(
            !file.points_to_directory(),
            "broken symlink (deleted target) should not point to a directory"
        );

        let target = file.link_target();
        assert!(
            target.is_broken(),
            "symlink with deleted target should be considered broken"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    /// The answer is cached after the first call, and only for symlinks --
    /// the sort asks it O(n log n) times, so it must give the same answer
    /// every time rather than only the first.
    #[test]
    fn repeated_calls_agree_for_every_kind_of_entry() {
        let temp_dir =
            std::env::temp_dir().join(format!("lez_test_ptd_cache_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let real_dir = temp_dir.join("real_dir");
        std::fs::create_dir(&real_dir).unwrap();
        let real_file = temp_dir.join("real_file");
        std::fs::write(&real_file, b"x").unwrap();

        let dir_link = temp_dir.join("dir_link");
        unix_fs::symlink(&real_dir, &dir_link).unwrap();
        let file_link = temp_dir.join("file_link");
        unix_fs::symlink(&real_file, &file_link).unwrap();

        for (path, expected) in [
            (real_dir, true),
            (real_file, false),
            (dir_link, true),
            (file_link, false),
        ] {
            let file = make_file(path.clone());
            for call in 0..4 {
                assert_eq!(
                    file.points_to_directory(),
                    expected,
                    "call {call} on {path:?} disagreed with the first"
                );
            }
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

#[cfg(all(test, unix))]
mod recursive_size_test {
    use super::File;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("lez_recsize_{label}_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn total_size_is_stable_across_cache_hits() {
        let dir = temp_dir("stable");
        fs::write(dir.join("one.bin"), vec![0u8; 4096]).unwrap();
        fs::write(dir.join("two.bin"), vec![0u8; 1024]).unwrap();

        let file = File::from_args(dir.clone(), None, None, false, true, false, None);
        let first = file.length();
        assert!(first >= 5120, "recursive size {first} below file bytes");

        // Second construction must hit DIRECTORY_SIZE_CACHE and agree.
        let file2 = File::from_args(dir.clone(), None, None, false, true, false, None);
        assert_eq!(first, file2.length());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn plain_files_do_not_get_recursive_size() {
        let dir = temp_dir("plain");
        fs::write(dir.join("file.txt"), b"data").unwrap();

        let file = File::from_args(dir.join("file.txt"), None, None, false, true, false, None);
        assert_eq!(file.length(), 4);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parent_aa_does_not_calculate_parent_hierarchy_size() {
        use crate::fs::dir::Dir;
        use crate::fs::fields as f;

        let parent = temp_dir("parent_aa_parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        // 1MB file in parent
        fs::write(parent.join("huge_parent.bin"), vec![0u8; 1024 * 1024]).unwrap();
        // 1KB file in child
        fs::write(child.join("small_child.bin"), vec![0u8; 1024]).unwrap();

        let child_dir = Dir::read_dir(child.clone()).unwrap();
        let aa_parent_file = File::new_aa_parent(parent.clone(), &child_dir, true, false, None);

        assert!(!aa_parent_file.is_recursive_size());
        assert!(matches!(aa_parent_file.size(), f::Size::None));

        let aa_current_file = File::new_aa_current(&child_dir, true, false, None);
        assert!(aa_current_file.is_recursive_size());
        assert!(aa_current_file.length() >= 1024);
        assert!(aa_current_file.length() < 1024 * 1024);

        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn dotfile_filter_synchronizes_with_recursive_size() {
        use crate::fs::DotFilter;

        let root = temp_dir("dotfile_sync");
        fs::write(root.join("visible.bin"), vec![0u8; 4096]).unwrap();
        fs::write(root.join(".hidden.bin"), vec![0u8; 8192]).unwrap();

        let hidden_subdir = root.join(".hidden_dir");
        fs::create_dir_all(&hidden_subdir).unwrap();
        fs::write(hidden_subdir.join("nested.bin"), vec![0u8; 16384]).unwrap();

        // Without dotfiles (DotFilter::JustFiles)
        let file_no_dots = File::from_args_with_filter(
            root.clone(),
            None,
            None,
            false,
            true,
            false,
            None,
            Some(DotFilter::JustFiles),
        );
        assert_eq!(file_no_dots.length(), 4096);

        // With dotfiles (DotFilter::Dotfiles)
        let file_with_dots = File::from_args_with_filter(
            root.clone(),
            None,
            None,
            false,
            true,
            false,
            None,
            Some(DotFilter::Dotfiles),
        );
        assert_eq!(file_with_dots.length(), 4096 + 8192 + 16384);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hardlinks_in_same_tree_are_deduplicated() {
        use crate::fs::DotFilter;

        let root = temp_dir("hardlinks_dedup");
        let file1 = root.join("file1.bin");
        fs::write(&file1, vec![0u8; 10000]).unwrap();

        let file1_hl = root.join("file1_hardlink.bin");
        fs::hard_link(&file1, &file1_hl).unwrap();

        let sub = root.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file1_hl2 = sub.join("file1_hardlink2.bin");
        fs::hard_link(&file1, &file1_hl2).unwrap();

        let file2 = root.join("file2.bin");
        fs::write(&file2, vec![0u8; 5000]).unwrap();

        let file = File::from_args_with_filter(
            root.clone(),
            None,
            None,
            false,
            true,
            false,
            None,
            Some(DotFilter::JustFiles),
        );
        assert_eq!(file.length(), 15000);

        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(all(test, unix))]
mod mime_type_test {
    use super::File;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "lez_mime_test_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_file_mimetype_detection_when_enabled() {
        let dir = temp_dir("enabled");
        let png_no_ext = dir.join("png_no_ext");
        fs::write(
            &png_no_ext,
            b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4",
        )
        .unwrap();

        let file_with_mime =
            File::from_args(png_no_ext.clone(), None, None, false, false, true, None);
        assert_eq!(file_with_mime.mimetype(), Some("image/png"));

        let file_without_mime = File::from_args(png_no_ext, None, None, false, false, false, None);
        assert_eq!(file_without_mime.mimetype(), None);

        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn windows_executable_detection_uses_pathext() {
        let dir = std::env::temp_dir().join(format!("lez_pathext_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let exe = File::from_args(dir.join("app.EXE"), None, None, false, false, false, None);
        assert!(exe.is_executable_file(), "PATHEXT lists .EXE");

        let txt = File::from_args(dir.join("notes.txt"), None, None, false, false, false, None);
        assert!(!txt.is_executable_file());

        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod is_empty_dir_test {
    use super::File;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "lez_empty_{tag}_{}_{}",
                std::process::id(),
                nanos
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(self, name: &str) -> Self {
            fs::write(self.0.join(name), b"").unwrap();
            self
        }

        fn subdir(self, name: &str) -> Self {
            fs::create_dir(self.0.join(name)).unwrap();
            self
        }

        fn is_empty(&self) -> bool {
            File::from_args(self.0.clone(), None, None, false, false, false, None).is_empty_dir()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_empty_directory_is_empty() {
        assert!(TempDir::new("empty").is_empty());
    }

    #[test]
    fn one_file_is_enough_to_make_it_not_empty() {
        assert!(!TempDir::new("one_file").file("a").is_empty());
    }

    /// The check reads the directory itself rather than a filtered listing, so
    /// this is the case that would break if the filtering were wrong: a
    /// hidden file still counts.
    #[test]
    fn a_hidden_file_still_counts() {
        assert!(!TempDir::new("hidden").file(".hidden").is_empty());
    }

    #[test]
    fn a_subdirectory_counts_too() {
        assert!(!TempDir::new("subdir").subdir("inner").is_empty());
    }

    #[test]
    fn a_file_is_not_an_empty_directory() {
        let dir = TempDir::new("not_a_dir").file("a");
        let file = File::from_args(dir.0.join("a"), None, None, false, false, false, None);
        assert!(!file.is_empty_dir());
    }
}

#[cfg(test)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod is_mount_point_test {
    use super::File;
    use std::path::PathBuf;

    fn mount_point(path: &str) -> bool {
        File::from_args(PathBuf::from(path), None, None, false, false, false, None).is_mount_point()
    }

    /// The root is in every mount table, and it is the one path with no last
    /// component — the case the name filter has to let through.
    #[test]
    fn the_root_is_a_mount_point() {
        assert!(mount_point("/"));
    }

    #[test]
    fn an_ordinary_directory_is_not() {
        let dir = std::env::temp_dir().join(format!("lez_mount_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        assert!(!mount_point(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_is_never_a_mount_point() {
        assert!(!mount_point("Cargo.toml"));
    }
}
