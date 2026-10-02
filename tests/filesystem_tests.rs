// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "filesystem/blocks_column.rs"]
mod blocks_column;
#[path = "filesystem/broken_symlinks.rs"]
mod broken_symlinks;
#[path = "filesystem/cachedir.rs"]
mod cachedir;
#[path = "filesystem/ignore_globs.rs"]
mod ignore_globs;
#[path = "filesystem/inspect_archives.rs"]
mod inspect_archives;
#[path = "filesystem/no_symlink_targets.rs"]
mod no_symlink_targets;
#[path = "filesystem/only_files_wildcards.rs"]
mod only_files_wildcards;
#[path = "filesystem/recsize_hardlinks.rs"]
mod recsize_hardlinks;
#[path = "filesystem/show_dotfiles.rs"]
mod show_dotfiles;
#[path = "filesystem/since_duration.rs"]
mod since_duration;
#[cfg(unix)]
#[path = "filesystem/symlink_absolute.rs"]
mod symlink_absolute;
#[cfg(unix)]
#[path = "filesystem/symlink_broken_targets.rs"]
mod symlink_broken_targets;
#[cfg(unix)]
#[path = "filesystem/symlink_chains.rs"]
mod symlink_chains;
#[cfg(unix)]
#[path = "filesystem/symlink_classify.rs"]
mod symlink_classify;
#[cfg(unix)]
#[path = "filesystem/symlink_dereference.rs"]
mod symlink_dereference;
#[cfg(unix)]
#[path = "filesystem/symlink_target_style.rs"]
mod symlink_target_style;
#[path = "filesystem/warn_hidden.rs"]
mod warn_hidden;
#[path = "filesystem/windows_visibility.rs"]
mod windows_visibility;
