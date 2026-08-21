<!--
SPDX-FileCopyrightText: 2023-2024 Christina Sørensen
SPDX-FileContributor: Christina Sørensen

SPDX-License-Identifier: EUPL-1.2
-->

<div align="center">
    
# ⚡ lsr

**A modern, fast, and feature-rich replacement for `ls` written in Rust.**

[![License](https://img.shields.io/badge/License-EUPL--1.2-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/Rust-1.90%2B-orange.svg)](https://www.rust-lang.org/)

</div>

![lsr demo](docs/images/screenshots.png)

---

**`lsr`** is a fast, modern file-listing command-line tool with smart defaults, enhanced file icons, Git integration, and continuous performance improvements.

- ⚡ **Fast & Lightweight:** Written in modern Rust with multithreaded directory scanning via Rayon.
- 🎨 **Rich Visuals:** Syntax highlighting, colored CLI help output, Nerd Font icons, and automatic color scaling.
- 🌿 **Git Integration:** View file and repo status (`M`odified, `U`ntracked, `I`gnored, etc.) directly in the file listing.
- 🌲 **Built-in Tree View:** Hierarchical directory tree out of the box (`lsr --tree`).
- 🚀 **Extended Capabilities:** Rich file classification, custom themes, and archive inspection.

---

---

<a id="try-it">
<h1>Try it!</h1>
</a>

### Cargo / Build from Source 🦀

Install `lsr` directly with Cargo:

```bash
cargo install --git https://github.com/fxrdhan/lsr.git
```

Or build from source:

```bash
git clone https://github.com/fxrdhan/lsr.git
cd lsr
cargo build --release
# Binary available at target/release/lsr
```

### Nix ❄️

If you already have Nix setup with flake support, you can try out `lsr` with the `nix run` command:

```bash
nix run github:fxrdhan/lsr
```

Nix will build `lsr` and run it.

If you want to pass arguments this way, use e.g. `nix run github:fxrdhan/lsr -- -la --icons`.

---

# Installation

`lsr` is available for macOS, Linux, and Windows. Detailed platform-specific installation instructions can be found in [INSTALL.md](INSTALL.md).

---

<a id="options">
<h1>Command-line options</h1>
</a>

`lsr`’s options are intuitive and familiar. Quick overview:

## Display options

<details>
<summary>Click to expand</summary>

- **-1**, **--oneline**: display one entry per line
- **-G**, **--grid**: display entries as a grid (default)
- **-l**, **--long**: display extended details and attributes
- **-R**, **--recurse**: recurse into directories
- **-T**, **--tree**: recurse into directories as a tree
- **--code[=MODE]**: print lines-of-code summary by language (Janet, Odin, Rust, C/C++, Python, Go, etc.)
- **--json**: output file listing and metadata as structured JSON
- **-x**, **--across**: sort the grid across, rather than downwards
- **-F**, **--classify[=(when)]**: display type indicator by file names (always, auto, never)
- **--colo[u]r=(when)**: when to use terminal colours (always, auto, never)
- **--colo[u]r-scale=(field)**: highlight levels of `field` distinctly (all, age, size)
- **--color-scale-mode=(mode)**: use gradient or fixed colors in --color-scale. valid options are `fixed` or `gradient`
- **--icons[=(when)]**: when to display icons (always, auto, never; requires '=' if value provided)
- **--no-symlink-targets**: do not show symlink targets (the `-> ...`)
- **--summary**: display total summary statistics of entries (directories, files, symlinks, and total)
- **--hyperlink[=(when)]**: when to display entries as hyperlinks (always, auto, never; requires '=' if value provided)
- **--absolute=(mode)**: display entries with their absolute path (on, follow, off)
- **--print-total**: print the total number of files and directories listed
- **-w**, **--width=(columns)**: set screen width in columns (clamped to `1..65535`)

</details>

## Filtering options

<details>
<summary>Click to expand</summary>

- **-a**, **--all**: show hidden and 'dot' files
- **--show-dotfiles**: show dot-prefixed files without showing other hidden files
- **-d**, **--treat-dirs-as-files**: list directories like regular files
- **-L**, **--level=(depth)**: limit the depth of recursion
- **-r**, **--reverse**: reverse the sort order
- **-s**, **--sort=(field)**: which field to sort by
- **--group-directories-first**: list directories before other files
- **--group-directories-last**: list directories after other files
- **-D**, **--only-dirs**: list only directories
- **-f**, **--only-files**: list only files
- **--no-symlinks**: don't show symbolic links
- **--show-symlinks**: explicitly show links (with `--only-dirs`, `--only-files`, to show symlinks that match the filter)
- **--git-ignore**: ignore files mentioned in `.gitignore`
- **--since=(duration)**: filter and display only files created or modified within the specified duration window (e.g. 10m, 1h, 2d, 1w)
- **-I**, **--ignore-glob=(globs)**: glob patterns (pipe-separated) of files to ignore
- **--ignore-glob-case-insensitive**: match ignore globs case-insensitively

Pass the `--all` option twice to also show the `.` and `..` directories.

</details>

## Long view options

<details>
<summary>Click to expand</summary>

These options are available when running with `--long` (`-l`):

- **-b**, **--binary**: list file sizes with binary prefixes (overrides `--bytes` if passed after)
- **-B**, **--bytes**: list file sizes in bytes, without any prefixes (overrides `--binary` if passed after)
- **-g**, **--group**: list each file’s group
- **--smart-group**: only show group if it has a different name from owner (automatically enables group column)
- **-h**, **--header**: add a header row to each column
- **-H**, **--links**: list each file’s number of hard links
- **-i**, **--inode**: list each file’s inode number
- **--loc[=MODE]**: display language and lines-of-code columns (Janet, Odin, Rust, C/C++, Python, Go, etc.)
- **-m**, **--modified**: use the modified timestamp field
- **-M**, **--mounts**: Show mount details (Linux and MacOS only).
- **-S**, **--blocks**, **--blocksize**: show size of allocated file system blocks
- **-t**, **--time=(field)**: which timestamp field to use (modified [aliases: mod, m, r], accessed [acc], changed [ch], created [cr])
- **-u**, **--accessed**: use the accessed timestamp field
- **-U**, **--created**: use the created timestamp field
- **-X**, **--dereference**: dereference symlinks for file information and sorting
- **-Z**, **--context**: list each file’s security context
- **-@**, **--extended**: list each file’s extended attributes and sizes
- **--changed**: use the changed timestamp field
- **--git**: list each file’s Git status, if tracked or ignored
- **--git-repos**: list each directory’s Git status, if tracked
- **--git-repos-no-status**: list whether a directory is a Git repository, but not its status (faster)
- **--no-git**: suppress Git status (always overrides `--git`, `--git-repos`, `--git-repos-no-status`)
- **--time-style**: how to format timestamps. valid timestamp styles are ‘`default`’, ‘`iso`’, ‘`long-iso`’, ‘`full-iso`’, ‘`relative`’, or a custom style ‘`+<FORMAT>`’ (E.g., ‘`+%Y-%m-%d %H:%M`’ => ‘`2023-09-30 13:00`’. For more specifications on the format string, see the _`lsr(1)` manual page_ and [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)).
- **--total-size**: show recursive directory size
- **--no-permissions**: suppress the permissions field
- **-o**, **--octal-permissions**: list each file's permission in octal format
- **--no-filesize**: suppress the filesize field
- **--no-user**: suppress the user field
- **--no-time**: suppress the time field
- **--stdin**: read file names from stdin

Some of the options accept parameters:

- Valid **--colo\[u\]r** options are **always**, **automatic** (or **auto** for short), and **never**.
- Valid sort fields are **accessed**, **changed**, **created**, **extension**, **Extension**, **inode**, **modified**, **name**, **Name**, **path**, **Path**, **size**, **block**, **type**, and **none**. Fields starting with a capital letter sort uppercase before lowercase. The modified field has the aliases **date**, **time**, **mod**, **old**, and **oldest**, while its reverse has the aliases **age**, **new**, and **newest**. The **block** field has the aliases **blocks** and **blocksize**.
- Valid time fields are **modified**, **changed**, **accessed**, and **created**.
- Valid time styles are **default**, **iso**, **long-iso**, **full-iso**, **relative**, and **relative-recent** (or **recent**).

See the `man` pages for further documentation of usage. They are available:
- online [in the repo](https://github.com/fxrdhan/lsr/tree/main/man)
- in your terminal via `man lsr`
</details>

## Custom Themes
<details>
<summary>Click to expand</summary>

**`lsr`** supports a `theme.yml` file, where you can customize theme options available for the `LS_COLORS`, `EZA_COLORS`, and `LSR_COLORS` environment variables, as well as specify custom icons for different file types and extensions.

An example theme file is available in `docs/theme.yml`, and can be placed in a directory specified by the 
environment variable `LSR_CONFIG_DIR`, `EZA_CONFIG_DIR`, or looked for by default in `$XDG_CONFIG_HOME/lsr` or `$XDG_CONFIG_HOME/eza`.

Full details are available on the [man page](https://github.com/fxrdhan/lsr/tree/main/man/eza_colors-explanation.5.md) and an example theme file is included [here](https://github.com/fxrdhan/lsr/tree/main/docs/theme.yml).

</details>

# Contributing to lsr

If you want to contribute to `lsr`, please check out our:
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

[![Star History Chart](https://api.star-history.com/svg?repos=fxrdhan/lsr&type=Date)](https://star-history.com/#fxrdhan/lsr&Date)
