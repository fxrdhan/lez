<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# AGENTS.md — Agent & Developer Guide for `lez`

> **`lez`** is a fast, modern, and feature-rich replacement for `ls` written in Rust (Rust 2024 edition, MSRV 1.90+).
> Lineage: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).

---

## 1. System Overview & Architecture

`lez` reads directories and files from the filesystem, extracts rich metadata (permissions, ownership, sizes, Git status, extended attributes, mounts, timestamps), formats them with syntax highlighting, Nerd Font icons, and renders them in various display modes (Grid, Long Details, Grid Details, One-line, Tree, Lines of Code Summary via `--code`, and JSON via `--json`).

Mode selection priority (`Mode::deduce` in `src/options/view.rs`): `--code` → `--json` → strict-mode checks → TTY default (Grid on TTY, Lines otherwise) → `--long` (+ `--grid` = GridDetails) → `--tree` → `--oneline`. Only `--binary`/`--bytes` use last-argument-wins semantics (clap `overrides_with`).

```
CLI Input (Args & Env)
        │
        ▼
[src/options/] ────► Deduce Options (Clap parser, Theme YAML, LS_COLORS, Env)
        │
        ▼
[src/fs/] ─────────► Scan Files & Dirs (Rayon, File/Dir metadata cache, GitCache)
        │
        ▼
[src/fs/filter.rs] ► Filter & Sort (Hidden files, GitIgnore, Glob, SortField)
        │
        ▼
[src/output/] ─────► Format & Render (Grid, Table Columns, Icons, ANSI Styles, Tree)
        │
        ▼
   Stdout / TTY
```

---

## 2. Directory & Module Breakdown

### Root Directory
- [`Cargo.toml`](Cargo.toml): Package config, dependencies, feature flags (`git`, `vendored-openssl`, `vendored-libgit2`, `powertest`, `nix`), and release profile (LTO, strip, opt-level 3).
- [`build.rs`](build.rs): Generates `version_string.txt` during compilation (git commit hash, date, features) consumed by clap in `src/options/parser.rs`.
- [`justfile`](justfile): Command runner recipes for building, testing, linting, packaging, and man page generation.
- [`flake.nix`](flake.nix): Nix flake definition for reproducible dev environment and CI builds.
- [`man/`](man/): Pandoc markdown sources for man pages (`lez.1.md`, `lez_colors.5.md`, `lez_colors-explanation.5.md`).
- [`completions/`](completions/): Shell completion scripts for `bash`, `zsh`, `fish`, `nushell`, and `powershell`.
- [`docs/`](docs/): Documentation, theme schema, and upstream triage reference ([`docs/UPSTREAM_TRIAGE.md`](docs/UPSTREAM_TRIAGE.md)).
- [`tests/`](tests/): Integration tests, trycmd CLI snapshots, LOC tests, and powertests.

### `src/` Architecture

- **Entry & Orchestration** ([`src/main.rs`](src/main.rs), [`src/lib.rs`](src/lib.rs), [`src/logger.rs`](src/logger.rs)): Argument parsing, logging configuration (`LEZ_DEBUG`), signal handling, file traversal, and execution dispatch.
  - Exit codes: `0` (Success), `1` (Runtime/IO error), `2` (Missing input path), `3` (Options error), `13` (Permission denied).
- **CLI & Configuration** ([`src/options/`](src/options/)):
  - [`parser.rs`](src/options/parser.rs): Clap CLI specification, flags, options, headings, value parsers, defaults.
  - [`mod.rs`](src/options/mod.rs), [`view.rs`](src/options/view.rs), [`filter.rs`](src/options/filter.rs): Option deduction and validation.
  - [`theme.rs`](src/options/theme.rs), [`config.rs`](src/options/config.rs), [`vars.rs`](src/options/vars.rs): Theme YAML parsing (`$LEZ_CONFIG_DIR`) and environment variables (`LS_COLORS`, `LEZ_COLORS`, etc.).
  - [`stdin.rs`](src/options/stdin.rs): Handles `--stdin` filename input.
- **Filesystem & Metadata Layer** ([`src/fs/`](src/fs/)):
  - [`file.rs`](src/fs/file.rs): Fundamental `File` struct with `OnceLock` lazy caching for metadata, xattr, mounts, security context, and recursive size.
  - [`dir.rs`](src/fs/dir.rs), [`dir_action.rs`](src/fs/dir_action.rs): Directory traversal, recursion (`--recurse`, `--tree`, `--level`), and dotfile/gitignore filtering.
  - [`filter.rs`](src/fs/filter.rs), [`fields.rs`](src/fs/fields.rs): Sorting (`SortField`, natural casing via `natord-plus-plus`) and domain file attributes.
  - [`feature/git.rs`](src/fs/feature/git.rs), [`feature/xattr.rs`](src/fs/feature/xattr.rs), [`mounts/`](src/fs/mounts/): Git repository status caching (`git2`), extended attributes, and mount points.
- **Output & View Rendering** ([`src/output/`](src/output/)):
  - [`grid.rs`](src/output/grid.rs), [`lines.rs`](src/output/lines.rs), [`details.rs`](src/output/details.rs), [`grid_details.rs`](src/output/grid_details.rs), [`tree.rs`](src/output/tree.rs): Display mode renderers.
  - [`json.rs`](src/output/json.rs): Structured JSON output sharing long table column definitions.
  - [`summary.rs`](src/output/summary.rs): Total counts and summary footer (`--summary`).
  - [`file_name.rs`](src/output/file_name.rs), [`icons.rs`](src/output/icons.rs): Filename formatting, Nerd Font icons (`phf_map`), symlink targets, classification symbols, OSC 8 hyperlinks.
  - [`render/`](src/output/render/): Specialized column renderers (permissions, users, groups, size, timestamps, git status, inode, links, flags, security context, loc).
- **Lines of Code (LOC) Engine** ([`src/loc/`](src/loc/), [`src/output/code.rs`](src/output/code.rs)): Fast parallelized source code line counter supporting 100+ languages (classifies code, comments, blanks).
- **Theming & Styling** ([`src/theme/`](src/theme/)): ANSI style definitions, `LS_COLORS` parser (`lsc.rs`), and default color palette.

---

## 3. Development Workflow & Commands

### Prerequisites
- **Rust**: 1.90+ (Rust 2024 edition)
- **C Compiler & libgit2**: Required for `git2` crate (or build with `--no-default-features` to omit Git).
- **Pandoc** (optional): Required to compile man pages.
- **Just** (optional): Command runner.

### Essential Commands
| Action | Cargo Command | Just Recipe |
|---|---|---|
| Check compilation | `cargo check` | `just check` |
| Build (debug) | `cargo build` | `just build` |
| Build (release) | `cargo build --release` | `just build-release` |
| Run unit tests | `cargo test --lib` | `just test` |
| Run all tests | `cargo nextest run` | `just test` |
| Run integration/CLI tests | `cargo nextest run --test cli_tests` | `just test` |
| Lint codebase | `cargo clippy` | `just clippy` |
| Format code | `cargo fmt` | `nix fmt` |
| Build man pages | `pandoc ...` | `just man` |

### Test Layers
| Layer | Location | How to run |
|---|---|---|
| Unit tests (inline `#[cfg(test)]`) | throughout `src/` | `cargo test --lib` |
| Rust integration tests | `tests/*.rs` | `cargo nextest run` |
| trycmd CLI snapshots | `tests/cmd/*.toml` + fixtures in `tests/itest/`, `tests/itest-loc/` | `cargo nextest run --test cli_tests` |
| Generated snapshots (nix-gated) | `tests/gen/` | nix build (`just itest`) |
| Powertest corpus (feature-gated) | `tests/ptests/` | built via powertest tool (`just regen`) |

Snapshot regeneration: `just idump` (refresh `.stdout`/`.stderr` dumps) and `just regen` (regenerate powertest cases from `powertest.yaml`). `just regen` is idempotent.

### Running the suite

Use `cargo nextest run`, not `cargo test`. `cargo test` runs one test binary at a time and parallelises only within each; this suite is 91 binaries with a median of five tests apiece, so most of a run leaves the machine idle. On a ten-core machine the same 2,168 tests take 256 s under `cargo test` and 49 s under nextest. Configuration lives in `.config/nextest.toml`; CI uses its `ci` profile, which retries a failure once.

Install it with `cargo install cargo-nextest --locked --version 0.9.128`. Pin that version: 0.9.129 and later require rustc 1.91, above this crate's MSRV of 1.90.

nextest does not run doc tests. `cargo test --doc` covers them, and CI runs it as a separate step.

**On macOS, exempt your terminal from Gatekeeper first.** Every freshly linked binary is assessed on its first execution, and this suite relinks 91 of them per source change and spawns the binary from 438 call sites. Without the exemption the run takes 21 minutes with the machine 86% idle, waiting on `syspolicyd` and `XprotectService`; with it, 256 s. Add your terminal under System Settings, Privacy & Security, Developer Tools. Gatekeeper stays active everywhere else.

---

## 4. Agent Guidelines & Rules for Contributing

### 1. Adding a New CLI Flag or Option
1. **Define argument** in [`src/options/parser.rs`](src/options/parser.rs) using `clap`.
2. **Handle option deduction** in [`src/options/mod.rs`](src/options/mod.rs) or the appropriate sub-options module (`view.rs`, `filter.rs`; recursion lives in `src/fs/dir_action.rs`).
3. **Propagate into runtime** in [`src/main.rs`](src/main.rs) and the corresponding `src/output/` or `src/fs/` renderer.
4. **Update documentation**: [README.md](README.md) and [`man/lez.1.md`](man/lez.1.md).
5. **Update Shell Completions**:
   - Update primary completions: `completions/{bash/lez, zsh/_lez, fish/lez.fish, nush/lez.nu, pwsh/_lez.ps1}`.
   - Regenerate `eza` compatibility copies with `sed 's/lez/eza/g'` (checked by `tests/completion_equals_tests.rs`).
   - If a flag uses `require_equals`, ensure all 5 shell backends complete values after `=` rather than a space.
6. **Add Tests** in [`tests/`](tests/).

### 2. Adding or Modifying File Icons & LOC Languages
- **Icons**: Edit [`src/output/icons.rs`](src/output/icons.rs) and register in `FILENAME_ICONS` or `EXTENSION_ICONS`.
- **Languages**: Edit [`src/loc/mod.rs`](src/loc/mod.rs) to define language comment syntax, register extension/filename, and add fixtures in `tests/itest-loc/`.

### 3. Cross-Platform Safety
- Guard platform-specific code with `#[cfg(unix)]`, `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "linux")]`, or `#[cfg(windows)]`.
- Always verify Windows compatibility (avoid raw Unix syscalls without fallbacks).
- Use `Path` / `PathBuf` methods rather than hardcoded `/` or `\` separators.

### 4. Code Quality & Commits
- Follow [Conventional Commits](https://www.conventionalcommits.org/) (e.g. `feat: ...`, `fix: ...`, `docs: ...`, `chore: ...`).
- **Upstream references live in PR body only**: Commit messages must NOT tag upstream (`eza#NNNN` or `eza-community/eza#NNNN`) — describe the change itself. Use full markdown links `[eza-community/eza#NNNN](https://github.com/eza-community/eza/pull/NNNN)` in the PR body.
- **Granular Atomic Commits (1 Upstream Task = 1 Commit)**: Never bundle multiple independent upstream PRs or features into a single monolithic commit. Each port/feature must have its own separate, atomic commit with unit tests.
- **No attribution trailers**: Do not add `Co-authored-by` trailers in commit messages.
- Ensure all licenses comply with REUSE / SPDX guidelines (`EUPL-1.2` or `MIT`).
- Run `cargo clippy` and `cargo test --lib` before committing.

---

## 5. Upstream Policy & Triage Summary

`lez` ports unmerged `eza-community/eza` work by hand rather than maintaining a git tracking branch.

### Key Guidelines
1. **Audit & Verification**: Before implementing any upstream report or feature request, reproduce against our binary first. Many upstream issues are already solved here or caused by external factors.
2. **Platform Diagnostics**: Linux-only reports can be reproduced quickly on macOS using Docker (`rust:*-bookworm`). Windows-specific behaviors are tested via `.github/workflows/windows-probe.yml`.
3. **Reference Log**: For the complete audit tables, closed/declined PRs & issues list, and pending architecture items, refer to [`docs/UPSTREAM_TRIAGE.md`](docs/UPSTREAM_TRIAGE.md).
