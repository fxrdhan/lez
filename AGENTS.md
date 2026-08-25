<!--
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-License-Identifier: EUPL-1.2
-->

# AGENTS.md — Agent & Developer Guide for `lsr`

> **`lsr`** is a fast, modern, and feature-rich replacement for `ls` written in Rust (Rust 2024 edition, MSRV 1.90+).
> Lineage: `exa` (original by Benjamin Sago) ➔ `eza` (community fork) ➔ `lsr` (by fxrdhan).

---

## 1. System Overview & Architecture

`lsr` reads directories and files from the filesystem, extracts rich metadata (permissions, ownership, sizes, Git status, extended attributes, mounts, timestamps), formats them with syntax highlighting, Nerd Font icons, and renders them in various display modes (Grid, Long Details, Grid Details, One-line, Tree, Lines of Code Summary via `--code`, and JSON via `--json`).

Mode selection priority (`Mode::deduce` in `src/options/view.rs`): `--code` → `--json` → strict-mode checks → TTY default (Grid on TTY, Lines otherwise) → `--long` (+ `--grid` = GridDetails) → `--tree` → `--oneline`. Only `--binary`/`--bytes` still use last-argument-wins semantics (clap `overrides_with`).

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
- [`man/`](man/): Pandoc markdown sources for man pages (`lsr.1.md`, `lsr_colors.5.md`, `lsr_colors-explanation.5.md`). These three are the whole set: `tests/man_docs_tests.rs` fails if a page is added here without being wired into the `@man` and `@mangen` recipes. The `eza`-named copies that used to sit alongside them were dropped — they documented a command this project does not install, and had drifted into crediting upstream.
- [`completions/`](completions/): Shell completion scripts for `bash`, `zsh`, `fish`, `nushell`, and `powershell`.
- [`tests/`](tests/): Integration tests, snapshot tests powered by `trycmd`, LOC tests, and powertests.

---

### `src/` Architecture

#### Entry Point & Orchestration
- [`src/main.rs`](src/main.rs):
  - Configures logging (`logger::configure`) based on `LSR_DEBUG` / `EZA_DEBUG` / `EXA_DEBUG`.
  - Sets up signal handlers (e.g. `SIGPIPE` default handling on Unix).
  - Parses arguments via `options::parser::get_command().get_matches()`.
  - Initializes the `Lsr` struct holding `Options`, `Theme`, terminal width, and `GitCache`.
  - Runs `Lsr::run()`:
    - If `--code` mode: counts lines of code and renders language breakdown via `src/output/code.rs`.
    - If `--json` mode: serializes files/directories via `src/output/json.rs` (special-cased for multi-directory output).
    - Otherwise: wraps input paths into `File` / `Dir` structs, verifies metadata, sorts/filters, recurses if requested (`--recurse`, `--tree`), and delegates rendering to `print_files()` and `print_dirs()`.
  - Maps standard exit codes:
    - `0` (`SUCCESS`): Successful execution.
    - `1` (`RUNTIME_ERROR`): I/O or filesystem error encountered.
    - `2`: At least one input path does not exist (listing of remaining paths still proceeds).
    - `3` (`OPTIONS_ERROR`): Invalid or conflicting CLI options.
    - `13` (`PERMISSION_DENIED`): Skipped directories due to OS permission denial.
- [`src/lib.rs`](src/lib.rs): Re-exports modules for internal library tests and benchmarks.
- [`src/logger.rs`](src/logger.rs): Debug logging implementation.
- [`src/info/`](src/info/): Internal "business logic" on already-read metadata (not filesystem probing nor output): `filetype.rs` (file type classification) and `sources.rs`.

#### CLI & Configuration (`src/options/`)
- [`src/options/parser.rs`](src/options/parser.rs): Defines the `clap::Command` CLI specification with all flags, options, headings, value parsers, and defaults.
- [`src/options/mod.rs`](src/options/mod.rs): Combines sub-options into `Options`. Deduction order: View → DirAction → FileFilter → Theme → FilesInput.
- [`src/options/filter.rs`](src/options/filter.rs): Handles filtering flags (`-a`/`--all`, `-D`/`--only-dirs`, `-f`/`--only-files`, `--git-ignore`, `-I`/`--ignore-glob`, `--ignore-glob-ci`, `--since <DURATION>`, `--no-symlinks`, `--show-symlinks`, `--sort`, `--reverse`). Sort aliases: `newest`/`new` → ModifiedAge (newest first), `old`/`oldest` → ModifiedDate.
- [`src/options/view.rs`](src/options/view.rs): Handles view mode selection (`Grid`, `Details`, `GridDetails`, `Lines`, `Code`, `Json`), column toggles, `--absolute`, `--dereference`, `--hyperlink`, `--icons`, `--summary`, `--print-total`.
- [`src/options/theme.rs`](src/options/theme.rs) & [`src/options/config.rs`](src/options/config.rs): Parses `theme.yml` configuration (from `$LSR_CONFIG_DIR`, `$EZA_CONFIG_DIR`, or `$XDG_CONFIG_HOME/lsr|eza`) and color variables. YAML sections: `filekinds`, `perms`, `size`, `users`, `links`, `git`, `git_repo`, `security_context`, `file_type`, `tags`, scalar UI styles, plus per-name maps `filenames`, `extensions`, `directorynames`.
- [`src/options/vars.rs`](src/options/vars.rs): Declares environment variables (`LS_COLORS`, `LSR_COLORS`/`EZA_COLORS`/`EXA_COLORS`, `NO_COLOR`, `COLUMNS`, `TIME_STYLE`, `EZA_STRICT`, `EZA_DEBUG`, `EZA_GRID_ROWS`, `EZA_ICON_SPACING`, `EZA_ICONS_AUTO`, `EZA_STDIN_SEPARATOR`, `LSR_MIN_LUMINANCE`/`LSR_MAX_LUMINANCE`, etc.) and the mockable `Vars` trait. Precedence for duplicated vars: `LSR_*` > `EZA_*` > `EXA_*`.
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
- [`src/fs/dir_action.rs`](src/fs/dir_action.rs): Handles directory encounter behavior (`--recurse`, `--tree`, `--level`, `--treat-dirs-as-files`).
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
- [`src/output/json.rs`](src/output/json.rs): JSON output mode (`--json`); reuses the same `Column` collection as the long view so `-l` and `--json -l` stay in parity.
- [`src/output/summary.rs`](src/output/summary.rs): Total counts summary footer (`--summary`).
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
  - `language.rs`, `loc.rs`: Language name and lines-of-code columns (`--loc`).

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

### Test Layers
| Layer | Location | How to run |
|---|---|---|
| Unit tests (inline `#[cfg(test)]`) | throughout `src/` | `cargo test --lib` |
| Rust integration tests | `tests/*.rs` | `cargo test --workspace` |
| trycmd CLI snapshots | `tests/cmd/*.toml` + fixtures in `tests/itest/`, `tests/itest-loc/` | `cargo test --test cli_tests` |
| Generated snapshots (nix-gated) | `tests/gen/` | nix build (`just itest`) |
| Powertest corpus (feature-gated) | `tests/ptests/` | built via powertest tool (`just regen`) |

Snapshot regeneration: `just idump` (refresh `.stdout`/`.stderr` dumps) and `just regen` (regenerate powertest cases from `powertest.yaml`). `just regen` is idempotent — on a clean tree it rewrites `tests/ptests` byte for byte — and `tests/powertest_config_tests.rs` holds the generator config to the rules that keep it that way. See [TESTING.md](TESTING.md).

The last two layers need `tests/test_dir`, which only the Nix build produces (`devtools/dir-generator.sh` calls `groupadd`, so macOS cannot create it). They therefore do not run in an ordinary `cargo test`, and drift in them used to surface only in CI. Two guards stand in for them locally: `tests/generated_suite_arguments_tests.rs` parses every generated case's arguments and fails if anything but a fixture path reaches the positional slot, and enabling `--features powertest` or `--features nix` without the fixture now fails outright instead of skipping in silence.

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
2. **Handle option deduction** in [`src/options/mod.rs`](src/options/mod.rs) or the appropriate sub-options module (`view.rs`, `filter.rs`; recursion behavior lives in `src/fs/dir_action.rs`).
3. **Propagate into runtime** in [`src/main.rs`](src/main.rs) and the corresponding `src/output/` or `src/fs/` renderer.
4. **Update documentation**:
   - Add flag to [README.md](README.md) under "Command-line options".
   - Update man pages in [`man/lsr.1.md`](man/lsr.1.md).
5. **Update Shell Completions**:
   - [`completions/bash/lsr`](completions/bash/lsr) — derives its flag list from `lsr --help`, so it only needs value lists
   - [`completions/zsh/_lsr`](completions/zsh/_lsr)
   - [`completions/fish/lsr.fish`](completions/fish/lsr.fish)
   - [`completions/nush/lsr.nu`](completions/nush/lsr.nu)
   - [`completions/pwsh/_lsr.ps1`](completions/pwsh/_lsr.ps1)

   Then regenerate the `eza`-named compatibility copies with
   `sed 's/lsr/eza/g'` — that is the whole rule for all five backends, and
   `the_compat_copies_are_the_primary_files_with_the_name_rewritten` in
   `tests/completion_equals_tests.rs` fails if a copy is edited instead.

   If the flag takes its value with `require_equals`, every backend has to
   offer it *after* the sign and not after a space, or completing it builds a
   command line where the value is read as a path. Each backend needs a
   different mechanism for that — see the same test file, which reads the set
   of equals-only flags off the clap command, so a new one fails the suite
   until all five are taught about it.
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
- **Upstream references live in the PR body only**: commit messages must NOT tag upstream (`eza#NNNN` or `eza-community/eza#NNNN`) — describe the change itself instead. The PR body carries the single authoritative mapping as full markdown links: `[eza-community/eza#1667](https://github.com/eza-community/eza/pull/1667)`. This keeps the upstream timeline to one cross-reference event per item (from the PR body), avoiding duplicate "mentioned this" events + notifications that pushing tagged commits would create. Plain `eza#NNNN` / bare `#NNNN` are dead text anyway: GitHub only auto-links `#N` within the same repository.
- **Granular Atomic Commits (1 Upstream Task = 1 Commit)**: Never bundle multiple independent upstream PRs or features into a single monolithic commit. Each upstream port or distinct feature must have its own separate, atomic commit with unit tests and clear commit messages. Batch PRs in `lsr` must contain at least 5 granular commits (one per item).
- Ensure all licenses comply with REUSE / SPDX guidelines (`EUPL-1.2` or `MIT`).
- Run `cargo clippy` and `cargo test --lib` before committing.

---

## 5. Upstream Triage Policy

`lsr` ports unmerged `eza-community/eza` work by hand; it does not sync (see
§1). That makes "which upstream work is still outstanding" a question someone
re-asks every few months, so the answer is written down here rather than
rediscovered.

**Snapshot taken 2026-08-24**, against 149 open upstream PRs and 286 open
upstream issues. 106 PRs were already ported. Of the remaining 43, one was
worth taking; the other 42 are listed below with the reason they are not.
"Not ported" here means *examined and declined*, not *unseen*.

### PRs deliberately not ported

| Reason | Upstream PRs |
|---|---|
| **Dependency bumps.** We keep our own `Cargo.lock` and pins, several already ahead of upstream. | 1543, 1554, 1575, 1607, 1659, 1660, 1666, 1703, 1745, 1749 |
| **Infrastructure specific to eza.** Crane migration, the `deb.gierens.de` APT matrix, Miri, `cargo shear`, their RISC-V and musl release targets, their cross-build containers. | 462, 971, 972, 1537, 1538, 1629, 1753, 1777, 1861, 1869, 1890, 1901 |
| **eza's own branding and README.** 1713 renames leftover `exa` strings, which we did to `lsr` in PR #37. | 1625, 1713, 1755, 1756, 1914 |
| **Already covered by a port we took.** 913 ← 925 (WSL hyperlinks), 1596 ← 1923 (stdin), 1838/1840/1844 ← 1848 (all three are the same non-UTF-8 `--time-style` fix), 1233/1504 ← commit `cfe0abb7`, which removed the Windows `_`-prefix filter outright. | 913, 1233, 1504, 1596, 1838, 1840, 1844 |
| **Dead or disproportionate.** 974 is ±25k lines of churn under `CHANGES_REQUESTED`; 1765 touches 744 files; 936 fixes clippy warnings we do not have; 575 waits on an upstream design decision that never came; 1658 conflicts. | 575, 936, 974, 1658, 1765 |
| **Product decisions, not oversights.** 770 (`--no-header`) needs a config file for option defaults, which we do not have; 1903 was solved our own way in PR #26. | 770, 1804, 1903 |

Revisit an entry only if its reason stops holding — for instance if `lsr` gains
a config file for option defaults, 770 becomes worth a look.

### Filtering the issues

**Swept 2026-08-24.** All 98 open `type: bug` reports (of 286 open issues) were
read and, where possible, run against our binary. 46 were already fixed here.
What remains is recorded below rather than left to be rediscovered.

Work an unread report in this order, because it is by far the cheapest:

1. **Reproduce against our binary before reading further.** Most reports die
   here — we fixed them through a different port, and the issue title gives no
   hint of that.
2. **Read the body and the whole discussion, not the title.** Titles
   under-describe and mislead. eza#521 is filed as `--git` not marking `I`,
   and the same root cause also broke `--git-ignore`. eza#825 is filed as
   missing icons, and its own thread traces them to an iTerm bug. eza#337 had a
   working fix in 2024 that never merged, which the title does not say.
3. **Drop eza's own infrastructure** — winget, `deb.gierens.de` builds, their
   flake, trycmd on ZFS.
4. **Group the rest by theme, one PR per theme, not one per issue.**

### What a machine here can and cannot reach

Correcting an assumption that cost time: **Linux containers run on macOS**, so
a Linux-only report is reproducible in minutes — build inside `rust:*-bookworm`
with `CARGO_TARGET_DIR=/tmp/target` and a mounted read-only checkout. Only
*Windows* containers cannot run here. Windows instead has the
`Windows Repro Probe` workflow (`.github/workflows/windows-probe.yml`), a
manual job that walks a set of reproductions on a runner and prints the result.

Out of reach either way: real hardware (a specific USB device, a CIFS or
macFUSE mount), systemd-homed, iSH, and a GUI terminal's font.

### Still open upstream, with what is known

| upstream | state |
|---|---|
| 875 | Alpine on iSH. Needs iSH; no code lead. |
| 1088 | `LS_COLORS` `ca` (Linux capabilities) unsupported. Reproducible on Linux, not on macOS; see issue #60. |
| 1428 | systemd-homed user names. We already resolve through `uzers` → `getpwuid_r` → NSS, which is the right API; nothing to act on without the setup. |
| 1500 | Hang on one USB device. Needs that device. |
| 1548 | Control-character quoting. Wants a `--quote-style` option: a design decision, not a fix. |
| 743 | `--color-scale` under tmux on Wayland. The flat gradient was ours and is fixed; whether truecolor also fails there is unproven. |
| 844, 1214, 1378, 1710 | Performance at 200k+ files. Does not reproduce at 12k; see issue #59. |
| 337, 404, 853, 1025, 1104, 1220, 1665, 1769 | Windows. Settled — see issue #57. |

Declined issues, with the evidence for declining them, are in the agent's
memory rather than here: they are conclusions about upstream reports, not rules
for this repository.

### Filtering the feature requests

**Swept 2026-08-25.** The remaining 180 open issues — 160 `type: feature` plus
the 20 labelled `chore`/`docs`/nothing — were read in full and, where the claim
was testable, run against our binary. The point of writing this down is that
**a large share of them are already implemented here**: a request that upstream
still lists as open is not evidence of a gap in `lsr`.

Reproduce before believing the label. Several of these are filed as features
but are defects (653 is a DST bug, 1904 is quadratic behaviour), and several
filed as bugs are satisfied by a flag we already ship.

#### Already delivered here

| upstream | what answers it |
|---|---|
| 341, 1847 | `--summary`, `--print-total` |
| 420 | `--ignore-submodule-contents` |
| 443, 710 | `--no-extended` |
| 472, 768 | `--json` — keyed objects, one per entry |
| 516 | `--show-symlinks` / `--no-symlinks` |
| 520, 1003 | `--spacing` |
| 589 | `--warn-hidden` |
| 630 | `--help` shows `--color-scale[=<FIELDS>...]` with its values |
| 653 | timezone offsets track DST; verified Feb/Jun/Nov across `TZ` |
| 736, 980, 1573 | `-t` / `-lt` / `-lrt` match `/bin/ls` byte for byte (`normalize_short_time_arg`) |
| 889 | `--octal-permissions --no-permissions` |
| 921 | `-d` with `--stdin` |
| 948 | `--cachedir-ignore` |
| 981 | `--absolute` documented, `SEE ALSO` uses man notation, `$version` is substituted by `just man` |
| 1042, 1737 | Jenkinsfile, bicep, bicepparam icons |
| 1073 | `--mime-types` |
| 1090 | `--absolute` |
| 1123 | `--quotes=always` |
| 1141 | multiple path arguments obey `--sort` |
| 1219 | the `ca` half — `LS_COLORS` capability styling (PR #66). Decoding the xattr into `cap_…=eip` is **not** done |
| 1446, 1778 | `--ignore-glob '**/dir/*'` hides contents and keeps the directory |
| 1484 | `completions/pwsh` |
| 1540 | `--no-symlink-targets` |
| 1616 | `-H` |
| 1657 | `--time-style relative-recent` |
| 1734 | `libgit2-sys 0.18.5+1.9.4`, past the 1.9.2 advisories |
| 1746 | `--follow-symlinks`, `-X` |
| 1750 | `--ignore-glob-ci` |
| 1773 | `--hyperlink[=WHEN]` |
| 1835 | `--sort=path` |
| 1904 | `Dir::contains` memoises into a set; 5000 `.log` files list in 0.07s |
| 1912 | `palette_derive` pinned to `=0.7.5` beside `palette` |
| 223, 579(1) | the Git column is dropped when nothing in the listing is in a repo |

Partly delivered, with the remainder named: 584 (`--quotes` yes, `-N` and
`QUOTING_STYLE` no), 600 (`.tar` yes, `.zip` no), 1466 (`--context` yes, MCS
translation no), 1571 (`--flags` on Mac/BSD/Windows, no Linux `lsattr`), 1642
(`--blocksize` in bytes, no block count), 1735 (csv/sqlite yes,
parquet/hdf5/npy no), 1768 (`--show-dotfiles` yes, the other two axes no), 1823
(`--git-glyphs` yes, choosing the glyph no).

#### Reproduced here, worth fixing

| upstream | reproduction |
|---|---|
| 509, 1743, 1448, 1892 | `natord` is the only name comparator. `LC_ALL=C lsr -1` over `00`–`FF` gives `0A 0B … 00 01`, where `ls` gives `00 01 … 0A 0B`. `--sort=name`, `.name`, `Name` all share it, so there is no lexicographic option at all. One fix closes four reports. |
| 1919 | `script.m` and `foo.c` both render `U+E61E`. One table entry. |
| 1868 | `--code` skips hidden directories and `-a` does not change that: 3 of this repo's 14 YAML files are counted, missing all 11 under `.github/`. |
| 922, 558 | one `write` per entry and one `statx` per entry even for `-1`. Our own Linux probe: 2000 files → 2000 `write`, 2006 `statx`. `ls` does 71 and 0. |
| 728 | `lsr -1 '/p/a b/c d.txt'` prints `'/p/a b'/'c d.txt'` — each component quoted separately. It still pastes back correctly, so this is cosmetic. |
| 1498 | `--total-size` walks hidden directories whether or not `--all` is given. Upstream disagrees on whether that is wrong; a directory's size arguably includes its hidden children. |

#### Not worth taking

Same reasons as the PR table: eza's own packaging and infrastructure (347, 475,
601, 646, 919, 930, 951, 1033, 1040, 1372, 1561, 1605, 1610, 1621, 1670, 1748,
1776, 1876, plus the doc issues 969, 1099, 1100, 1487 which describe eza's own
prose), and eza's governance (1872).

Declined on the evidence in their own threads: 479 (PGO measured at ~1%, by the
person proposing it), 756 (eza deliberately never reads file contents), 729
(`lib.rs` is not a public API), 1117 (the proposed Unicode icons do not render
on the platforms in the thread), 905 and 1401 (blocked on Windows ACL semantics
and on libgit2's sha256 support respectively).

Everything not named above is a real gap and a product decision, not an
oversight. The largest cluster by far is **139 — a configuration file for
option defaults**, which also unblocks 766, 812, 1587, 1707, 1875 and upstream
PR 770.
