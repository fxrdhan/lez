// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "git/gitignore_explicit_target.rs"]
mod gitignore_explicit_target;
#[path = "git/glyphs.rs"]
mod glyphs;
#[path = "git/repos_dotgit.rs"]
mod repos_dotgit;
#[path = "git/submodule_ignore.rs"]
mod submodule_ignore;
#[path = "git/symlink_status.rs"]
mod symlink_status;
#[path = "git/untracked_scan.rs"]
mod untracked_scan;
#[path = "git/worktree.rs"]
mod worktree;
