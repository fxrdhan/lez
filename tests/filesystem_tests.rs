// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "filesystem/blocks_flag.rs"]
mod blocks_flag;
#[path = "filesystem/broken_symlinks.rs"]
mod broken_symlinks;
#[path = "filesystem/cachedir_ignore.rs"]
mod cachedir_ignore;
#[path = "filesystem/ignore_glob_path.rs"]
mod ignore_glob_path;
#[path = "filesystem/inspect_archives.rs"]
mod inspect_archives;
#[path = "filesystem/no_symlink_targets.rs"]
mod no_symlink_targets;
#[path = "filesystem/only_files_wildcard.rs"]
mod only_files_wildcard;
#[path = "filesystem/recsize_hardlink_stress.rs"]
mod recsize_hardlink_stress;
#[path = "filesystem/show_dotfiles.rs"]
mod show_dotfiles;
#[path = "filesystem/since_flag.rs"]
mod since_flag;
#[path = "filesystem/total_size_traversal.rs"]
mod total_size_traversal;
#[path = "filesystem/warn_hidden.rs"]
mod warn_hidden;
