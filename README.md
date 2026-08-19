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
- 🎨 **Rich Visuals:** Syntax highlighting, Nerd Font icons, and automatic color scaling.
- 🌿 **Git Integration:** View file and repo status (`M`odified, `U`ntracked, `I`gnored, etc.) directly in the file listing.
- 🌲 **Built-in Tree View:** Hierarchical directory tree out of the box (`lsr --tree`).
- 🚀 **Extended Capabilities:** Rich file classification, custom themes, and archive inspection.

---

---

<a id="try-it">
<h1>Try it!</h1>
</a>

### Nix ❄️

If you already have Nix setup with flake support, you can try out eza with the `nix run` command:

    nix run github:eza-community/eza

Nix will build eza and run it.

If you want to pass arguments this way, use e.g. `nix run github:eza-community/eza -- -ol`.

# Installation

eza is available for Windows, macOS and Linux. Platform and distribution
specific installation instructions can be found in [INSTALL.md](INSTALL.md).

[![Packaging status](https://repology.org/badge/vertical-allrepos/eza.svg?columns=3)](https://repology.org/project/eza/versions)

---

<a id="options">
<h1>Command-line options</h1>
</a>

eza’s options are almost, but not quite, entirely unlike `ls`’s. Quick overview:

## Display options

<details>
<summary>Click to expand</summary>

- **-1**, **--oneline**: display one entry per line
- **-G**, **--grid**: display entries as a grid (default)
- **-l**, **--long**: display extended details and attributes
- **-R**, **--recurse**: recurse into directories
- **-T**, **--tree**: recurse into directories as a tree
- **-x**, **--across**: sort the grid across, rather than downwards
- **-F**, **--classify=(when)**: display type indicator by file names (always, auto, never)
- **--colo[u]r=(when)**: when to use terminal colours (always, auto, never)
- **--colo[u]r-scale=(field)**: highlight levels of `field` distinctly(all, age, size)
- **--color-scale-mode=(mode)**: use gradient or fixed colors in --color-scale. valid options are `fixed` or `gradient`
- **--icons=(when)**: when to display icons (always, auto, never)
- **--hyperlink=(when)**: when to display entries as hyperlinks (always, auto, never)
- **--absolute=(mode)**: display entries with their absolute path (on, follow, off)
- **-w**, **--width=(columns)**: set screen width in columns

</details>

## Filtering options

<details>
<summary>Click to expand</summary>

- **-a**, **--all**: show hidden and 'dot' files
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
- **-I**, **--ignore-glob=(globs)**: glob patterns (pipe-separated) of files to ignore

Pass the `--all` option twice to also show the `.` and `..` directories.

</details>

## Long view options

<details>
<summary>Click to expand</summary>

These options are available when running with `--long` (`-l`):

- **-b**, **--binary**: list file sizes with binary prefixes
- **-B**, **--bytes**: list file sizes in bytes, without any prefixes
- **-g**, **--group**: list each file’s group
- **--smart-group**: only show group if it has a different name from owner
- **-h**, **--header**: add a header row to each column
- **-H**, **--links**: list each file’s number of hard links
- **-i**, **--inode**: list each file’s inode number
- **-m**, **--modified**: use the modified timestamp field
- **-M**, **--mounts**: Show mount details (Linux and MacOS only).
- **-S**, **--blocksize**: show size of allocated file system blocks
- **-t**, **--time=(field)**: which timestamp field to use
- **-u**, **--accessed**: use the accessed timestamp field
- **-U**, **--created**: use the created timestamp field
- **-X**, **--dereference**: dereference symlinks for file information
- **-Z**, **--context**: list each file’s security context
- **-@**, **--extended**: list each file’s extended attributes and sizes
- **--changed**: use the changed timestamp field
- **--git**: list each file’s Git status, if tracked or ignored
- **--git-repos**: list each directory’s Git status, if tracked
- **--git-repos-no-status**: list whether a directory is a Git repository, but not its status (faster)
- **--no-git**: suppress Git status (always overrides `--git`, `--git-repos`, `--git-repos-no-status`)
- **--time-style**: how to format timestamps. valid timestamp styles are ‘`default`’, ‘`iso`’, ‘`long-iso`’, ‘`full-iso`’, ‘`relative`’, or a custom style ‘`+<FORMAT>`’ (E.g., ‘`+%Y-%m-%d %H:%M`’ => ‘`2023-09-30 13:00`’. For more specifications on the format string, see the _`eza(1)` manual page_ and [chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).).
- **--total-size**: show recursive directory size
- **--no-permissions**: suppress the permissions field
- **-o**, **--octal-permissions**: list each file's permission in octal format
- **--no-filesize**: suppress the filesize field
- **--no-user**: suppress the user field
- **--no-time**: suppress the time field
- **--stdin**: read file names from stdin

Some of the options accept parameters:

- Valid **--colo\[u\]r** options are **always**, **automatic** (or **auto** for short), and **never**.
- Valid sort fields are **accessed**, **changed**, **created**, **extension**, **Extension**, **inode**, **modified**, **name**, **Name**, **size**, **type**, and **none**. Fields starting with a capital letter sort uppercase before lowercase. The modified field has the aliases **date**, **time**, and **newest**, while its reverse has the aliases **age** and **oldest**.
- Valid time fields are **modified**, **changed**, **accessed**, and **created**.
- Valid time styles are **default**, **iso**, **long-iso**, **full-iso**, and **relative**.



See the `man` pages for further documentation of usage. They are available
- online [in the repo](https://github.com/eza-community/eza/tree/main/man)
- in your terminal via `man eza`, as of version [`[0.18.13] - 2024-04-25`](https://github.com/eza-community/eza/blob/main/CHANGELOG.md#01813---2024-04-25)
</details>


## Custom Themes
<details>
<summary>Click to expand</summary>

**Eza** has recently added support for a `theme.yml` file, where you can specify all of the existing theme-ing options
available for the `LS_COLORS` and `EXA_COLORS` environment variables, as well as the option to specify different icons
for different file types and extensions. Any existing environment variables set will continue to work and will take
precedence for backwards compatibility.

#### **New** Pre-made themes
Check out the themes available in the official [eza-themes](https://github.com/eza-community/eza-themes) repository, or contribute your own.

An example theme file is available in `docs/theme.yml`, and needs to either be placed in a directory specified by the 
environment variable `EZA_CONFIG_DIR`, or will looked for by default in `$XDG_CONFIG_HOME/eza`.

Full details are available on the [man page](https://github.com/eza-community/eza/tree/main/man/eza_colors-explanation.5.md) and an example theme file is included [here](https://github.com/eza-community/eza/tree/main/docs/theme.yml)

</details>


# Hacking on eza

If you wanna contribute to eza, firstly, you're expected to follow our 
[code of conduct](https://github.com/eza-community/eza/blob/main/CODE_OF_CONDUCT.md). 
After having understood the code of conduct, you can have a look at our
[CONTRIBUTING.md](https://github.com/eza-community/eza/blob/main/CONTRIBUTING.md) 
for more info about actual hacking.

[![Star History Chart](https://api.star-history.com/svg?repos=eza-community/eza&type=Date)](https://star-history.com/#eza-community/eza&Date)
