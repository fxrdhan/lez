// SPDX-FileCopyrightText: 2024 Christina Sørensen
// SPDX-License-Identifier: EUPL-1.2
//
// SPDX-FileCopyrightText: 2023-2024 Christina Sørensen, eza contributors
// SPDX-FileCopyrightText: 2014 Benjamin Sago
// SPDX-License-Identifier: MIT
//! Getting the Git status of files and directories.

use std::env;
use std::ffi::OsStr;
#[cfg(target_family = "unix")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git2::StatusEntry;
use log::{debug, error, info, warn};

use crate::fs::fields as f;

/// A **Git cache** is assembled based on the user’s input arguments.
///
/// This uses vectors to avoid the overhead of hashing: it’s not worth it when the
/// expected number of Git repositories per exa invocation is 0 or 1...
pub struct GitCache {
    /// A list of discovered Git repositories and their paths.
    repos: Vec<GitRepo>,

    /// Paths that we’ve confirmed do not have Git repositories underneath them.
    misses: Vec<PathBuf>,

    /// Whether the status walk has to describe every file inside an untracked
    /// directory, rather than reporting the directory once. See
    /// `repo_to_statuses`.
    deep_untracked: bool,
}

impl GitCache {
    #[must_use]
    pub fn has_anything_for(&self, index: &Path) -> bool {
        self.repos.iter().any(|e| e.has_path(index))
    }

    #[must_use]
    pub fn get(&self, index: &Path, prefix_lookup: bool) -> f::Git {
        self.repos
            .iter()
            .find(|repo| repo.has_path(index))
            .map(|repo| repo.search(index, prefix_lookup))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn get_child(&self, dir: &Path, file: &Path, prefix_lookup: bool) -> f::Git {
        self.repos
            .iter()
            .find(|repo| repo.has_path(file) || repo.has_path(dir))
            .map(|repo| repo.search_child(dir, file, prefix_lookup))
            .unwrap_or_default()
    }

    /// Whether `path` sits inside a submodule working tree of any known
    /// repository.
    #[must_use]
    pub fn is_submodule_path(&self, path: &Path) -> bool {
        self.repos
            .iter()
            .find(|repo| repo.has_path(path))
            .is_some_and(|repo| repo.is_submodule_path(path))
    }
}

use std::iter::FromIterator;
impl GitCache {
    /// Build a cache over `paths`, telling the status walk whether it has to
    /// look inside untracked directories. It does when the listing recurses,
    /// or when `--git-ignore` has to judge each nested file; a flat listing
    /// only shows the directory itself.
    #[must_use]
    pub fn from_paths<I>(paths: I, deep_untracked: bool) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut cache: Self = paths.into_iter().collect();
        cache.deep_untracked = deep_untracked;
        for repo in &mut cache.repos {
            repo.deep_untracked = deep_untracked;
        }
        cache
    }
}

impl FromIterator<PathBuf> for GitCache {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let iter = iter.into_iter();
        let mut git = Self {
            repos: Vec::with_capacity(iter.size_hint().0),
            misses: Vec::new(),
            deep_untracked: true,
        };

        if let Ok(path) = env::var("GIT_DIR") {
            // These flags are consistent with how `git` uses GIT_DIR:
            let flags = git2::RepositoryOpenFlags::NO_SEARCH | git2::RepositoryOpenFlags::NO_DOTGIT;
            match GitRepo::discover(path.into(), flags, git.deep_untracked) {
                Ok(repo) => {
                    debug!("Opened GIT_DIR repo");
                    git.repos.push(repo);
                }
                Err(miss) => {
                    git.misses.push(miss);
                }
            }
        }

        for path in iter {
            if git.misses.contains(&path) {
                debug!("Skipping {path:?} because it already came back Gitless");
            } else if git.repos.iter().any(|e| e.has_path(&path)) {
                debug!("Skipping {path:?} because we already queried it");
            } else {
                let flags = git2::RepositoryOpenFlags::FROM_ENV;
                match GitRepo::discover(path, flags, git.deep_untracked) {
                    Ok(r) => {
                        if let Some(r2) = git.repos.iter_mut().find(|e| e.has_workdir(&r.workdir)) {
                            debug!(
                                "Adding to existing repo (workdir matches with {:?})",
                                r2.workdir
                            );
                            r2.extra_paths.push(r.original_path);
                            continue;
                        }

                        debug!("Discovered new Git repo");
                        git.repos.push(r);
                    }
                    Err(miss) => {
                        git.misses.push(miss);
                    }
                }
            }
        }

        git
    }
}

/// A **Git repository** is one we’ve discovered somewhere on the filesystem.
pub struct GitRepo {
    /// The queryable contents of the repository: either a `git2` repo, or the
    /// cached results from when we queried it last time.
    contents: Mutex<GitContents>,

    /// The working directory of this repository.
    /// This is used to check whether two repositories are the same.
    workdir: PathBuf,

    /// The same directory, canonicalised once, so that the ignore-rule
    /// fallback can make paths relative without a syscall per lookup.
    workdir_canonical: PathBuf,

    /// The path that was originally checked to discover this repository.
    /// This is as important as the `extra_paths` (it gets checked first), but
    /// is separate to avoid having to deal with a non-empty Vec.
    original_path: PathBuf,

    /// Any other paths that were checked only to result in this same
    /// repository.
    extra_paths: Vec<PathBuf>,

    /// Cached relative paths of this repository's submodules, used by
    /// `--ignore-submodule-contents` to prune recursion. `None` until the
    /// first query.
    submodules: Mutex<Option<Vec<PathBuf>>>,

    /// Passed to the status walk; see `GitCache::deep_untracked`.
    deep_untracked: bool,
}

/// A repository’s queried state.
enum GitContents {
    /// All the interesting Git stuff goes through this.
    Before { repo: git2::Repository },

    /// Temporary value used in `repo_to_statuses` so we can move the
    /// repository out of the `Before` variant.
    Processing,

    /// The data we’ve extracted from the repository, but only after we’ve
    /// actually done so. The repository is kept so that paths the status
    /// walk never reported can still be checked against the ignore rules.
    After {
        repo: git2::Repository,
        statuses: Git,
    },
}

impl GitRepo {
    /// Whether `path` lies inside one of this repository's submodule
    /// working trees. The list is discovered lazily via git2 and cached.
    pub fn is_submodule_path(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.workdir) else {
            return false;
        };
        let Ok(mut guard) = self.submodules.lock() else {
            return false;
        };
        let workdir = self.workdir.clone();
        let submodules = guard.get_or_insert_with(|| {
            let mut out = Vec::new();
            if let Ok(repo) = git2::Repository::open(&workdir)
                && let Ok(submodules) = repo.submodules()
            {
                for sm in submodules {
                    out.push(sm.path().to_path_buf());
                }
            }
            out
        });
        submodules.iter().any(|sm| relative.starts_with(sm))
    }

    /// Searches through this repository for a path (to a file or directory,
    /// depending on the prefix-lookup flag) and returns its Git status.
    ///
    /// Actually querying the `git2` repository for the mapping of paths to
    /// Git statuses is only done once, and gets cached so we don’t need to
    /// re-query the entire repository the times after that.
    ///
    /// The temporary `Processing` enum variant is used after the `git2`
    /// repository is moved out, but before the results have been moved in!
    /// See <https://stackoverflow.com/q/45985827/3484614>
    fn search(&self, index: &Path, prefix_lookup: bool) -> f::Git {
        use std::mem::replace;

        let mut contents = self.contents.lock().unwrap();
        if let GitContents::After {
            ref repo,
            ref statuses,
        } = *contents
        {
            debug!("Git repo {:?} has been found in cache", self.workdir);
            let result = statuses.status(index, prefix_lookup);
            return self.rescue_unreported_ignore(repo, result, prefix_lookup, index);
        }

        debug!("Querying Git repo {:?} for the first time", self.workdir);
        let repo = replace(&mut *contents, GitContents::Processing).inner_repo();
        let statuses = repo_to_statuses(
            &repo,
            &self.workdir,
            &self.listing_roots(),
            self.deep_untracked,
        );
        let result = statuses.status(index, prefix_lookup);
        let result = self.rescue_unreported_ignore(&repo, result, prefix_lookup, index);
        let _processing = replace(&mut *contents, GitContents::After { repo, statuses });
        result
    }

    fn search_child(&self, dir: &Path, file: &Path, prefix_lookup: bool) -> f::Git {
        use std::mem::replace;

        let mut contents = self.contents.lock().unwrap();
        if let GitContents::After {
            ref repo,
            ref statuses,
        } = *contents
        {
            debug!("Git repo {:?} has been found in cache", self.workdir);
            let result = statuses.child_status(dir, file, prefix_lookup);
            return self.rescue_unreported_child_ignore(repo, result, prefix_lookup, dir, file);
        }

        debug!("Querying Git repo {:?} for the first time", self.workdir);
        let repo = replace(&mut *contents, GitContents::Processing).inner_repo();
        let statuses = repo_to_statuses(
            &repo,
            &self.workdir,
            &self.listing_roots(),
            self.deep_untracked,
        );
        let result = statuses.child_status(dir, file, prefix_lookup);
        let result = self.rescue_unreported_child_ignore(&repo, result, prefix_lookup, dir, file);
        let _processing = replace(&mut *contents, GitContents::After { repo, statuses });
        result
    }

    /// libgit2 leaves an ignored file out of the status walk altogether when
    /// the directory holding it is itself ignored, which a lone `*` in
    /// `.gitignore` always causes (libgit2#6890). Such a file reaches us with
    /// no status at all, so `--git` showed no `I` for it and `--git-ignore`
    /// went on displaying it. Ask the ignore rules about those paths directly.
    ///
    /// `recurse_ignored_dirs` would also fix it, but it walks every ignored
    /// directory — on a repository with a large `target/` that turns a listing
    /// from hundredths of a second into half a minute — and it stops reporting
    /// the ignored directories themselves.
    fn rescue_unreported_ignore(
        &self,
        repo: &git2::Repository,
        result: f::Git,
        prefix_lookup: bool,
        path: &Path,
    ) -> f::Git {
        if prefix_lookup || !is_unreported(result) || !self.ignored_by_rules(repo, path) {
            return result;
        }

        f::Git {
            unstaged: f::GitStatus::Ignored,
            ..result
        }
    }

    /// The same rescue for a file reached through its parent directory.
    /// Listing an ignored directory does not mark everything inside it as
    /// ignored — `child_status` filters the parent's own rule out on purpose —
    /// so this only applies when the directory itself is not ignored.
    fn rescue_unreported_child_ignore(
        &self,
        repo: &git2::Repository,
        result: f::Git,
        prefix_lookup: bool,
        dir: &Path,
        file: &Path,
    ) -> f::Git {
        if self.ignored_by_rules(repo, dir) {
            return result;
        }

        self.rescue_unreported_ignore(repo, result, prefix_lookup, file)
    }

    /// Whether Git's ignore rules cover this path. Only untracked paths
    /// qualify: Git never ignores a file that is in the index, however well
    /// it matches a pattern.
    fn ignored_by_rules(&self, repo: &git2::Repository, path: &Path) -> bool {
        let path = reorient(path);
        let Ok(relative) = path.strip_prefix(&self.workdir_canonical) else {
            return false;
        };
        if relative.as_os_str().is_empty() {
            return false;
        }
        if repo
            .index()
            .is_ok_and(|index| index.get_path(relative, 0).is_some())
        {
            return false;
        }

        repo.is_path_ignored(relative).unwrap_or(false)
    }

    /// The absolute paths of every listing that resolved to this repository:
    /// status queries only ever concern paths beneath these (see `has_path`).
    fn listing_roots(&self) -> Vec<PathBuf> {
        std::iter::once(&self.original_path)
            .chain(self.extra_paths.iter())
            .map(|p| reorient(p))
            .collect()
    }

    /// Whether this repository has the given working directory.
    fn has_workdir(&self, path: &Path) -> bool {
        self.workdir == path
    }

    /// Whether this repository cares about the given path at all.
    fn has_path(&self, path: &Path) -> bool {
        path.starts_with(&self.original_path)
            || self.extra_paths.iter().any(|e| path.starts_with(e))
    }

    /// Open a Git repository. Depending on the flags, the path is either
    /// the repository's "gitdir" (or a "gitlink" to the gitdir), or the
    /// path is the start of a rootwards search for the repository.
    fn discover(
        path: PathBuf,
        flags: git2::RepositoryOpenFlags,
        deep_untracked: bool,
    ) -> Result<Self, PathBuf> {
        info!("Opening Git repository for {path:?} ({flags:?})");
        let unused: [&OsStr; 0] = [];
        let repo = match git2::Repository::open_ext(&path, flags, unused) {
            Ok(r) => r,
            Err(e) => {
                error!("Error opening Git repository for {path:?}: {e:?}");
                return Err(path);
            }
        };

        if let Some(workdir) = repo.workdir() {
            let workdir = workdir.to_path_buf();
            let contents = Mutex::new(GitContents::Before { repo });
            Ok(Self {
                contents,
                workdir_canonical: reorient(&workdir),
                workdir,
                original_path: path,
                extra_paths: Vec::new(),
                submodules: Mutex::new(None),
                deep_untracked,
            })
        } else {
            warn!("Repository has no workdir?");
            Err(path)
        }
    }
}

impl GitContents {
    /// Assumes that the repository hasn’t been queried, and extracts it
    /// (consuming the value) if it has. This is needed because the entire
    /// enum variant gets replaced when a repo is queried (see above).
    fn inner_repo(self) -> git2::Repository {
        if let Self::Before { repo } = self {
            repo
        } else {
            unreachable!("Tried to extract a non-Repository")
        }
    }
}

/// Iterates through a repository’s statuses, consuming it and returning the
/// mapping of files to their Git status.
/// We will have already used the working directory at this point, so it gets
/// passed in rather than deriving it from the `Repository` again.
fn repo_to_statuses(
    repo: &git2::Repository,
    workdir: &Path,
    roots: &[PathBuf],
    deep_untracked: bool,
) -> Git {
    let mut statuses = Vec::new();

    info!("Getting Git statuses for repo with workdir {workdir:?}");

    // Mirror `GIT_STATUS_OPT_DEFAULTS`, which libgit2 applies when given no options.
    let mut options = git2::StatusOptions::new();
    options.include_ignored(true).include_untracked(true);

    // Limit the scan to the listed paths: a small corner of a large repository
    // should not pay for a scan of the whole working tree. Roots at or outside
    // the workdir fall back to a full scan.
    let workdir_canonical = reorient(workdir);
    let pathspecs: Option<Vec<&Path>> = roots
        .iter()
        .map(|root| match root.strip_prefix(&workdir_canonical) {
            Ok(rel) if !rel.as_os_str().is_empty() => Some(rel),
            _ => None,
        })
        .collect();
    let limited = pathspecs.is_some();
    if let Some(pathspecs) = pathspecs {
        debug!("Limiting Git status scan to {pathspecs:?}");
        for spec in pathspecs {
            options.pathspec(spec);
        }
    }

    // Describing every file inside an untracked directory is what makes a
    // status walk expensive on a repository carrying thousands of them, and a
    // flat listing of the working tree does not need it: it shows the
    // directory, and one entry answers that. `git status` collapses them for
    // the same reason.
    //
    // It is needed when the listing recurses, when --git-ignore has to judge
    // each nested file, and — the case that is easy to miss — when an
    // untracked directory is itself what was asked for, since then its
    // contents are the listing. That last one is covered by the pathspec: a
    // limited scan only walks what was named, so recursing inside it is
    // bounded and cheap. The unlimited scan is the expensive one, and the only
    // one worth narrowing.
    options.recurse_untracked_dirs(deep_untracked || limited);

    match repo.statuses(Some(&mut options)) {
        Ok(es) => {
            for e in es.iter() {
                if let Some(p) = get_path_from_status_entry(&e) {
                    let elem = (workdir.join(p), e.status());
                    statuses.push(elem);
                }
            }
            // We manually add the `.git` at the root of the repo as ignored, since it is in practice.
            // Also we want to avoid `eza --tree --all --git-ignore` to display files inside `.git`.
            statuses.push((workdir.join(".git"), git2::Status::IGNORED));
        }
        Err(e) => {
            error!("Error looking up Git statuses: {e:?}");
        }
    }

    Git { statuses }
}

#[allow(clippy::unnecessary_wraps)]
fn get_path_from_status_entry(e: &StatusEntry<'_>) -> Option<PathBuf> {
    #[cfg(target_family = "unix")]
    return Some(PathBuf::from(OsStr::from_bytes(e.path_bytes())));
    #[cfg(not(target_family = "unix"))]
    // In git2 0.21, `path` became fallible for non-UTF-8 paths.
    return if let Ok(p) = e.path() {
        Some(PathBuf::from(p))
    } else {
        info!("Git status ignored for non UTF-8 path {:?}", e.path_bytes());
        None
    };
}

// The `repo.statuses` call above takes a long time. exa debug output:
//
//   20.311276  INFO:exa::fs::feature::git: Getting Git statuses for repo with workdir "/vagrant/"
//   20.799610  DEBUG:exa::output::table: Getting Git status for file "./Cargo.toml"
//
// Even inserting another logging line immediately afterwards doesn’t make it
// look any faster.

/// Container of Git statuses for all the files in this folder’s Git repository.
struct Git {
    statuses: Vec<(PathBuf, git2::Status)>,
}

impl Git {
    /// Get either the file or directory status for the given path.
    /// “Prefix lookup” means that it should report an aggregate status of all
    /// paths starting with the given prefix (in other words, a directory).
    fn status(&self, index: &Path, prefix_lookup: bool) -> f::Git {
        if prefix_lookup {
            self.dir_status(index)
        } else {
            self.file_status(index)
        }
    }

    fn child_status(&self, dir: &Path, file: &Path, prefix_lookup: bool) -> f::Git {
        let dir_path = reorient(dir);
        let path = reorient(file);

        let s = self
            .statuses
            .iter()
            .filter(|p| {
                if p.1 == git2::Status::IGNORED {
                    if dir_path.starts_with(&p.0) {
                        false
                    } else {
                        path.starts_with(&p.0)
                    }
                } else if prefix_lookup {
                    p.0.starts_with(&path)
                } else {
                    p.0 == path
                }
            })
            .fold(git2::Status::empty(), |a, b| a | b.1);

        let staged = index_status(s);
        let unstaged = working_tree_status(s);
        f::Git { staged, unstaged }
    }

    /// Get the user-facing status of a file.
    /// We check the statuses directly applying to a file, and for the ignored
    /// status we check if any of its parents directories is ignored by git.
    fn file_status(&self, file: &Path) -> f::Git {
        let path = reorient(file);

        let s = self
            .statuses
            .iter()
            .filter(|p| {
                if p.1 == git2::Status::IGNORED {
                    path.starts_with(&p.0)
                } else {
                    p.0 == path
                }
            })
            .fold(git2::Status::empty(), |a, b| a | b.1);

        let staged = index_status(s);
        let unstaged = working_tree_status(s);
        f::Git { staged, unstaged }
    }

    /// Get the combined, user-facing status of a directory.
    /// Statuses are aggregating (for example, a directory is considered
    /// modified if any file under it has the status modified), except for
    /// ignored status which applies to files under (for example, a directory
    /// is considered ignored if one of its parent directories is ignored).
    fn dir_status(&self, dir: &Path) -> f::Git {
        let path = reorient(dir);

        let s = self
            .statuses
            .iter()
            .filter(|p| {
                if p.1 == git2::Status::IGNORED {
                    path.starts_with(&p.0)
                } else {
                    p.0.starts_with(&path)
                }
            })
            .fold(git2::Status::empty(), |a, b| a | b.1);

        let staged = index_status(s);
        let unstaged = working_tree_status(s);
        f::Git { staged, unstaged }
    }
}

/// Converts a path to an absolute path based on the current directory.
/// Paths need to be absolute for them to be compared properly, otherwise
/// you’d ask a repo about “./README.md” but it only knows about
/// “/vagrant/README.md”, prefixed by the workdir.
///
/// Note: only the parent directory is canonicalized, preserving the leaf
/// file or symlink name without following symlink targets.
#[cfg(unix)]
fn reorient(path: &Path) -> PathBuf {
    use std::env::current_dir;

    let full_path = match current_dir() {
        Ok(dir) => dir.join(path),
        Err(_) => Path::new(".").join(path),
    };

    if let (Some(parent), Some(file_name)) = (full_path.parent(), full_path.file_name())
        && let Ok(canon_parent) = parent.canonicalize()
    {
        return canon_parent.join(file_name);
    }

    full_path.canonicalize().unwrap_or(full_path)
}

#[cfg(windows)]
fn reorient(path: &Path) -> PathBuf {
    use std::env::current_dir;

    let full_path = match current_dir() {
        Ok(dir) => dir.join(path),
        Err(_) => Path::new(".").join(path),
    };

    let p = if let (Some(parent), Some(file_name)) = (full_path.parent(), full_path.file_name())
        && let Ok(canon_parent) = parent.canonicalize()
    {
        canon_parent.join(file_name)
    } else {
        full_path.canonicalize().unwrap_or(full_path)
    };

    // On Windows UNC path is returned. We need to strip the prefix for it to work.
    match p.to_str() {
        Some(text) => PathBuf::from(text.trim_start_matches(r"\\?\")),
        None => p,
    }
}

/// The character to display if the file has been modified, but not staged.
fn working_tree_status(status: git2::Status) -> f::GitStatus {
    #[rustfmt::skip]
    return match status {
        s if s.contains(git2::Status::WT_NEW)         => f::GitStatus::New,
        s if s.contains(git2::Status::WT_MODIFIED)    => f::GitStatus::Modified,
        s if s.contains(git2::Status::WT_DELETED)     => f::GitStatus::Deleted,
        s if s.contains(git2::Status::WT_RENAMED)     => f::GitStatus::Renamed,
        s if s.contains(git2::Status::WT_TYPECHANGE)  => f::GitStatus::TypeChange,
        s if s.contains(git2::Status::IGNORED)        => f::GitStatus::Ignored,
        s if s.contains(git2::Status::CONFLICTED)     => f::GitStatus::Conflicted,
        _                                             => f::GitStatus::NotModified,
    };
}

/// Whether the status walk had nothing at all to say about a path. A clean
/// tracked file looks the same, which is why the ignore-rule fallback checks
/// the index before it answers.
fn is_unreported(status: f::Git) -> bool {
    status.staged == f::GitStatus::NotModified && status.unstaged == f::GitStatus::NotModified
}

/// The character to display if the file has been modified and the change
/// has been staged.
fn index_status(status: git2::Status) -> f::GitStatus {
    #[rustfmt::skip]
    return match status {
        s if s.contains(git2::Status::INDEX_NEW)         => f::GitStatus::New,
        s if s.contains(git2::Status::INDEX_MODIFIED)    => f::GitStatus::Modified,
        s if s.contains(git2::Status::INDEX_DELETED)     => f::GitStatus::Deleted,
        s if s.contains(git2::Status::INDEX_RENAMED)     => f::GitStatus::Renamed,
        s if s.contains(git2::Status::INDEX_TYPECHANGE)  => f::GitStatus::TypeChange,
        _                                                => f::GitStatus::NotModified,
    };
}

fn current_branch(repo: &git2::Repository) -> Option<String> {
    let head = match repo.head() {
        Ok(head) => Some(head),
        Err(ref e)
            if e.code() == git2::ErrorCode::UnbornBranch
                || e.code() == git2::ErrorCode::NotFound =>
        {
            return None;
        }
        Err(e) => {
            error!("Error looking up Git branch: {e:?}");
            return None;
        }
    };

    // In git2 0.21, `shorthand` became fallible for non-UTF-8 branch names.
    head.and_then(|h| h.shorthand().ok().map(std::string::ToString::to_string))
}

impl f::SubdirGitRepo {
    #[must_use]
    pub fn from_path(dir: &Path, status: bool) -> Self {
        if dir.file_name() == Some(std::ffi::OsStr::new(".git")) || dir.ends_with(".git") {
            return f::SubdirGitRepo {
                status: if status {
                    Some(f::SubdirGitRepoStatus::NoRepo)
                } else {
                    None
                },
                branch: None,
                is_worktree: false,
            };
        }

        let path = &reorient(dir);

        let git_file = dir.join(".git");
        let is_gitlink_worktree = git_file.is_file()
            && std::fs::read_to_string(&git_file)
                .map(|content| {
                    let trimmed = content.trim_start();
                    trimmed.starts_with("gitdir:")
                        && (trimmed.contains("/worktrees/") || trimmed.contains(r"\worktrees\"))
                })
                .unwrap_or(false);

        if let Ok(repo) = git2::Repository::open(path) {
            let is_worktree = repo.is_worktree() || is_gitlink_worktree;
            let branch = current_branch(&repo);
            if !status {
                return Self {
                    status: None,
                    branch,
                    is_worktree,
                };
            }
            match repo.statuses(None) {
                Ok(es) => {
                    if es.iter().any(|s| s.status() != git2::Status::IGNORED) {
                        return Self {
                            status: Some(f::SubdirGitRepoStatus::GitDirty),
                            branch,
                            is_worktree,
                        };
                    }
                    return Self {
                        status: Some(f::SubdirGitRepoStatus::GitClean),
                        branch,
                        is_worktree,
                    };
                }
                Err(e) => {
                    error!("Error looking up Git statuses: {e:?}");
                }
            }
        }
        f::SubdirGitRepo {
            status: if status {
                Some(f::SubdirGitRepoStatus::NoRepo)
            } else {
                None
            },
            branch: None,
            is_worktree: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File as StdFile};
    use std::io::Write;

    struct TestGitRepo {
        path: PathBuf,
    }

    impl TestGitRepo {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("lsr_test_git_{}_{}", name, std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();

            let repo = git2::Repository::init(&path).unwrap();
            let workdir = repo.workdir().unwrap().to_path_buf();
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            Self { path: workdir }
        }

        fn create_file(&self, rel_path: &str, content: &[u8]) -> PathBuf {
            let file_path = self.path.join(rel_path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut file = StdFile::create(&file_path).unwrap();
            file.write_all(content).unwrap();
            file_path
        }

        #[cfg(unix)]
        fn create_symlink(&self, target: &str, link_rel_path: &str) -> PathBuf {
            let link_path = self.path.join(link_rel_path);
            if let Some(parent) = link_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            std::os::unix::fs::symlink(target, &link_path).unwrap();
            link_path
        }

        fn add_path_to_index(&self, rel_path: &str) {
            let repo = git2::Repository::open(&self.path).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(rel_path)).unwrap();
            index.write().unwrap();
        }

        #[cfg(unix)]
        fn remove_path_from_index(&self, rel_path: &str) {
            let repo = git2::Repository::open(&self.path).unwrap();
            let mut index = repo.index().unwrap();
            index.remove_path(Path::new(rel_path)).unwrap();
            index.write().unwrap();
        }

        fn commit_all(&self, msg: &str) {
            let repo = git2::Repository::open(&self.path).unwrap();
            let mut index = repo.index().unwrap();
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write().unwrap();

            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();

            let parent_commit = match repo.head() {
                Ok(head) => head.peel_to_commit().ok(),
                Err(_) => None,
            };

            let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
            repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
                .unwrap();
        }
    }

    impl Drop for TestGitRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_listing_roots_collects_reoriented_paths() {
        let test_repo = TestGitRepo::new("listing_roots");
        let sub_a = test_repo.path.join("sub_a");
        let sub_b = test_repo.path.join("sub_b");
        fs::create_dir_all(&sub_a).unwrap();
        fs::create_dir_all(&sub_b).unwrap();

        let repo = GitRepo {
            contents: Mutex::new(GitContents::Processing),
            workdir_canonical: reorient(&test_repo.path),
            workdir: test_repo.path.clone(),
            original_path: sub_a.clone(),
            extra_paths: vec![sub_b.clone()],
            submodules: Mutex::new(None),
            deep_untracked: true,
        };

        let roots = repo.listing_roots();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], reorient(&sub_a));
        assert_eq!(roots[1], reorient(&sub_b));
    }

    #[test]
    fn test_scoped_status_queries_only_targets_subpath() {
        let test_repo = TestGitRepo::new("scoped_subpath");
        let file_root = test_repo.create_file("root.txt", b"root init\n");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        let file_b = test_repo.create_file("sub_b/file_b.txt", b"b init\n");

        test_repo.commit_all("initial commit");

        // Modify all files in working directory
        let mut f_root = StdFile::create(&file_root).unwrap();
        f_root.write_all(b"root modified\n").unwrap();
        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();
        let mut f_b = StdFile::create(&file_b).unwrap();
        f_b.write_all(b"b modified\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let sub_a_path = test_repo.path.join("sub_a");

        // Query status scoped strictly to sub_a
        let roots = vec![reorient(&sub_a_path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // Status for sub_a/file_a.txt should be Modified
        let file_a_status = git_status.file_status(&file_a);
        assert!(file_a_status.unstaged == f::GitStatus::Modified);

        // Status for sub_a directory should be Modified
        let dir_a_status = git_status.dir_status(&sub_a_path);
        assert!(dir_a_status.unstaged == f::GitStatus::Modified);

        // Because query was scoped to sub_a, file_b and root.txt should NOT be in scanned statuses
        let file_b_status = git_status.file_status(&file_b);
        assert!(file_b_status.unstaged == f::GitStatus::NotModified);

        let root_status = git_status.file_status(&file_root);
        assert!(root_status.unstaged == f::GitStatus::NotModified);
    }

    #[test]
    fn test_scoped_status_fallback_on_repo_root() {
        let test_repo = TestGitRepo::new("fallback_root");
        let file_root = test_repo.create_file("root.txt", b"root init\n");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        let file_b = test_repo.create_file("sub_b/file_b.txt", b"b init\n");

        test_repo.commit_all("initial commit");

        // Modify all files
        let mut f_root = StdFile::create(&file_root).unwrap();
        f_root.write_all(b"root modified\n").unwrap();
        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();
        let mut f_b = StdFile::create(&file_b).unwrap();
        f_b.write_all(b"b modified\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();

        // Listing root is repository root (workdir) -> triggers full scan fallback
        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // All modified files must be detected
        assert!(git_status.file_status(&file_root).unstaged == f::GitStatus::Modified);
        assert!(git_status.file_status(&file_a).unstaged == f::GitStatus::Modified);
        assert!(git_status.file_status(&file_b).unstaged == f::GitStatus::Modified);
    }

    #[test]
    fn test_scoped_status_fallback_on_outside_path() {
        let test_repo = TestGitRepo::new("fallback_outside");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        test_repo.commit_all("initial commit");

        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();

        // Listing root is outside the repo workdir -> triggers full scan fallback
        let outside_path = std::env::temp_dir();
        let roots = vec![reorient(&outside_path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        assert!(git_status.file_status(&file_a).unstaged == f::GitStatus::Modified);
    }

    /// A lone `*` makes libgit2 drop ignored files from the status walk
    /// entirely (libgit2#6890), so the `I` never reached the Git column and
    /// `--git-ignore` never hid the file. Ignored directories were reported
    /// all along and have to keep being reported.
    #[test]
    fn lone_star_gitignore_still_marks_files_ignored() {
        let test_repo = TestGitRepo::new("lone_star");
        test_repo.create_file(".gitignore", b"*\n!.gitignore\n!kept.txt\n");
        let kept = test_repo.create_file("kept.txt", b"kept\n");
        let dropped = test_repo.create_file("dropped.log", b"dropped\n");
        let sub_file = test_repo.create_file("sub/nested.log", b"nested\n");
        let sub = test_repo.path.join("sub");

        let git_cache = GitCache::from_iter(vec![test_repo.path.clone()]);

        assert!(
            git_cache.get(&dropped, false).unstaged == f::GitStatus::Ignored,
            "a file the status walk never reported is still ignored"
        );
        assert!(
            git_cache.get(&sub, true).unstaged == f::GitStatus::Ignored,
            "ignored directories were already reported and must stay reported"
        );
        assert!(
            git_cache.get(&kept, false).unstaged == f::GitStatus::New,
            "a negated file is not ignored"
        );
        assert!(
            git_cache
                .get_child(&test_repo.path, &dropped, false)
                .unstaged
                == f::GitStatus::Ignored,
            "the same holds when the file is reached through its parent"
        );

        // Listing the ignored directory itself does not paint its contents
        // ignored — you already know where you are.
        assert!(
            git_cache.get_child(&sub, &sub_file, false).unstaged != f::GitStatus::Ignored,
            "contents of a directory being listed are not marked by its own rule"
        );
    }

    /// Git never ignores a file that is in the index, however well it matches
    /// a pattern. Without the index check the rescue would mark every
    /// force-added file as ignored, since the rules alone still match it.
    #[test]
    fn a_tracked_file_matching_the_pattern_is_not_ignored() {
        let test_repo = TestGitRepo::new("lone_star_tracked");
        test_repo.create_file(".gitignore", b"*\n!.gitignore\n");
        let tracked = test_repo.create_file("tracked.log", b"tracked\n");
        let untracked = test_repo.create_file("untracked.log", b"untracked\n");
        test_repo.add_path_to_index("tracked.log");
        test_repo.commit_all("force-add an ignored file");

        let git_cache = GitCache::from_iter(vec![test_repo.path.clone()]);

        assert!(
            git_cache.get(&tracked, false).unstaged == f::GitStatus::NotModified,
            "a tracked file stays tracked even though `*` matches it"
        );
        assert!(
            git_cache.get(&untracked, false).unstaged == f::GitStatus::Ignored,
            "its untracked neighbour is still ignored"
        );
    }

    #[test]
    fn test_git_cache_end_to_end_with_scoped_path() {
        let test_repo = TestGitRepo::new("cache_e2e");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        let file_b = test_repo.create_file("sub_b/file_b.txt", b"b init\n");
        test_repo.commit_all("initial commit");

        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();
        let mut f_b = StdFile::create(&file_b).unwrap();
        f_b.write_all(b"b modified\n").unwrap();

        let sub_a_path = test_repo.path.join("sub_a");
        let git_cache = GitCache::from_iter(vec![sub_a_path.clone()]);

        assert!(git_cache.has_anything_for(&sub_a_path));
        let status_a = git_cache.get(&file_a, false);
        assert!(status_a.unstaged == f::GitStatus::Modified);

        let dir_status_a = git_cache.get(&sub_a_path, true);
        assert!(dir_status_a.unstaged == f::GitStatus::Modified);
    }

    #[test]
    fn test_scoped_status_multiple_subpaths() {
        let test_repo = TestGitRepo::new("multi_subpath");
        let file_root = test_repo.create_file("root.txt", b"root init\n");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        let file_b = test_repo.create_file("sub_b/file_b.txt", b"b init\n");
        let file_c = test_repo.create_file("sub_c/file_c.txt", b"c init\n");

        test_repo.commit_all("initial commit");

        // Modify all files
        let mut f_root = StdFile::create(&file_root).unwrap();
        f_root.write_all(b"root modified\n").unwrap();
        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();
        let mut f_b = StdFile::create(&file_b).unwrap();
        f_b.write_all(b"b modified\n").unwrap();
        let mut f_c = StdFile::create(&file_c).unwrap();
        f_c.write_all(b"c modified\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let sub_a_path = test_repo.path.join("sub_a");
        let sub_b_path = test_repo.path.join("sub_b");

        // Scoped to sub_a and sub_b
        let roots = vec![reorient(&sub_a_path), reorient(&sub_b_path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // sub_a and sub_b files should be modified
        assert!(git_status.file_status(&file_a).unstaged == f::GitStatus::Modified);
        assert!(git_status.file_status(&file_b).unstaged == f::GitStatus::Modified);

        // sub_c and root.txt should not be in the scoped scan
        assert!(git_status.file_status(&file_c).unstaged == f::GitStatus::NotModified);
        assert!(git_status.file_status(&file_root).unstaged == f::GitStatus::NotModified);
    }

    #[test]
    fn test_scoped_status_untracked_and_ignored() {
        let test_repo = TestGitRepo::new("untracked_ignored");
        test_repo.create_file(".gitignore", b"*.ignored\n");
        test_repo.commit_all("add gitignore");

        let untracked_a = test_repo.create_file("sub_a/untracked.txt", b"untracked");
        let ignored_a = test_repo.create_file("sub_a/test.ignored", b"ignored");
        let untracked_b = test_repo.create_file("sub_b/untracked.txt", b"untracked");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let sub_a_path = test_repo.path.join("sub_a");

        let roots = vec![reorient(&sub_a_path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // Within sub_a: untracked is New, ignored is Ignored
        assert!(git_status.file_status(&untracked_a).unstaged == f::GitStatus::New);
        assert!(git_status.file_status(&ignored_a).unstaged == f::GitStatus::Ignored);

        // Outside sub_a: untracked_b was not scanned
        assert!(git_status.file_status(&untracked_b).unstaged == f::GitStatus::NotModified);
    }

    #[test]
    fn test_scoped_status_mixed_roots_fallback() {
        let test_repo = TestGitRepo::new("mixed_fallback");
        let file_root = test_repo.create_file("root.txt", b"root init\n");
        let file_a = test_repo.create_file("sub_a/file_a.txt", b"a init\n");
        let file_b = test_repo.create_file("sub_b/file_b.txt", b"b init\n");

        test_repo.commit_all("initial commit");

        let mut f_root = StdFile::create(&file_root).unwrap();
        f_root.write_all(b"root modified\n").unwrap();
        let mut f_a = StdFile::create(&file_a).unwrap();
        f_a.write_all(b"a modified\n").unwrap();
        let mut f_b = StdFile::create(&file_b).unwrap();
        f_b.write_all(b"b modified\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let sub_a_path = test_repo.path.join("sub_a");

        // Passing both a subpath and repo root triggers full scan fallback
        let roots = vec![reorient(&sub_a_path), reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        assert!(git_status.file_status(&file_root).unstaged == f::GitStatus::Modified);
        assert!(git_status.file_status(&file_a).unstaged == f::GitStatus::Modified);
        assert!(git_status.file_status(&file_b).unstaged == f::GitStatus::Modified);
    }

    #[cfg(windows)]
    #[test]
    fn reorient_handles_non_unicode_paths_without_panicking() {
        use std::ffi::OsString;
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        let invalid: Vec<u16> = std::env::temp_dir()
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0xD800))
            .collect();
        let os_str = OsString::from_wide(&invalid);
        let path = std::path::PathBuf::from(os_str);
        let _ = super::reorient(&path);
    }

    #[cfg(unix)]
    #[test]
    fn test_reorient_preserves_leaf_symlink() {
        let test_repo = TestGitRepo::new("reorient_symlink");
        let target = test_repo.create_file("target.txt", b"target");
        let symlink = test_repo.create_symlink("target.txt", "link.txt");

        let reoriented = reorient(&symlink);
        assert_eq!(reoriented.file_name().unwrap(), "link.txt");
        assert_ne!(reoriented, reorient(&target));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_modified_status_detected() {
        let test_repo = TestGitRepo::new("symlink_modified");
        let target = test_repo.create_file("target.txt", b"target content");
        let link = test_repo.create_symlink("target.txt", "link.txt");
        test_repo.commit_all("initial commit");

        // Symlink target is changed (pointing to a different file)
        fs::remove_file(&link).unwrap();
        test_repo.create_symlink("target2.txt", "link.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // Symlink status must be Modified
        let link_status = git_status.file_status(&link);
        assert!(link_status.unstaged == f::GitStatus::Modified);
        assert!(link_status.staged == f::GitStatus::NotModified);

        // Target file status must remain NotModified
        let target_status = git_status.file_status(&target);
        assert!(target_status.unstaged == f::GitStatus::NotModified);
        assert!(target_status.staged == f::GitStatus::NotModified);
    }

    #[cfg(unix)]
    #[test]
    fn test_unmodified_symlink_with_modified_target() {
        let test_repo = TestGitRepo::new("unmodified_symlink");
        let target = test_repo.create_file("target.txt", b"original content");
        let link = test_repo.create_symlink("target.txt", "link.txt");
        test_repo.commit_all("initial commit");

        // Target content is modified, but symlink itself is unchanged
        let mut f_target = StdFile::create(&target).unwrap();
        f_target.write_all(b"modified content\n").unwrap();

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // Symlink itself is unmodified
        let link_status = git_status.file_status(&link);
        assert!(link_status.unstaged == f::GitStatus::NotModified);
        assert!(link_status.staged == f::GitStatus::NotModified);

        // Target file is modified
        let target_status = git_status.file_status(&target);
        assert!(target_status.unstaged == f::GitStatus::Modified);
        assert!(target_status.staged == f::GitStatus::NotModified);
    }

    #[cfg(unix)]
    #[test]
    fn test_broken_symlink_git_status() {
        let test_repo = TestGitRepo::new("broken_symlink");
        let broken_link = test_repo.create_symlink("non_existent.txt", "broken.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // Untracked broken symlink should be New
        let status = git_status.file_status(&broken_link);
        assert!(status.unstaged == f::GitStatus::New);

        // Commit it
        test_repo.commit_all("add broken symlink");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);
        let status = git_status.file_status(&broken_link);
        assert!(status.unstaged == f::GitStatus::NotModified);

        // Change broken symlink target to another non-existent target
        fs::remove_file(&broken_link).unwrap();
        test_repo.create_symlink("another_non_existent.txt", "broken.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);
        let status = git_status.file_status(&broken_link);
        assert!(status.unstaged == f::GitStatus::Modified);
    }

    #[cfg(unix)]
    #[test]
    fn test_staged_symlink_operations() {
        let test_repo = TestGitRepo::new("staged_symlink");
        let _target = test_repo.create_file("target.txt", b"target");
        let link = test_repo.create_symlink("target.txt", "link.txt");

        // Staged addition
        test_repo.add_path_to_index("target.txt");
        test_repo.add_path_to_index("link.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        let link_status = git_status.file_status(&link);
        assert!(link_status.staged == f::GitStatus::New);
        assert!(link_status.unstaged == f::GitStatus::NotModified);

        test_repo.commit_all("commit symlink");

        // Staged modification
        fs::remove_file(&link).unwrap();
        test_repo.create_symlink("target_modified.txt", "link.txt");
        test_repo.add_path_to_index("link.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);
        let link_status = git_status.file_status(&link);
        assert!(link_status.staged == f::GitStatus::Modified);
        assert!(link_status.unstaged == f::GitStatus::NotModified);

        test_repo.commit_all("commit modified symlink");

        // Staged deletion
        fs::remove_file(&link).unwrap();
        test_repo.remove_path_from_index("link.txt");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);
        let link_status = git_status.file_status(&link);
        assert!(link_status.staged == f::GitStatus::Deleted);
        assert!(link_status.unstaged == f::GitStatus::NotModified);
    }

    #[test]
    fn test_dotgit_dir_ignored_as_subrepo() {
        let test_repo = TestGitRepo::new("dotgit_subrepo");
        test_repo.create_file("file.txt", b"hello");
        test_repo.commit_all("initial commit");

        // 1. Direct path to .git directory with status=true
        let dotgit_path = test_repo.path.join(".git");
        let res = f::SubdirGitRepo::from_path(&dotgit_path, true);
        assert!(res.status == Some(f::SubdirGitRepoStatus::NoRepo));
        assert_eq!(res.branch, None);

        // 2. Direct path to .git directory with status=false
        let res_no_stat = f::SubdirGitRepo::from_path(&dotgit_path, false);
        assert!(res_no_stat.status.is_none());
        assert_eq!(res_no_stat.branch, None);

        // 3. Relative Path::new(".git")
        let res_rel = f::SubdirGitRepo::from_path(Path::new(".git"), true);
        assert!(res_rel.status == Some(f::SubdirGitRepoStatus::NoRepo));
        assert_eq!(res_rel.branch, None);

        // 4. Legitimate repository path should return actual branch
        let res_repo = f::SubdirGitRepo::from_path(&test_repo.path, true);
        assert!(res_repo.branch.is_some());
        assert!(
            res_repo.branch.as_deref() == Some("master")
                || res_repo.branch.as_deref() == Some("main")
        );
        assert!(res_repo.status == Some(f::SubdirGitRepoStatus::GitClean));
        assert!(!res_repo.is_worktree);
    }

    #[test]
    fn test_worktree_detection() {
        let test_repo = TestGitRepo::new("worktree_detection");
        test_repo.create_file("file.txt", b"hello");
        test_repo.commit_all("initial commit");

        let wt_path = std::env::temp_dir().join(format!(
            "lsr_test_git_worktree_{}_{}",
            "unit",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&wt_path);

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let _ = repo.worktree("wt_branch", &wt_path, None).unwrap();

        let res_wt = f::SubdirGitRepo::from_path(&wt_path, true);
        assert!(res_wt.is_worktree);
        assert_eq!(res_wt.branch.as_deref(), Some("wt_branch"));
        assert_eq!(res_wt.status, Some(f::SubdirGitRepoStatus::GitClean));

        let res_main = f::SubdirGitRepo::from_path(&test_repo.path, true);
        assert!(!res_main.is_worktree);

        let _ = fs::remove_dir_all(&wt_path);
    }

    #[test]
    fn test_child_status_scoped_to_parent_dir() {
        let test_repo = TestGitRepo::new("child_status_scoped");
        test_repo.create_file(".gitignore", b"target/\n*.log\n");
        let target_bin = test_repo.create_file("target/debug/app", b"binary content");
        let src_rs = test_repo.create_file("src/main.rs", b"fn main() {}");
        let src_log = test_repo.create_file("src/output.log", b"log content");
        test_repo.commit_all("add gitignore");

        let repo = git2::Repository::open(&test_repo.path).unwrap();
        let target_path = test_repo.path.join("target");
        let target_debug_path = test_repo.path.join("target/debug");
        let src_path = test_repo.path.join("src");

        let roots = vec![reorient(&test_repo.path)];
        let git_status = repo_to_statuses(&repo, &test_repo.path, &roots, true);

        // 1. Direct file_status on target directory when queried from root: Ignored
        assert!(git_status.file_status(&target_path).unstaged == f::GitStatus::Ignored);

        // 2. Child status when querying child of target: target/ rule is excluded
        let child_stat = git_status.child_status(&target_path, &target_bin, false);
        assert!(child_stat.unstaged != f::GitStatus::Ignored);

        // 3. Child status when querying nested debug directory: target/ rule is excluded
        let debug_stat = git_status.child_status(&target_path, &target_debug_path, false);
        assert!(debug_stat.unstaged != f::GitStatus::Ignored);

        // 4. In src directory (which is not ignored), *.log rule ignores src/output.log:
        let src_rs_stat = git_status.child_status(&src_path, &src_rs, false);
        assert!(src_rs_stat.unstaged != f::GitStatus::Ignored);
        let src_log_stat = git_status.child_status(&src_path, &src_log, false);
        assert!(src_log_stat.unstaged == f::GitStatus::Ignored);
    }
}
