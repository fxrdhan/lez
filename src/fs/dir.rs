// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
use crate::fs::feature::git::GitCache;
use crate::fs::fields::GitStatus;
use std::collections::HashSet;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::{Path, PathBuf};
use std::slice::Iter as SliceIter;
use std::sync::OnceLock;

use log::info;

use crate::fs::File;

/// A **Dir** provides a cached list of the file paths in a directory that’s
/// being listed.
///
/// This object gets passed to the Files themselves, in order for them to
/// check the existence of surrounding files, then highlight themselves
/// accordingly. (See `File#get_source_files`)
#[derive(Debug)]
pub struct Dir {
    /// A vector of the files that have been read from this directory.
    contents: Vec<DirEntry>,

    /// The path that was read.
    pub path: PathBuf,

    /// The same paths as `contents`, in a form that can be searched in
    /// constant time. Built on first use, since most listings never ask.
    paths: OnceLock<HashSet<PathBuf>>,
}

impl Dir {
    /// Create a new, empty `Dir` object representing the directory at the given path.
    ///
    /// This function does not attempt to read the contents of the directory; it merely
    /// initializes an instance of `Dir` with an empty `DirEntry` list and the specified path.
    /// To populate the `Dir` object with actual directory contents, use the `read` function.
    pub fn new(path: PathBuf) -> Self {
        Self {
            contents: vec![],
            path,
            paths: OnceLock::new(),
        }
    }

    /// Reads the contents of the directory into `DirEntry`.
    ///
    /// It is recommended to use this method in conjunction with `new` in recursive
    /// calls, rather than `read_dir`, to avoid holding multiple open file descriptors
    /// simultaneously, which can lead to "too many open files" errors.
    pub fn read(&mut self) -> io::Result<&Self> {
        info!("Reading directory {:?}", self.path);

        self.contents = fs::read_dir(&self.path)?.collect::<Result<Vec<_>, _>>()?;
        // The contents just changed, so anything derived from them is stale.
        self.paths = OnceLock::new();

        info!("Read directory success {:?}", self.path);
        Ok(self)
    }

    /// Create a new Dir object filled with all the files in the directory
    /// pointed to by the given path. Fails if the directory can’t be read, or
    /// isn’t actually a directory, or if there’s an IO error that occurs at
    /// any point.
    ///
    /// The `read_dir` iterator doesn’t actually yield the `.` and `..`
    /// entries, so if the user wants to see them, we’ll have to add them
    /// ourselves after the files have been read.
    pub fn read_dir(path: PathBuf) -> io::Result<Self> {
        info!("Reading directory {:?}", path);

        let contents = fs::read_dir(&path)?.collect::<Result<Vec<_>, _>>()?;

        info!("Read directory success {:?}", path);
        Ok(Self {
            contents,
            path,
            paths: OnceLock::new(),
        })
    }

    /// Produce an iterator of IO results of trying to read all the files in
    /// this directory.
    #[must_use]
    pub fn files<'dir, 'ig>(
        &'dir self,
        dots: DotFilter,
        git: Option<&'ig GitCache>,
        git_ignoring: bool,
        deref_links: bool,
        total_size: bool,
        mime_read_contents: bool,
    ) -> Files<'dir, 'ig> {
        Files {
            inner: self.contents.iter(),
            dir: self,
            dotfiles: dots.shows_dotfiles(),
            #[cfg(windows)]
            windows_hidden: dots.shows_windows_hidden(),
            dots: dots.dots(),
            git,
            git_ignoring,
            deref_links,
            total_size,
            mime_read_contents,
        }
    }

    /// Whether this directory contains a file with the given path.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        self.paths
            .get_or_init(|| self.contents.iter().map(DirEntry::path).collect())
            .contains(path)
    }

    /// Append a path onto the path specified by this directory.
    #[must_use]
    pub fn join(&self, child: &Path) -> PathBuf {
        self.path.join(child)
    }
}

/// Iterator over reading the contents of a directory as `File` objects.
#[allow(clippy::struct_excessive_bools)]
pub struct Files<'dir, 'ig> {
    /// The internal iterator over the paths that have been read already.
    inner: SliceIter<'dir, DirEntry>,

    /// The directory that begat those paths.
    dir: &'dir Dir,

    /// Whether to include dotfiles in the list.
    dotfiles: bool,

    #[cfg(windows)]
    /// Whether Windows hidden-attribute entries should be visible.
    windows_hidden: bool,

    /// Whether the `.` or `..` directories should be produced first, before
    /// any files have been listed.
    dots: DotsNext,

    git: Option<&'ig GitCache>,

    git_ignoring: bool,

    /// Whether symbolic links should be dereferenced when querying information.
    deref_links: bool,

    /// Whether to calculate the directory size recursively
    total_size: bool,

    /// Whether to determine MIME types for styling decisions.
    mime_read_contents: bool,
}

impl<'dir> Files<'dir, '_> {
    fn parent(&self) -> PathBuf {
        // We can’t use `Path#parent` here because all it does is remove the
        // last path component, which is no good for us if the path is
        // relative. For example, while the parent of `/testcases/files` is
        // `/testcases`, the parent of `.` is an empty path. Adding `..` on
        // the end is the only way to get to the *actual* parent directory.
        self.dir.path.join("..")
    }

    /// Go through the directory until we encounter a file we can list (which
    /// varies depending on the dotfile visibility flag)
    fn next_visible_file(&mut self) -> Option<File<'dir>> {
        loop {
            if let Some(entry) = self.inner.next() {
                let path = entry.path();
                let filename = File::filename(&path);
                if !self.dotfiles && filename.starts_with('.') {
                    continue;
                }

                // Also hide _prefix files on Windows because it's used by old applications
                // as an alternative to dot-prefix files.
                #[cfg(windows)]
                if !self.dotfiles && filename.starts_with('_') {
                    continue;
                }

                if self.git_ignoring {
                    let git_status = self.git.map(|g| g.get(&path, false)).unwrap_or_default();
                    if git_status.unstaged == GitStatus::Ignored {
                        continue;
                    }
                }

                let file = File::from_args(
                    path,
                    self.dir,
                    filename,
                    self.deref_links,
                    self.total_size,
                    self.mime_read_contents,
                    entry.file_type().ok(),
                );

                // Windows has its own concept of hidden files, when dotfiles are
                // hidden Windows hidden files should also be filtered out
                #[cfg(windows)]
                if !self.windows_hidden && file.attributes().is_some_and(|a| a.hidden) {
                    continue;
                }

                return Some(file);
            }

            return None;
        }
    }
}

/// The dot directories that need to be listed before actual files, if any.
/// If these aren’t being printed, then `FilesNext` is used to skip them.
enum DotsNext {
    /// List the `.` directory next.
    Dot,

    /// List the `..` directory next.
    DotDot,

    /// Forget about the dot directories and just list files.
    Files,
}

impl<'dir> Iterator for Files<'dir, '_> {
    type Item = File<'dir>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.dots {
            DotsNext::Dot => {
                self.dots = DotsNext::DotDot;
                Some(File::new_aa_current(
                    self.dir,
                    self.total_size,
                    self.mime_read_contents,
                ))
            }

            DotsNext::DotDot => {
                self.dots = DotsNext::Files;
                Some(File::new_aa_parent(
                    self.parent(),
                    self.dir,
                    self.total_size,
                    self.mime_read_contents,
                ))
            }

            DotsNext::Files => self.next_visible_file(),
        }
    }
}

/// Usually files in Unix use a leading dot to be hidden or visible, but two
/// entries in particular are “extra-hidden”: `.` and `..`, which only become
/// visible after an extra `-a` option.
#[derive(PartialEq, Eq, Debug, Default, Copy, Clone)]
pub enum DotFilter {
    /// Shows files, dotfiles, and `.` and `..`.
    DotfilesAndDots,

    /// Show files and dotfiles, but hide `.` and `..`.
    Dotfiles,

    /// Show dotfiles by name only, but keep platform hidden-attribute files hidden.
    DotfilesByName,

    /// Just show files, hiding anything beginning with a dot.
    #[default]
    JustFiles,
}

impl DotFilter {
    /// Whether this filter should show dotfiles in a listing.
    fn shows_dotfiles(self) -> bool {
        match self {
            Self::JustFiles => false,
            Self::Dotfiles => true,
            Self::DotfilesByName => true,
            Self::DotfilesAndDots => true,
        }
    }
    #[cfg(windows)]
    /// Whether this filter should reveal Windows hidden-attribute entries.
    fn shows_windows_hidden(self) -> bool {
        cfg!(windows) && matches!(self, Self::Dotfiles | Self::DotfilesAndDots)
    }
    /// Whether this filter should add dot directories to a listing.
    fn dots(self) -> DotsNext {
        match self {
            Self::JustFiles => DotsNext::Files,
            Self::Dotfiles => DotsNext::Files,
            Self::DotfilesByName => DotsNext::Files,
            Self::DotfilesAndDots => DotsNext::Dot,
        }
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, GetFileAttributesW, SetFileAttributesW,
    };

    fn unique_temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("eza-show-dotfiles-{nanos}"));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        path
    }

    fn set_hidden(path: &Path) {
        let wide = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        unsafe {
            let attrs = GetFileAttributesW(wide.as_ptr());
            assert_ne!(attrs, u32::MAX);
            assert_ne!(
                SetFileAttributesW(wide.as_ptr(), attrs | FILE_ATTRIBUTE_HIDDEN),
                0
            );
        }
    }

    #[test]
    fn show_dotfiles_does_not_show_windows_hidden_attributes() {
        let path = unique_temp_dir();
        fs::write(path.join(".dotfile"), "").unwrap();
        fs::write(path.join("_underscore"), "").unwrap();
        fs::write(path.join("hidden.txt"), "").unwrap();
        set_hidden(&path.join("hidden.txt"));
        let dir = Dir::read_dir(path.clone()).unwrap();
        let names: Vec<_> = dir
            .files(DotFilter::DotfilesByName, None, false, false, false, false)
            .map(|file| file.name)
            .collect();
        assert!(names.contains(&".dotfile".to_string()));
        assert!(names.contains(&"_underscore".to_string()));
        assert!(!names.contains(&"hidden.txt".to_string()));
        let _ = fs::remove_dir_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File as StdFile;
    use std::io::Write;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("lsr_test_dir_{}_{}", name, std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn create_file(&self, name: &str) -> PathBuf {
            let file_path = self.path.join(name);
            let mut file = StdFile::create(&file_path).unwrap();
            file.write_all(b"test").unwrap();
            file_path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_dir_contains_finds_existing_file() {
        let test_dir = TestDir::new("contains_existing");
        let file_a = test_dir.create_file("foo.txt");
        let _file_b = test_dir.create_file("bar.rs");

        let dir = Dir::read_dir(test_dir.path.clone()).unwrap();
        assert!(dir.contains(&file_a));
    }

    #[test]
    fn test_dir_contains_returns_false_for_missing_file() {
        let test_dir = TestDir::new("contains_nonexistent");
        let _file_a = test_dir.create_file("foo.txt");

        let dir = Dir::read_dir(test_dir.path.clone()).unwrap();
        let non_existent = test_dir.path.join("nonexistent.txt");
        assert!(!dir.contains(&non_existent));
    }

    #[test]
    fn test_dir_contains_cache_invalidation_on_reread() {
        let test_dir = TestDir::new("cache_invalidation");
        let file_a = test_dir.create_file("foo.txt");

        let mut dir = Dir::read_dir(test_dir.path.clone()).unwrap();
        assert!(dir.contains(&file_a));

        let file_b = test_dir.path.join("bar.txt");
        // Verify before creation: false and cached
        assert!(!dir.contains(&file_b));

        // Now create file_b on disk
        test_dir.create_file("bar.txt");

        // Before re-read, dir.contains(&file_b) is still false because of cache
        assert!(!dir.contains(&file_b));

        // Re-read directory
        dir.read().unwrap();

        // After re-read, cache must be invalidated and contain file_b
        assert!(dir.contains(&file_b));
        assert!(dir.contains(&file_a));
    }

    #[test]
    fn test_dir_new_then_read() {
        let test_dir = TestDir::new("new_then_read");
        let file_a = test_dir.create_file("a.txt");

        let mut dir = Dir::new(test_dir.path.clone());
        assert!(!dir.contains(&file_a));

        dir.read().unwrap();
        assert!(dir.contains(&file_a));
    }

    #[test]
    fn test_empty_dir_contains_returns_false() {
        let test_dir = TestDir::new("empty_dir");
        let dir = Dir::read_dir(test_dir.path.clone()).unwrap();
        assert!(!dir.contains(&test_dir.path.join("anything.txt")));
    }
}
