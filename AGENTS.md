<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# AGENTS.md — Agent & Developer Guide for `lez`

> **`lez`** is a fast, modern, and feature-rich replacement for `ls` written in Rust (Rust 2024 edition, MSRV 1.90+).
> **Lineage**: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).

---

## 1. System Overview & Architecture

### Core Pipeline
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

### Mode Deduction Priority
Evaluated in `Mode::deduce` ([`src/options/view.rs`](src/options/view.rs)):
`--code` ➔ `--json` ➔ Strict-mode checks ➔ TTY default (Grid on TTY, Lines otherwise) ➔ `--long` (`+ --grid` = GridDetails) ➔ `--tree` ➔ `--oneline`.
*(Note: Only `--binary`/`--bytes` use last-argument-wins semantics via clap `overrides_with`).*

---

## 2. Directory & Module Map

### Root & Infrastructure
| Path | Purpose |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Package manifest, profiles (LTO, strip, opt-level 3), features (`git`, `inspect-archives`, `vendored-*`). |
| [`build.rs`](build.rs) | Generates `version_string.txt` (commit hash, date, features) for clap. |
| [`justfile`](justfile) | Task runner recipes for build, test, lint, package, and docs. |
| [`flake.nix`](flake.nix) | Nix dev environment and CI build definitions. |
| [`man/`](man/) | Pandoc markdown sources for man pages (`lez.1.md`, `lez_colors.5.md`, `lez_colors-explanation.5.md`). |
| [`completions/`](completions/) | Shell completions (`bash`, `zsh`, `fish`, `nushell`, `powershell`). |
| [`docs/`](docs/) | Documentation and upstream triage log ([`docs/UPSTREAM_TRIAGE.md`](docs/UPSTREAM_TRIAGE.md)). |
| [`tests/`](tests/) | Unit tests, integration tests (`tests/*.rs`), trycmd snapshots (`tests/cmd/`), and powertests (`tests/ptests/`). |

### `src/` Subsystems
| Subsystem | Key Modules | Responsibility |
|---|---|---|
| **Entry & Orchestration** | [`src/main.rs`](src/main.rs), [`src/lib.rs`](src/lib.rs), [`src/logger.rs`](src/logger.rs) | CLI execution, logging (`LEZ_DEBUG`), signal handling, exit codes. |
| **Options & Config** | [`src/options/`](src/options/) (`parser.rs`, `vars.rs`, `theme.rs`, `config.rs`, `filter.rs`, `view.rs`, `error.rs`) | Clap parsing, env vars, config directory (`$LEZ_CONFIG_DIR`), theme YAML, option error formatting. |
| **Filesystem & Cache** | [`src/fs/`](src/fs/) (`file.rs`, `dir.rs`, `dir_action.rs`, `filter.rs`, `fields.rs`, `archives.rs`) | Metadata caching with `OnceLock`, traversal, sorting (`natord-plus-plus`), filtering, `.tar` inspection. |
| **Features & OS** | [`src/fs/feature/`](src/fs/feature/) (`git.rs`, `xattr.rs`), [`src/fs/mounts/`](src/fs/mounts/) | `git2` status caching, extended attributes, Linux/macOS mount points, Linux `capctl` capabilities. |
| **Rendering & Output** | [`src/output/`](src/output/) (`grid.rs`, `details.rs`, `lines.rs`, `tree.rs`, `json.rs`, `icons.rs`, `summary.rs`, `render/`) | Renderers, Nerd Font icons (`phf_map`), symlink targets, hyperlinks (OSC 8), column formatters. |
| **LOC Engine** | [`src/loc/`](src/loc/), [`src/output/code.rs`](src/output/code.rs) | Parallelized source code line counter for 100+ languages (comments, blanks, code). |
| **Theme & Color** | [`src/theme/`](src/theme/) (`lsc.rs`, `mod.rs`) | ANSI styles, `LS_COLORS`/`LEZ_COLORS` parsing, color palette. |

---

## 3. Development & Testing Workflow

### Essential Commands
| Action | Cargo Command | Just Recipe |
|---|---|---|
| Check compilation | `cargo check` | `just check` |
| Build (debug / release) | `cargo build` / `cargo build --release` | `just build` / `just build-release` |
| Run full test suite | `cargo nextest run --workspace` + `cargo test --doc` | `just test` |
| Run CLI snapshot tests | `cargo nextest run --test cli_tests` | `just test` |
| Run benchmarks | `cargo bench` | — |
| Lint & Format | `cargo clippy` & `cargo fmt` | `just clippy` & `nix fmt` |
| Build man pages | `pandoc ...` | `just man` |
| Dump / Regen Snapshots | — | `just idump` / `just regen` |

### Test Suite Rules & Performance
- **Run with Nextest**: Always use `cargo nextest run` instead of `cargo test` (parallelizes across 91 binaries; 49s vs 256s).
- **Pinned Version**: Use `cargo-nextest` version `0.9.128` (versions 0.9.129+ require rustc 1.91, above MSRV 1.90).
- **Doc Tests**: Nextest does not run doc tests; run `cargo test --doc` separately.
- **macOS Gatekeeper Exemption**: Exempt your terminal under *System Settings ➔ Privacy & Security ➔ Developer Tools* to avoid `syspolicyd` stalling the 91 relinked test binaries (reduces runtime from 21m to ~4m).

### Test Layers
| Layer | Location | Purpose & Execution |
|---|---|---|
| **Unit tests** | `src/**` (`#[cfg(test)]`) | Module-level unit tests (`cargo test --lib`). |
| **Integration tests** | `tests/*.rs` | Full feature behavior & CLI flags (`cargo nextest run`). |
| **CLI snapshots** | `tests/cmd/*.toml` + `tests/itest/` | Snapshot output assertions (`trycmd`). Dumps refreshed via `just idump`. |
| **LOC tests** | `tests/itest-loc/` | Verification for language line-counting fixtures. |
| **Powertest corpus** | `tests/ptests/` + `powertest.yaml` | Permutation tests generated by `powertest` tool (`just regen`). |

---

## 4. Extension & Contribution Playbooks

### 1. Adding a New CLI Flag or Option
1. **Define Argument** in [`src/options/parser.rs`](src/options/parser.rs) using `clap`.
2. **Handle Option Deduction** in [`src/options/mod.rs`](src/options/mod.rs) or sub-options (`view.rs`, `filter.rs`, `dir_action.rs`).
3. **Propagate into Runtime** in [`src/main.rs`](src/main.rs) and the corresponding [`src/fs/`](src/fs/) or [`src/output/`](src/output/) renderer.
4. **Update Documentation**: [README.md](README.md) and [`man/lez.1.md`](man/lez.1.md).
5. **Update Shell Completions**:
   - Update `completions/{bash/lez, zsh/_lez, fish/lez.fish, nush/lez.nu, pwsh/_lez.ps1}`.
   - Sync `eza` mirror files with `sed 's/lez/eza/g'` (validated by [`tests/completion_equals_tests.rs`](tests/completion_equals_tests.rs)).
   - If flag uses `require_equals`, ensure all 5 shells complete after `=` rather than space.
6. **Powertest & Tests**:
   - If flag uses `require_equals`, add to [`powertest.yaml`](powertest.yaml) as a key without `values:` (e.g. `--color=always`).
   - Add integration tests in [`tests/`](tests/).

### 2. Adding an Environment Variable
1. **Declare Static Variable** in [`src/options/vars.rs`](src/options/vars.rs).
2. **Handle Fallback Precedence**: Follow `LEZ_*` ➔ `EZA_*` ➔ `EXA_*` ➔ generic/POSIX (`LS_COLORS`, `TIME_STYLE`, `NO_COLOR`).
3. **Update `MockVars` (Mandatory)**: Add the variable field to struct `MockVars` and update `get`/`set` match arms in [`src/options/vars.rs`](src/options/vars.rs). *Failure to do so triggers a panic in `setting_an_unknown_variable_is_refused`.*

### 3. Adding File Icons & LOC Languages
- **Icons**: Edit [`src/output/icons.rs`](src/output/icons.rs) and register in `FILENAME_ICONS` or `EXTENSION_ICONS`.
- **LOC Languages**: Edit [`src/loc/mod.rs`](src/loc/mod.rs) to define comment syntax and register extensions/filenames; add test fixture in `tests/itest-loc/`.

---

## 5. Engineering Standards & Conventions

### Performance & Memory Rules
- **Lazy Evaluation via `OnceLock`**: Never eagerly query metadata on [`File`](src/fs/file.rs). Extended attributes, Git status, security context, mounts, and recursive sizes must stay lazily evaluated via `OnceLock` only when the active view/columns request them.
- **Stat Amplification Prevention**: Avoid probing filesystem metadata unnecessarily on directory traversal (e.g. respect `LEZ_NO_EMPTY_DIR_ICON` to prevent roundtrips on FUSE/network shares).
- **Parallelism**: Use Rayon `par_iter()` for expensive batch operations (directory scanning and LOC counting).
- **Memoization**: Cache repetitive queries (e.g. `Dir::contains` set memoization).

### Exit Codes & Error Handling
| Exit Code | Meaning | Context |
|---|---|---|
| `0` | Success | Normal execution. |
| `1` | Runtime / I/O Error | Non-permission filesystem or I/O failure. |
| `2` | Missing Input Path | Specified input path does not exist. |
| `3` | Options Error | Conflicting CLI flags or invalid argument combinations ([`OptionsError`](src/options/error.rs)). |
| `13` | Permission Denied | `EACCES` when accessing target paths. |

*Rule: Never panic or unwrap on user inputs or runtime I/O. Wrap options errors in `OptionsError` ([`src/options/error.rs`](src/options/error.rs)) and propagate gracefully.*

### Cross-Platform Matrix & Features
- **Feature Flags**:
  - `git` (default): `git2` integration for repo status.
  - `inspect-archives` (default): `tar` integration for archive listing.
  - `vendored-openssl` / `vendored-libgit2`: Static build helpers.
  - Build without git/tar via `--no-default-features`.
- **Platform Guards**:
  - Linux: `capctl` (capabilities decoding), `proc-mounts`.
  - macOS: `uzers`, macOS xattrs (`com.apple.*`), BSD file flags.
  - Windows: `windows-sys` (console, file attributes).
  - Use `Path` / `PathBuf` methods rather than hardcoded `/` or `\` separators.

### Commit & PR Standards
- **Conventional Commits**: Format titles following conventional commit specifications (e.g. `feat: ...`, `fix: ...`, `docs: ...`, `chore: ...`).
- **Granular Atomic Commits**: Never bundle multiple independent tasks or changes into a single monolithic commit. Each feature or fix must have its own separate, atomic commit with tests.
- **Clean PR History & CI Failures**: Do not pollute commit history with "fix CI" or typo commits. Fix locally, amend the commit (`git commit --amend`), and update via `git push --force-with-lease` to keep PR history clean and green.
- **Strict PR Template**: All PRs must strictly adhere to [`.github/PULL_REQUEST_TEMPLATE/pull_request_template.md`](.github/PULL_REQUEST_TEMPLATE/pull_request_template.md).

---

## 6. Upstream Policy & Triage Summary

`lez` ports unmerged `eza-community/eza` work selectively by hand rather than maintaining a git tracking branch.
1. **Audit & Verification**: Before implementing any upstream report or feature request, reproduce against our binary first. Many upstream issues are already solved here or caused by external factors.
2. **Platform Diagnostics**:
   - Linux-only reports can be reproduced quickly on macOS using Docker (`rust:*-bookworm`).
   - Windows-specific behaviors are tested via `.github/workflows/windows-probe.yml`.
3. **Reference Log**: For the complete audit tables, closed/declined PRs & issues list, and pending architecture items, refer to [`docs/UPSTREAM_TRIAGE.md`](docs/UPSTREAM_TRIAGE.md).
