<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# AGENTS.md — Agent & Developer Guide for `lsr`

> **`lsr`** is a fast, modern, and feature-rich replacement for `ls` written in Rust (Rust 2024 edition, MSRV 1.90+).
> Lineage: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lsr` (by fxrdhan).

---

## 1. System Overview & Architecture

`lsr` reads directories and files from the filesystem, extracts rich metadata (permissions, ownership, sizes, Git status, extended attributes, mounts, timestamps), formats them with syntax highlighting, Nerd Font icons, and renders them in various display modes (Grid, Long Details, Grid Details, One-line, Tree, and Lines of Code Summary).

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
- [`Cargo.toml`](Cargo.toml): Package configuration (`lsr` v0.24+), dependencies (`clap`, `rayon`, `nu-ansi-term`, `git2`, `phf`, `serde_norway`, etc.), feature flags (`git`, `vendored-openssl`, `vendored-libgit2`, `powertest`, `nix`), and release profile (LTO, strip, opt-level 3).
- [`build.rs`](build.rs): Generates `version_string.txt` during compilation (reads git commit hash, date, features) consumed by `clap` in `src/options/parser.rs`.
- [`justfile`](justfile): Command runner recipes for building, testing, linting, packaging, and man page generation.
- [`flake.nix`](flake.nix): Nix flake definition for reproducible developer environment and CI builds.
- [`man/`](man/): Pandoc markdown sources for man pages (`eza.1.md`, `eza_colors.5.md`, `eza_colors-explanation.5.md`).
- [`completions/`](completions/): Shell completion scripts for `bash`, `zsh`, `fish`, `nushell`, and `powershell`.
- [`tests/`](tests/): Integration tests, snapshot tests powered by `trycmd`, LOC tests, and powertests.

---

### `src/` Architecture

#### Entry Point & Orchestration
- [`src/main.rs`](src/main.rs):
  - Configures logging (`logger::configure`) based on `EZA_DEBUG` / `EXA_DEBUG`.
  - Sets up signal handlers (e.g. `SIGPIPE` default handling on Unix).
  - Parses arguments via `options::parser::get_command().get_matches()`.
  - Initializes `Exa` struct holding `Options`, `Theme`, terminal width, and `GitCache`.
  - Runs `Exa::run()`:
    - If `--code` mode: counts lines of code and renders language breakdown via `src/output/code.rs`.
    - Otherwise: wraps input paths into `File` / `Dir` structs, verifies metadata, sorts/filters, recurses if requested (`--recurse`, `--tree`), and delegates rendering to `print_files()` and `print_dirs()`.
  - Maps standard exit codes:
    - `0` (`SUCCESS`): Successful execution.
    - `1` (`RUNTIME_ERROR`): I/O or filesystem error encountered.
    - `3` (`OPTIONS_ERROR`): Invalid or conflicting CLI options.
    - `13` (`PERMISSION_DENIED`): Skipped directories due to OS permission denial.
- [`src/lib.rs`](src/lib.rs): Re-exports modules for internal library tests and benchmarks.
- [`src/logger.rs`](src/logger.rs): Debug logging implementation.

#### CLI & Configuration (`src/options/`)
- [`src/options/parser.rs`](src/options/parser.rs): Defines the `clap::Command` CLI specification with all flags, options, headings, value parsers, and defaults.
- [`src/options/mod.rs`](src/options/mod.rs): Combines sub-options into `Options`. Implements backward precedence heuristics (later flags override earlier flags from shell aliases).
- [`src/options/dir_action.rs`](src/options/dir_action.rs): Handles recursion settings (`--recurse`, `--tree`, `--level`, `--treat-dirs-as-files`).
- [`src/options/filter.rs`](src/options/filter.rs): Handles filtering flags (`-a`/`--all`, `-D`/`--only-dirs`, `-f`/`--only-files`, `--git-ignore`, `-I`/`--ignore-glob`, `--no-symlinks`, `--show-symlinks`, `--sort`, `--reverse`).
- [`src/options/view.rs`](src/options/view.rs): Handles view mode selection (`Grid`, `Details`, `GridDetails`, `Lines`, `Code`), column toggles, `--absolute`, `--dereference`, `--hyperlink`, `--icons`.
- [`src/options/theme.rs`](src/options/theme.rs) & [`src/options/config.rs`](src/options/config.rs): Parses `theme.yml` configuration (from `$LSR_CONFIG_DIR`, `$EZA_CONFIG_DIR`, or `$XDG_CONFIG_HOME/lsr|eza`) and color variables.
- [`src/options/vars.rs`](src/options/vars.rs): Declares environment variables (`LS_COLORS`, `EZA_COLORS`, `EXA_COLORS`, `NO_COLOR`, `COLUMNS`, `TIME_STYLE`, `EZA_STRICT`, `EZA_DEBUG`, `EZA_GRID_ROWS`, `EZA_ICON_SPACING`, `EZA_ICONS_AUTO`, `EZA_STDIN_SEPARATOR`, etc.) and the mockable `Vars` trait.
- [`src/options/stdin.rs`](src/options/stdin.rs): Handles `--stdin` filename input with configurable delimiter.
- [`src/options/error.rs`](src/options/error.rs): Error formatting for option validation.

#### Filesystem & Metadata Layer (`src/fs/`)
- [`src/fs/file.rs`](src/fs/file.rs): The fundamental `File` struct. Uses `OnceLock` for lazy loading and caching of:
  - File metadata (`stat`).
  - File type (regular, directory, symlink, socket, pipe, char/block device).
  - Extended attributes (`xattr`).
  - Mount points (`mounts`).
  - Recursive directory size (`recursive_size`).
  - Security context (`SecurityContextType`).
- [`src/fs/dir.rs`](src/fs/dir.rs): `Dir` struct and `Files` iterator for reading directory entries with dotfile/gitignore filtering.
- [`src/fs/filter.rs`](src/fs/filter.rs): Core sorting and filtering algorithms (`SortField`, natural casing sort via `natord-plus-plus`, directory-first/last ordering).
- [`src/fs/fields.rs`](src/fs/fields.rs): Domain types for file attributes (Permissions, Inode, Links, GitStatus, FileSize, Blocks, etc.).
- [`src/fs/feature/git.rs`](src/fs/feature/git.rs): Git integration backed by `git2` (libgit2). Caches repo statuses (`Modified`, `New`, `Deleted`, `Renamed`, `Ignored`, `Typechange`) and branch heads.
- [`src/fs/feature/xattr.rs`](src/fs/feature/xattr.rs): Extended attributes extraction for macOS, Linux, and BSD.
- [`src/fs/mounts/`](src/fs/mounts/): Mount point resolution on Linux (`proc-mounts`) and macOS (`getmntinfo`).
- [`src/fs/recursive_size.rs`](src/fs/recursive_size.rs): Computes directory sizes recursively when `--total-size` is passed.

#### Output & View Rendering (`src/output/`)
- [`src/output/mod.rs`](src/output/mod.rs): Defines `View`, `Mode`, and `TerminalWidth`.
- [`src/output/grid.rs`](src/output/grid.rs): Multi-column grid rendering formatted with `uutils_term_grid`.
- [`src/output/lines.rs`](src/output/lines.rs): Single-column / one-per-line output (`-1`).
- [`src/output/details.rs`](src/output/details.rs): Long table view (`-l`), calculates column widths, prints optional headers (`-h`).
- [`src/output/grid_details.rs`](src/output/grid_details.rs): Hybrid grid + long details view.
- [`src/output/tree.rs`](src/output/tree.rs): Recursive tree structure drawing with edge connectors (`├──`, `└──`, `│`).
- [`src/output/table.rs`](src/output/table.rs): Table row and cell layout generator.
- [`src/output/file_name.rs`](src/output/file_name.rs): Formats filename with extension styling, icons, symlink arrow target (`=>`), classification symbols (`*`, `/`, `@`, etc.), and OSC 8 terminal hyperlinks.
- [`src/output/icons.rs`](src/output/icons.rs): Fast static mapping of file names, extensions, and directory names to Nerd Font Unicode icons using `phf_map`.
- [`src/output/cell.rs`](src/output/cell.rs) & [`src/output/escape.rs`](src/output/escape.rs): ANSI cell width calculations and string escaping/quoting.
- [`src/output/color_scale.rs`](src/output/color_scale.rs): Luminance gradient calculations for size and date color scaling (`--color-scale`).
- [`src/output/render/`](src/output/render/): Specialized column renderers:
  - `permissions_unix.rs`, `permissions_windows.rs`, `octal.rs`: File permissions.
  - `users.rs`, `groups.rs`: User and group IDs / names.
  - `size.rs`, `blocks.rs`: File size with binary/decimal units or raw bytes.
  - `times.rs`: Timestamps (modified, created, accessed, changed) formatted by style or custom strftime format.
  - `git.rs`: Staged/unstaged status indicators (`.M`, `M.`, `UU`, `??`, etc.).
  - `inode.rs`, `links.rs`, `flags.rs`, `securityctx.rs`: Advanced OS attributes.

#### Lines of Code (LOC) Engine (`src/loc/` & `src/output/code.rs`)
- [`src/loc/mod.rs`](src/loc/mod.rs): Comment-aware source code line counter supporting 100+ programming languages. Uses static `phf_map` of filenames and extensions.
  - Classifies lines into: `code`, `comments` (line & block comment aware, string literal skip), and `blanks`.
  - Parallelized file scanning via `rayon`.
- [`src/output/code.rs`](src/output/code.rs): Formats the `--code` summary table with language names, file counts, line counts, and percentages.

#### Theming & Styling (`src/theme/`)
- [`src/theme/mod.rs`](src/theme/mod.rs) & [`src/theme/ui_styles.rs`](src/theme/ui_styles.rs): Complete ANSI style model for all UI elements (file kinds, permissions, sizes, dates, headers, git status).
- [`src/theme/lsc.rs`](src/theme/lsc.rs): Parser for standard `LS_COLORS` strings.
- [`src/theme/default_theme.rs`](src/theme/default_theme.rs): Built-in default color palette.

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
| Run all tests | `cargo test --workspace` | `just test` |
| Run integration/CLI tests | `cargo test --test cli_tests` | `just test` |
| Lint codebase | `cargo clippy` | `just clippy` |
| Format code | `cargo fmt` | `nix fmt` |
| Build man pages | `pandoc ...` | `just man` |

### Nix Environment (Optional)
```bash
nix develop       # Enter development shell
nix flake check   # Run complete validation suite
nix build         # Build package
```

---

## 4. Agent Guidelines & Rules for Contributing

When making modifications or adding new features to `lsr`:

### 1. Adding a New CLI Flag or Option
If adding a new CLI flag:
1. **Define argument** in [`src/options/parser.rs`](src/options/parser.rs) using `clap`.
2. **Handle option deduction** in [`src/options/mod.rs`](src/options/mod.rs) or the appropriate sub-options module (`view.rs`, `filter.rs`, `dir_action.rs`).
3. **Propagate into runtime** in [`src/main.rs`](src/main.rs) and the corresponding `src/output/` or `src/fs/` renderer.
4. **Update documentation**:
   - Add flag to [README.md](README.md) under "Command-line options".
   - Update man pages in [`man/eza.1.md`](man/eza.1.md).
5. **Update Shell Completions**:
   - [`completions/bash/eza`](completions/bash/eza)
   - [`completions/zsh/_eza`](completions/zsh/_eza)
   - [`completions/fish/eza.fish`](completions/fish/eza.fish)
   - [`completions/nush/eza.nu`](completions/nush/eza.nu)
   - [`completions/pwsh/_eza.ps1`](completions/pwsh/_eza.ps1)
6. **Add Unit/CLI Tests** in [`tests/`](tests/).

### 2. Adding or Modifying File Icons
- Edit [`src/output/icons.rs`](src/output/icons.rs).
- Use proper Unicode characters for Nerd Fonts glyphs.
- Register filename or extension in the `FILENAME_ICONS` or `EXTENSION_ICONS` PHF maps.

### 3. Adding New Programming Languages for `--loc` / `--code`
- Edit [`src/loc/mod.rs`](src/loc/mod.rs).
- Define `Language` with `name`, `line_comments`, and `block_comments`.
- Register the extension or whole filename in `BY_EXTENSION` or `BY_FILENAME`.
- Add test fixtures in `tests/itest-loc/` if applicable.

### 4. Cross-Platform Safety
- Platform-specific code MUST be guarded with `#[cfg(unix)]`, `#[cfg(target_os = "macos")]`, `#[cfg(target_os = "linux")]`, or `#[cfg(windows)]`.
- Always verify Windows compatibility (avoid raw Unix syscalls without fallbacks).
- Use `Path` / `PathBuf` methods rather than hardcoded `/` or `\` separators.

### 5. Code Quality & Commits
- Follow [Conventional Commits](https://www.conventionalcommits.org/) (e.g. `feat: ...`, `fix: ...`, `docs: ...`, `chore: ...`).
- **Granular Atomic Commits (1 Upstream Task = 1 Commit)**: Never bundle multiple independent upstream PRs or features into a single monolithic commit. Each upstream port or distinct feature must have its own separate, atomic commit with unit tests and clear commit messages. Batch PRs in `lsr` must contain at least 5 granular commits (one per item).
- Ensure all licenses comply with REUSE / SPDX guidelines (`EUPL-1.2` or `MIT`).
- Run `cargo clippy` and `cargo test --lib` before committing.
