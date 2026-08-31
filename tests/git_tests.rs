// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

#![allow(unused_imports, dead_code)]

mod common;

#[path = "git/gitignore.rs"]
mod gitignore;
#[path = "git/glyphs.rs"]
mod glyphs;
#[path = "git/repos_dotgit.rs"]
mod repos_dotgit;
#[path = "git/submodules.rs"]
mod submodules;
#[path = "git/symlinks.rs"]
mod symlinks;
#[path = "git/untracked.rs"]
mod untracked;
#[path = "git/worktree.rs"]
mod worktree;
