<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileCopyrightText: 2026 fxrdhan
SPDX-FileContributor: Christina Sørensen
SPDX-FileContributor: fxrdhan

SPDX-License-Identifier: EUPL-1.2
-->

<div align="center">
    
# lez

**An alternative for `ls` written in Rust.**

[![License](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/Rust-1.90%2B-orange.svg)](https://www.rust-lang.org/)
[![binary cache](https://img.shields.io/endpoint?url=https%3A%2F%2Flez.cachix.org%2Fapi%2Fv1%2Fcache%2Fbadges%2Fshield.svg)](https://app.cachix.org/cache/lez)

</div>

![lez demo](docs/images/screenshots.png)

---

**`lez`** is a fast, modern file-listing command-line tool with smart defaults, enhanced file icons, Git integration, and continuous performance improvements.

- **Fast & Lightweight:** Written in modern Rust (2024 Edition) with multithreaded directory scanning via Rayon.
- **Rich Visuals:** Syntax highlighting, colored CLI help output, Nerd Font icons, and automatic luminance color scaling.
- **Git Integration:** View file and repo status (`M`odified, `U`ntracked, `I`gnored, etc.) directly in the file listing.
- **Built-in Tree View:** Hierarchical directory tree out of the box (`lez --tree`).
- **Structured Data Export:** Full metadata serialization via `--json` in complete parity with the long view.
- **Archive Inspection:** Inspect files inside `.tar` archives directly in the long view (`lez -l --inspect-archives`).
- **Lines-of-Code Counter:** Comment-aware LOC breakdowns for 100+ programming languages (`lez --code`).
- **Deep OS Integration:** Native macOS Finder color tags, Linux capability decoding (`security.capability`), and Windows `PATHEXT` executables.

---

# Performance

Measured against [eza](https://github.com/eza-community/eza) v0.23.5 and [lsd](https://github.com/lsd-rs/lsd) v1.2.0. Apple M1 Pro, 10 cores, macOS; release builds, warm page cache, minimum of 35 interleaved runs per binary.

| Workload | `lez` | `eza` 0.23.5 | `lsd` 1.2.0 | vs `eza` | vs `lsd` |
| --- | ---: | ---: | ---: | ---: | ---: |
| Grid view, 10,000 files | **34.4 ms** | 68.6 ms | 72.1 ms | 2.00× | 2.10× |
| Recursive tree, 29,572 files | **308.9 ms** | 565.9 ms | 1,482.4 ms | 1.83× | 4.80× |
| Directory-grouped, 10,000 entries incl. 3,000 symlinks | **186.1 ms** | 312.8 ms | 338.2 ms | 1.68× | 1.82× |
| Long view, 10,000 files | **162.2 ms** | 175.7 ms | 185.3 ms | 1.08× | 1.14× |
| Long view with Git status, this repository | 22.3 ms | **20.8 ms** | 26.9 ms | 0.93× | 1.21× |

Where the gains come from: symlink directory lookups are cached, so the grouping sort resolves each entry once instead of on every comparison; sorts above 2,048 entries run on all cores; and the traversal itself avoids redundant allocations and stats.

The last row is the one case where eza is ahead, by about a millisecond and a half on a listing that is dominated by libgit2. Note also that `lez` sorts through an ICU collator by default, which orders accents, case, and digit runs correctly and costs more than a byte comparison. `--sort=lexicographic` opts out of it and is faster still.

### Reproducing

```bash
# 10,000 files in one flat directory
mkdir -p /tmp/bench && cd /tmp/bench && seq 1 10000 | xargs -I{} touch file_{}.txt

hyperfine --warmup 3 'lez /tmp/bench' 'eza /tmp/bench' 'lsd /tmp/bench'
hyperfine --warmup 3 'lez -l /tmp/bench' 'eza -l /tmp/bench' 'lsd -l /tmp/bench'
hyperfine --warmup 3 'lez --tree ~/.cargo/registry' 'eza --tree ~/.cargo/registry' 'lsd --tree ~/.cargo/registry'
```

`eza` requires `--icons=auto` where `lez` accepts a bare `--icons`; passing a bare `--icons` to `eza` before a path makes it reject the path as an invalid value, which produces a misleadingly fast result.

---

# Feature Matrix

| Feature / Capability | `lez` | `eza` | `lsd` |
| :--- | :---: | :---: | :---: |
| **Nerd Font Icons & Color Themes** | ✅ | ✅ | ✅ |
| **Directory Tree View** (`--tree`) | ✅ | ✅ | ✅ |
| **Hyperlink Support** (`--hyperlink` OSC 8) | ✅ | ✅ | ✅ |
| **Custom Column Order** (`--blocks`) | ❌ | ❌ | ✅ |
| **Classic GNU `ls` Mode** (`--classic`) | ❌ | ❌ | ✅ |
| **Unicode Emoji Fallback** (`--icon-theme unicode`) | ❌ | ❌ | ✅ |
| **Multithreaded Traversal** (Rayon Engine) | ✅ | ⚠️ Limited | ❌ |
| **Lines-of-Code Counter** (`--code`, `--loc`) | ✅ 100+ langs | ✅ 50+ langs | ❌ |
| **Structured JSON Export** (`--json`) | ✅ | ❌ | ❌ |
| **Archive Inspection** (`--inspect-archives` for `.tar`) | ✅ | ❌ | ❌ |
| **Time-Window Filtering** (`--since`) | ✅ | ❌ | ❌ |
| **Size Precision Formatting** (`--size-digits`) | ✅ | ❌ | ❌ |
| **Nix Store Hash Abbreviation** (`--short-nix`) | ✅ | ❌ | ❌ |
| **Global & Per-Directory Config Files** (`config.toml` / `.lez.toml`) | ✅ | ❌ | ✅ |
| **Deep Git Integration** (`--git`, `--git-glyphs`, `--git-repos`) | ✅ | ✅ | ⚠️ Basic |
| **macOS Finder Color Tags** (`-e`, `--tags`) | ✅ | ✅ | ❌ |
| **BSD & macOS File Flags** (`-O`, `--flags`) | ✅ | ✅ | ❌ |
| **Filesystem Mount Points** (`-M`, `--mounts`) | ✅ | ✅ | ❌ |
| **Automatic Color Scaling & Heatmap** (`--color-scale`) | ✅ | ✅ | ❌ |

---

<a id="try-it">
<h1>Try it!</h1>
</a>

### Cargo / Build from Source

Install `lez` from crates.io:

```bash
cargo install lez
```

Or track the latest commit on `main`:

```bash
cargo install --git https://github.com/fxrdhan/lez.git
```

Or build from source:

```bash
git clone https://github.com/fxrdhan/lez.git
cd lez
cargo build --release
# Binary available at target/release/lez
```

### Nix

If you already have Nix setup with flake support, you can try out `lez` with the `nix run` command:

```bash
nix run github:fxrdhan/lez
```

Nix will build `lez` and run it.

If you want to pass arguments this way, use e.g. `nix run github:fxrdhan/lez -- -la --icons`.

#### Binary Cache

Every commit on `main` is validated with `nix flake check` in CI, and the resulting store paths are pushed to a public binary cache on [Cachix](https://www.cachix.org). Contributors and Nix users can pull prebuilt outputs instead of compiling from scratch:

```bash
cachix use lez
nix run github:fxrdhan/lez
```

**Performance & Validation:**

| Scenario | What Happens | Duration |
|---|---|---|
| **Warm Run / Downstream Users** | Full closure substituted directly from Cachix | **~33 s** *(Instant download)* |
| **Cold Build (Building from Source)** | Single-pass compilation in isolated Nix sandbox | **~7 min** |

> **Note on Caching**: Nix store paths are strictly content-addressed. Builds with modified source code produce a new derivation hash and compile inside the Nix sandbox, while unchanged closures, downstream `nix develop` environments, and CI runs on `main` enjoy the ~33 s binary substitution. A weekly cold-build canary keeps the flake honest against upstream bitrot.

---

# Installation

`lez` is available for macOS, Linux, and Windows. Detailed platform-specific installation instructions can be found in [INSTALL.md](INSTALL.md).

---

<a id="options">
<h1>Command-line options</h1>
</a>

`lez`’s options are intuitive and familiar. Quick overview:

## Display options

<details>
<summary>Click to expand</summary>

- **-1**, **--oneline**: display one entry per line
- **-G**, **--grid**: display entries as a grid (default)
- **-l**, **--long**: display extended details and attributes
- **-R**, **--recurse**: recurse into directories
- **-T**, **--tree**: recurse into directories as a tree
- **--follow-symlinks**: drill down into symbolic links that point to directories
- **--code[=MODE]**: print lines-of-code summary by language (modes: `lines`, `percent`, `both`)
- **--json**: output file listing and metadata as structured JSON
- **-x**, **--across**: sort the grid across, rather than downwards
- **-F**, **--classify[=(when)]**: display type indicator by file names (always, auto, never)
- **--colo[u]r=(when)**: when to use terminal colours (always, auto, never)
- **--colo[u]r-scale=(fields)**: highlight levels of `fields` distinctly (all, age, size)
- **--color-scale-mode=(mode)**: use gradient or fixed colors in `--color-scale` (`fixed` or `gradient`)
- **--icons[=(when)]**: when to display icons (always, auto, never; requires '=' if value provided)
- **--spacing=(spaces)**: number of spaces between columns in grid views (default: 2, range: 0..=255)
- **--no-symlink-targets**: do not show symlink targets (the `-> ...`)
- **--quotes=(when)**: when to quote file names (always, auto, never; requires '=' if value provided)
- **--summary**: display total summary statistics of entries (directories, files, symlinks, and total)
- **--hyperlink[=(when)]**: when to display entries as hyperlinks (always, auto, never; requires '=' if value provided)
- **--absolute=(mode)**: display entries with their absolute path (on, follow, off)
- **--short-nix**: abbreviate Nix store hashes in file names and paths
- **--print-total**: print the total number of files and directories listed
- **--mime-types**: determine file MIME types to better inform styling decisions (unix only)
- **-w**, **--width=(columns)**: set screen width in columns (clamped to `1..65535`)

</details>

## Filtering options

<details>
<summary>Click to expand</summary>

- **-a**, **--all**: show hidden and 'dot' files (use twice to also show `.` and `..`)
- **-A**, **--almost-all**: equivalent to `--all`; included for compatibility with `ls -A`
- **--show-dotfiles**: show dot-prefixed files without showing other hidden files
- **-d**, **--treat-dirs-as-files**: list directories like regular files
- **-L**, **--level=(depth)**: limit the depth of recursion
- **-r**, **--reverse**: reverse the sort order
- **-s**, **--sort=(field)**: which field to sort by; the path field accepts the aliases `relative-path`, `relpath`, and `relative_path` (capitalised variants sort uppercase first)
- **-t**: sort by modification time, newest first (GNU `ls` compatibility; shorthand for `--sort=age`)
- **--group-directories-first**: list directories before other files
- **--group-directories-last**: list directories after other files
- **-D**, **--only-dirs**: list only directories
- **-f**, **--only-files**: list only files
- **--no-symlinks**: don't show symbolic links
- **--show-symlinks**: explicitly show links (with `--only-dirs` and `--only-files`)
- **--git-ignore**: ignore files mentioned in `.gitignore`
- **-W**, **--warn-hidden**: print a tally of hidden and gitignored entries; give twice to always print it
- **--cachedir-ignore**: ignore directories containing a `CACHEDIR.TAG` file
- **--ignore-submodule-contents**: don't list the contents of Git submodules
- **--since=(duration)**: filter and display only files created or modified within the specified duration window (e.g. 10m, 1h, 2d, 1w)
- **-I**, **--ignore-glob=(globs)**: glob patterns (pipe-separated) of files to ignore; patterns containing `/` match against paths relative to the listing root and the flag may be given multiple times
- **--ignore-glob-ci=(globs)**: glob patterns (pipe-separated) of files to ignore (case-insensitive)

</details>

## Long view options

<details>
<summary>Click to expand</summary>

These options are available when running with `--long` (`-l`):

- **-b**, **--binary**: list file sizes with binary prefixes (overrides `--bytes` if passed after)
- **-B**, **--bytes**: list file sizes in bytes, without any prefixes (overrides `--binary` if passed after)
- **--size-digits=(NUM)**, **--digits=(NUM)**: number of digits to display for file sizes (1..=8, default: 3; also configurable via `LEZ_SIZE_DIGITS`)
- **-g**, **--group**: list each file’s group
- **--smart-group**: only show group if it has a different name from owner (automatically enables group column)
- **-n**, **--numeric**: show user and group as their numeric IDs
- **-h**, **--header**: add a header row to each column
- **-H**, **--links**: list each file’s number of hard links
- **-i**, **--inode**: list each file’s inode number
- **--loc[=MODE]**: display language and lines-of-code columns (modes: `lines`, `percent`, `both`)
- **-m**, **--modified**: use the modified timestamp field
- **-M**, **--mounts**: show mount details (Linux and macOS only)
- **-S**, **--blocks**, **--blocksize**: show size of allocated file system blocks
- **-t**, **--time=(field)**: which timestamp field to use (modified [aliases: mod, m], accessed [acc], changed [ch], created [cr])
- **-u**, **--accessed**: use the accessed timestamp field
- **-U**, **--created**: use the created timestamp field
- **--changed**: use the changed timestamp field
- **--utc**: show timestamps in the UTC timezone
- **-X**, **--dereference**: dereference symlinks for file information and sorting
- **-Z**, **--context**: list each file’s security context
- **-O**, **--flags**: list file flags (macOS, BSD, and Windows only)
- **-@**, **--extended**: list each file’s extended attributes and sizes
- **--no-extended**: don't show the `@` marker that a file has extended attributes
- **-e**, **--tags**: list each file's color tags stored in extended attributes (macOS Finder tags)
- **--inspect-archives**: list the contents of supported archives (.tar) in long view, with each entry's file name coloured by type
- **--git**: list each file’s Git status, if tracked or ignored
- **--git-glyphs**: display Git status with Nerd Font glyphs instead of ASCII characters
- **--git-repos**: list each directory’s Git status, if tracked
- **--git-repos-no-status**: list whether a directory is a Git repository, but not its status (faster)
- **--no-git**: suppress all Git fields and `.gitignore` handling (overrides `--git`, `--git-repos`, `--git-repos-no-status`, `--git-ignore`)
- **--time-style**: how to format timestamps. Valid styles: `default`, `iso`, `long-iso`, `full-iso`, `relative`, `relative-recent`, or custom `+<FORMAT>` (e.g., `+%Y-%m-%d %H:%M`; see [_`lez(1)` manual page_](man/lez.1.md) and [chrono format](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)).
- **--total-size**: show recursive directory size
- **-o**, **--octal-permissions**: list each file's permission in octal format
- **--no-permissions**: suppress the permissions field
- **--no-filesize**: suppress the filesize field
- **--no-user**: suppress the user field
- **--no-time**: suppress the time field
- **--stdin**: read file names from stdin
- **--config**: load default options from specified configuration file (`.toml`, `.yaml`, or `.yml`)
- **--no-config**: do not load any global or per-directory configuration files

Some of the options accept parameters:

- Valid **--colo\[u\]r** options are **always**, **automatic** (or **auto** for short), and **never**.
- Valid sort fields are **accessed**, **changed**, **created**, **extension**, **Extension**, **inode**, **lexicographic**, **Lexicographic**, **modified**, **name**, **Name**, **path**, **Path**, **size**, **block**, **type**, and **none**. Fields starting with a capital letter sort uppercase before lowercase. The modified field has the aliases **date**, **time**, **mod**, **old**, and **oldest**, while its reverse has the aliases **age**, **new**, and **newest**. The **block** field has the aliases **blocks** and **blocksize**. The **lexicographic** field has the aliases **lex** and **lg**, and compares names code point by code point — no natural ordering of digit runs, no locale collation — so **Lexicographic** matches `ls` under the C locale.
- Valid time fields are **modified**, **changed**, **accessed**, and **created**.
- Valid time styles are **default**, **iso**, **long-iso**, **full-iso**, **relative**, and **relative-recent** (or **recent**).

See the `man` pages for further documentation of usage. They are available:
- online [in the repo](https://github.com/fxrdhan/lez/tree/main/man)
- in your terminal via `man lez`
</details>

## Configuration Files

<details>
<summary>Click to expand</summary>

**`lez`** supports structured configuration files (TOML or YAML) to set default flags without needing cumbersome shell aliases or wrapper scripts.

### Discovery & Precedence

1. **CLI Flag**: `--config <PATH>` or `--no-config`
2. **Environment Variable**: `LEZ_CONFIG_FILE`
3. **Local (per-directory) Config**: `.lez.toml`, `.lez.yaml`, `.lez.yml`, `.eza.toml`, `.eza.yaml` in the current working directory
4. **Global Config**: `config.toml`, `lez.toml`, `config.yaml` in `$LEZ_CONFIG_DIR` (or `$XDG_CONFIG_HOME/lez`, `~/.config/lez`)
5. **Built-in Defaults**

### Example `config.toml`

```toml
[display]
header = true
time_style = "relative-recent"

[icons]
icons = "auto"
spacing = 1

[filter]
git_ignore = true

[git]
git_glyphs = true
```

An annotated sample configuration is provided in [`docs/config.example.toml`](docs/config.example.toml).

</details>

## Environment Variables

<details>
<summary>Click to expand</summary>

`lez` supports several environment variables to configure default behavior, styles, and integrations:

| Variable | Description |
|---|---|
| `LEZ_CONFIG_FILE` / `EZA_CONFIG_FILE` | Explicit path to a configuration file to load (`.toml`, `.yaml`, or `.yml`). |
| `LEZ_CONFIG_DIR` / `EZA_CONFIG_DIR` | Directory containing `config.toml` and `theme.yml` (default: `$XDG_CONFIG_HOME/lez` or `~/.config/lez`). |
| `LEZ_COLORS` / `EZA_COLORS` / `LS_COLORS` | Specifies color styles and file extensions styling using standard terminal ANSI escape codes. |
| `LEZ_MIN_LUMINANCE` / `LEZ_MAX_LUMINANCE` | Minimum and maximum luminance values (0..=100) for color scaling on dates and sizes. |
| `LEZ_QUOTING_STYLE` / `EZA_QUOTING_STYLE` | Default quoting style for filenames with spaces/special characters (`always`, `auto`, `never`). |
| `LEZ_ICON_SPACING` / `EZA_ICON_SPACING` | Number of spaces to insert after Nerd Font icons (default: `1`). |
| `LEZ_NO_EMPTY_DIR_ICON` / `EZA_NO_EMPTY_DIR_ICON` | Set to anything to give every directory the same icon. Distinguishing an empty one costs a filesystem round trip per directory, which is slow on FUSE and network mounts. |
| `LEZ_STDIN_SEPARATOR` / `EZA_STDIN_SEPARATOR` | Delimiter for paths read from standard input with `--stdin` (default: newline `\n`). |
| `LEZ_SIZE_DIGITS` / `EZA_SIZE_DIGITS` | Default number of digits (1..=8) to display for formatted file sizes (default: `3`). |
| `LEZ_OVERRIDE_AUTO_COLOR` | Force automatic color detection behavior. |
| `TIME_STYLE` | Default timestamp format style (`default`, `iso`, `long-iso`, `full-iso`, `relative`, `relative-recent`, or `+<FORMAT>`). |
| `NO_COLOR` / `CLICOLOR` / `CLICOLOR_FORCE` | Standard terminal color control flags. |

</details>

## Custom Themes & Schema Validation

<details>
<summary>Click to expand</summary>

**`lez`** supports a `theme.yml` file, where you can customize theme options available for the `LS_COLORS`, `EZA_COLORS`, and `LEZ_COLORS` environment variables, as well as specify custom icons for different file types and extensions.

An example theme file is available in [`docs/theme.yml`](docs/theme.yml), and can be placed in a directory specified by 
`$LEZ_CONFIG_DIR`, `$EZA_CONFIG_DIR`, or looked for by default in `$XDG_CONFIG_HOME/lez` or `$XDG_CONFIG_HOME/eza`.

### Schema Validation in IDEs

You can enable autocomplete and schema validation in VSCode, Neovim, or Zed by referencing the bundled JSON Schema at the top of your `theme.yml`:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/fxrdhan/lez/main/docs/theme-schema.json

filekinds:
  directory: { foreground: Blue, bold: true }
  symlink: { foreground: Cyan }
```

Full styling details are available in the [lez_colors-explanation(5) man page](man/lez_colors-explanation.5.md) and [lez_colors(5) man page](man/lez_colors.5.md).

</details>

# Contributing to lez

If you want to contribute to `lez`, please check out our:
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

---

# Lineage & Credits

**`lez`** is a fork of [**eza**](https://github.com/eza-community/eza), which is
itself a maintained fork of [**exa**](https://github.com/ogham/exa) by Benjamin
Sago. The overwhelming majority of this codebase was written by Benjamin Sago,
Christina Sørensen, and the eza contributors, and `lez` continues to port work
from eza's open pull requests and issues.

- **exa** — original implementation, by Benjamin Sago ([ogham/exa](https://github.com/ogham/exa)), MIT
- **eza** — community fork, maintained by Christina Sørensen ([eza-community/eza](https://github.com/eza-community/eza)), EUPL-1.2
- **lez** — this fork, by [fxrdhan](https://github.com/fxrdhan), EUPL-1.2

`lez` is licensed under the [EUPL-1.2](LICENSE.txt), inherited from eza; files
originating in exa remain under the MIT license, as recorded in their SPDX
headers.

## The name

This project was called `lsr` through v0.24.1, until it turned out
[rockorager/lsr](https://github.com/rockorager/lsr) and the
[`lsr` crate](https://crates.io/crates/lsr) already held that name.
