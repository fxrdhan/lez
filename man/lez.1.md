% lez(1) $version

<!-- This is the lez(1) man page, written in Markdown. -->
<!-- To generate the roff version, run `just man`, -->
<!-- and the man page will appear in the ‘target’ directory. -->


NAME
====

lez — a modern, fast, and feature-rich replacement for ls written in Rust


SYNOPSIS
========

`lez [options] [files...]`

**lez** is a fast, modern replacement for `ls` written in Rust.
It uses colours for information by default, helping you distinguish between many types of files, such as whether you are the owner, or in the owning group.

It also has extra features not present in the original `ls`, such as viewing the Git status for a directory, or recursing into directories with a tree view, Lines of Code statistics (`--code`), and disk block allocation sorting (`--sort=blocks`).


EXAMPLES
========

`lez`
: Lists the contents of the current directory in a grid.

`lez --oneline --reverse --sort=size`
: Displays a list of files with the largest at the top.

`lez --long --header --inode --git`
: Displays a table of files with a header, showing each file’s metadata, inode, and Git status.

`lez --long --tree --level=3`
: Displays a tree of files, three levels deep, as well as each file’s metadata.

`lez --code`
: Displays a summary of lines of code by language across the tree.


META OPTIONS
===============

`--help`, `-?`
: Show list of command-line options with syntax-highlighted ANSI colors.

`--version`
: Show version of lez.

`-v`
: Sort numerically within names, the way `ls -v` does. This is already the
  default ordering, since sorting by name runs the collator with numeric
  awareness on; the flag exists so the reflex works. Up to and including
  v0.26.1, `-v` was an alias for `--version`.

`--config=PATH`
: Load default options from the specified configuration file (`.toml`, `.yaml`, or `.yml`).

`--no-config`
: Do not load any global or per-directory configuration files.


DISPLAY OPTIONS
===============

`-1`, `--oneline`
: Display one entry per line.

Symbolic link targets are not displayed in this mode, so output stays clean when piped into other commands such as `xargs`; use a details view (`-l`) to see them.

`--absolute[=WHEN]`
: Display entries with their absolute path.

Valid settings are '`on`', '`follow`', and '`off`'.
When used without a value, defaults to '`on`'. Note: when providing an explicit value, an equals sign is required (`--absolute=WHEN`).

'`on`': Show absolute paths for all entries.
'`follow`': Show absolute paths and resolve symbolic links to their targets.
'`off`': Show relative paths (default behavior).

`-F`, `--classify[=WHEN]`
: Display file kind indicators next to file names.

Valid settings are ‘`always`’, ‘`automatic`’ (or ‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’. Note: when providing an explicit value, an equals sign is required (`--classify=WHEN`).

`automatic` or `auto` will display file kind indicators only when the standard output is connected to a real terminal. If `lez` is run while in a `tty`, or the output of `lez` is either redirected to a file or piped into another program, file kind indicators will not be used. Setting this option to ‘`always`’ causes `lez` to always display file kind indicators, while ‘`never`’ disables the use of file kind indicators.

`-G`, `--grid`
: Display entries as a grid (default).

`-l`, `--long`
: Display extended file metadata as a table.

`-R`, `--recurse`
: Recurse into directories.

`-T`, `--tree`
: Recurse into directories as a tree.

`--code[=MODE]`
: Print a lines-of-code summary by language instead of listing files, in the spirit of tools like `tokei` and `cloc`.

: The given paths (or the current directory) are walked recursively, honouring a git repository’s `.gitignore` when one is present, and each recognised language (including Odin, Rust, C/C++, Python, Go, and 100+ others) is reported with its file, line, code, comment, and blank counts, plus a bar visualising its share of the code. Valid modes are ‘`lines`’, ‘`percent`’, and ‘`both`’ (the default).

`--json`
: Output file listing and metadata as structured JSON for easy parsing and scripting.

`--follow-symlinks`
: Drill down into symbolic links that point to directories.

`-X`, `--dereference`
: Dereference symbolic links when displaying information and sorting (e.g. resolving target file size with `--sort=size`).

`-x`, `--across`
: Sort the grid across, rather than downwards.

`--color=WHEN`, `--colour=WHEN`
: When to use terminal colours.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’. Note: when providing an explicit value, an equals sign is required (`--color=WHEN`).

`--color-scale[=FIELD]`, `--colour-scale[=FIELD]`
: Highlight levels of ‘`field`’ distinctly.

Valid fields are ‘`all`’, ‘`age`’, and ‘`size`’.
When used without a value, defaults to ‘`all`’. Note: when providing an explicit value, an equals sign is required (`--color-scale=FIELD`).

`--color-scale-mode=MODE`, `--colour-scale-mode=MODE`
: Mode of color scale.

Valid options are `fixed` to use a fixed color (disabling color scale), or `gradient` to use an automatic darker (old/small file) to lighter (recent/big file) gradient of colors.
When used without a value, defaults to `gradient`.

The size gradient runs over orders of magnitude rather than bytes, so a single large file does not flatten every ordinary one to the same shade. The age gradient runs over elapsed time directly.

`--icons[=WHEN]`
: Display icons next to file names.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’. Note: when providing an explicit value, an equals sign is required (`--icons=WHEN`).

`automatic` or `auto` will display icons only when the standard output is connected to a real terminal. If `lez` is run while in a `tty`, or the output of `lez` is either redirected to a file or piped into another program, icons will not be used. Setting this option to ‘`always`’ causes `lez` to always display icons, while ‘`never`’ disables the use of icons.

`--quotes=WHEN`
: When to quote file names. The default, `auto`, quotes names that contain spaces or quotes; `always` quotes every name; `never` quotes nothing (like `ls -N`).

A quoted name is written so a shell reads back the name on disk. Single quotes are used by default, double quotes for a name holding an apostrophe, and for a name holding both kinds the single quotes are broken out of for each apostrophe, as `ls` does: `julia's "file".txt` prints as `'julia'\''s "file".txt'`.

`--spacing=SPACES`
: Number of spaces to print between columns in the grid views. Accepts `0` to `255`; the default is `2`.

`--short-nix`
: Abbreviate Nix store hashes in file names and paths.

: A path component beginning with a Nix store hash — exactly 32 characters of Nix’s base32 alphabet followed by a dash, like `vlkia5wk0svsikwv50554mh06iayg2m2-source.drv` — is displayed with the hash shortened to its first 8 characters and an ellipsis, painted dim so the name stands out: `vlkia5wk…-source.drv`. This applies to listed names, symbolic link targets, and absolute paths.

`--no-symlink-targets`
: Do not show symlink targets (the `-> ...`) in long details and lines view modes.

`--summary`
: Display total summary statistics of entries (directories count, files count, symlinks count, and total count).

`--hyperlink[=WHEN]`
: Display entries as hyperlinks.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’. Note: when providing an explicit value, an equals sign is required (`--hyperlink=WHEN`).

`--mime-types`
: Determine file MIME types to better inform styling decisions and icon selection (Unix only). Can also be enabled via the `LEZ_MIME_TYPES` or `EZA_MIME_TYPES` environment variable.

`-w`, `--width=COLS`
: Set screen width in columns (clamped to the safe range `1..65535` to prevent integer overflow and division-by-zero).


FILTERING AND SORTING OPTIONS
=============================

`-a`, `--all`
: Show hidden and “dot” files.
Use this twice to also show the ‘`.`’ and ‘`..`’ directories.

`-A`, `--almost-all`
: Equivalent to --all; included for compatibility with `ls -A`.

`--show-dotfiles`
: Show dot-prefixed files without showing other hidden files.

`-d`, `--treat-dirs-as-files`
: This flag, inherited from `ls`, changes how `lez` handles directory arguments.

: Instead of recursing into directories and listing their contents (the default behavior), it treats directories as regular files and lists information about the directory entry itself.

: This is useful when you want to see metadata about the directory (e.g., permissions, size, modification time) rather than its contents.

: For simply listing only directories and not files, consider using the `--only-dirs` (`-D`) option as an alternative.

`-L`, `--level=DEPTH`
: Limit the depth of recursion.

`-r`, `--reverse`
: Reverse the sort order.

`-s`, `--sort=SORT_FIELD`
: Which field to sort by.

Valid sort fields are ‘`name`’, ‘`Name`’, ‘`lexicographic`’, ‘`Lexicographic`’, ‘`extension`’, ‘`Extension`’, ‘`path`’, ‘`Path`’, ‘`size`’, ‘`block`’ [Unix only], ‘`modified`’, ‘`changed`’, ‘`accessed`’, ‘`created`’, ‘`inode`’, ‘`type`’, and ‘`none`’.

The ‘`block`’ sort field has the aliases ‘`blocks`’ and ‘`blocksize`’.

The ‘`lexicographic`’ and ‘`Lexicographic`’ sort fields have the aliases ‘`lex`’ (‘`Lex`’) and ‘`lg`’ (‘`Lg`’). They compare names one code point at a time, without the natural ordering of digit runs that every other name field applies and without locale collation, so ‘`--sort=Lexicographic`’ gives the same order as ‘`ls`’ under the C locale. Use them for names that only look numeric, such as hexadecimal identifiers, where treating digit runs as numbers scatters related files.

The ‘`path`’ and ‘`Path`’ sort fields have the aliases ‘`relative-path`’ (‘`Relative-path`’, ‘`Relative-Path`’), ‘`relpath`’ (‘`Relpath`’), and ‘`relative_path`’ (‘`Relative_path`’).

The `modified` sort field has the aliases ‘`date`’, ‘`time`’, ‘`mod`’, ‘`old`’, and ‘`oldest`’, and its reverse order has the aliases ‘`age`’, ‘`new`’, and ‘`newest`’.

Sort fields starting with a capital letter will sort uppercase before lowercase: ‘A’ then ‘B’ then ‘a’ then ‘b’. Fields starting with a lowercase letter will mix them: ‘A’ then ‘a’ then ‘B’ then ‘b’.

`-t`
: Sort by modification time, newest first (GNU `ls` compatibility; shorthand for `--sort=age`). When passed with a field argument (e.g. `-t modified`), it selects the timestamp field to display instead.

`-I`, `--ignore-glob=GLOBS`
: Glob patterns, pipe-separated, of files to ignore.

`--ignore-glob-ci=GLOBS`
: Glob patterns, pipe-separated, of files to ignore, matched case-insensitively.

`--git-ignore` [if lez was built with git support]
: Do not list files that are ignored by Git.

`--cachedir-ignore`
: Do not list directories that contain a `CACHEDIR.TAG` file carrying the correct signature (see <https://bford.info/cachedir/>).

`--ignore-submodule-contents` [if built with git support]
: Do not list the contents of Git submodules.

`-W`, `--warn-hidden`
: After the listing, print a tally of hidden and Git-ignored entries. Give the option twice (`-WW`) to always print the tally, even when nothing was filtered.

`--since=DURATION`
: Filter and display only files created or modified within the specified duration window (e.g. `10m`, `1h`, `2d`, `1w`).

`--group-directories-first`
: List directories before other files.

`--group-directories-last`
: List directories after other files.

`-D`, `--only-dirs`
: List only directories, not files.

`-f`, `--only-files`
: List only files, not directories.

`--show-symlinks`
: Explicitly show symbolic links (when used with `--only-files` | `--only-dirs`).

`--no-symlinks`
: Do not show symbolic links.

`--show-dotfiles`
: Explicitly show dotfiles even if hidden.


LONG VIEW OPTIONS
=================

These options are available when running with `--long` (`-l`):

`-b`, `--binary`
: List file sizes with binary prefixes. Overrides preceding `-B`/`--bytes` flags.

`-B`, `--bytes`
: List file sizes in bytes, without any prefixes. Overrides preceding `-b`/`--binary` flags.

`--size-digits=(NUM)`, `--digits=(NUM)`
: Number of digits to display for file sizes (1..=8, default: 3). Can also be set via the `LEZ_SIZE_DIGITS` environment variable.

`--changed`
: Use the changed timestamp field.

`-g`, `--group`
: List each file’s group.

`--smart-group`
: Only show group if it has a different name from owner. Automatically enables the group column in long view.

`-h`, `--header`
: Add a header row to each column.

`-H`, `--links`
: List each file’s number of hard links.

`-i`, `--inode`
: List each file’s inode number.

`--loc[=MODE]`
: Add a language column and a lines-of-code column to the long view.

: Only regular files in a recognised programming language (including Janet, Odin, Rust, C/C++, Python, Go, and 100+ others) are counted; counting is comment-aware, so the code column excludes comment and blank lines.

: Valid modes are ‘`lines`’ (the count of code lines), ‘`percent`’ (each file’s share of the code in the whole tree), and ‘`both`’ (the default). In `percent` and `both` modes the denominator is the total code across the recursed tree, or the git repository if one is present.

`-m`, `--modified`
: Use the modified timestamp field.

`-M`, `--mounts`
: Show mount details (Linux and macOS only).

`-n`, `--numeric`
: List numeric user and group IDs.

`-O`, `--flags`
: List file flags on Linux, macOS, and BSD systems, and file attributes on Windows systems. On Linux systems, lists inode flags/attributes (`FS_IOC_GETFLAGS`, equivalent to `lsattr`). On BSD systems see chflags(1) for a list of file flags and their meanings. By default, attributes are displayed in a long form. To display attributes as single-character abbreviations, set the environment variable `LEZ_FLAGS_FORMAT=short` (or `LEZ_WINDOWS_ATTRIBUTES=short`).

`-S`, `--blocksize`
: List the allocated size of each file, in bytes.

`--blocks`
: List the allocated size of each file, in blocks.

`-t`, `--time=WORD`
: Which timestamp field to list.

: Valid timestamp fields are ‘`modified`’ (aliases: ‘`mod`’, ‘`m`’), ‘`changed`’ (alias: ‘`ch`’), ‘`accessed`’ (alias: ‘`acc`’), and ‘`created`’ (alias: ‘`cr`’).

`--time-style=STYLE`
: How to format timestamps.

: Valid timestamp styles are ‘`default`’, ‘`iso`’, ‘`long-iso`’, ‘`full-iso`’, ‘`relative`’, or a custom style ‘`+<FORMAT>`’ (e.g., ‘`+%Y-%m-%d %H:%M`’ => ‘`2023-09-30 13:00`’).

`<FORMAT>` should be a chrono format string. For details on the chrono format syntax, please read: https://docs.rs/chrono/latest/chrono/format/strftime/index.html .

Alternatively, `<FORMAT>` can be a two line string, the first line will be used for non-recent files and the second for recent files. E.g., if `<FORMAT>` is "`%Y-%m-%d %H<newline>--%m-%d %H:%M`", non-recent files => "`2022-12-30 13`", recent files => "`--09-30 13:34`".

`--total-size`
: Show recursive directory size (unix only).

`-u`, `--accessed`
: Use the accessed timestamp field.

`-U`, `--created`
: Use the created timestamp field.

`--utc`
: Show the time in the UTC timezone.

`--no-permissions`
: Suppress the permissions field.

`-o`, `--octal-permissions`
: List each file's permissions in octal format.

`--no-filesize`
: Suppress the file size field.

`--no-user`
: Suppress the user field.

`--no-time`
: Suppress the time field.

`--stdin`
: When you wish to pipe directories to lez/read from stdin. Separate one per line or define custom separation char in `LEZ_STDIN_SEPARATOR` / `EZA_STDIN_SEPARATOR` env variable.

`--print-total`
: Print the total number of files and directories listed at the bottom of the output.

`-@`, `--extended`
: List each file’s extended attributes and sizes.

`--no-extended`
: Don’t show the `@` marker that a file has extended attributes.

`-e`, `--tags`
: List each file’s colour tags, read from the extended attributes that macOS Finder writes. Tagged names are painted with the tag’s colour.

`--inspect-archives` [if built with inspect-archives support]
: In the long view, list the entries of supported archives (currently uncompressed `.tar`) below the archive itself. Detection is extension-based; corrupt archives are listed like regular files. Each entry's own file name is coloured by type as a normal listing would colour it, while the archive path and the entry size stay in the punctuation style; names the theme has no rule for stay punctuation too.

`-Z`, `--context`
: List each file's security context.

`--git` [if lez was built with git support]
: List each file’s Git status, if tracked.
This adds a two-character column indicating the staged and unstaged statuses respectively. The status character can be ‘`-`’ for not modified, ‘`M`’ for a modified file, ‘`N`’ for a new file, ‘`D`’ for deleted, ‘`R`’ for renamed, ‘`T`’ for type-change, ‘`I`’ for ignored, and ‘`U`’ for conflicted. Directories will be shown to have the status of their contents, which is how ‘deleted’ is possible if a directory contains a file that has a certain status, it will be shown to have that status.

`--git-glyphs` [if lez was built with git support]
: Display Git status with Nerd Font glyphs / icons instead of standard ASCII characters.

`--git-repos` [if lez was built with git support]
: List each directory’s Git status, if tracked.
Symbols shown are `|`= clean, `+`= dirty, and `~`= for unknown.

`--git-repos-no-status` [if lez was built with git support]
: List if a directory is a Git repository, but not its status.
All Git repository directories will be shown as (themed) `-` without status indicated.

`--no-git`
: Don't show Git status (always overrides `--git`, `--git-repos`, `--git-repos-no-status`).


ENVIRONMENT VARIABLES
=====================

If an environment variable prefixed with `LEZ_` is not set, for backward compatibility, it will default to its counterpart starting with `EZA_` or `EXA_`.

lez responds to the following environment variables:

## `COLUMNS`

Overrides the width of the terminal, in characters, however, `-w` takes precedence.

For example, ‘`COLUMNS=80 lez`’ will show a grid view with a maximum width of 80 characters.

This option won’t do anything when lez’s output doesn’t wrap, such as when using the `--long` view.

## `LEZ_STRICT`, `EZA_STRICT`

Enables _strict mode_, which will make lez error when two command-line options are incompatible.

Usually, options can override each other going right-to-left on the command line, so that lez can be given aliases: creating an alias ‘`lez=lez --sort=ext`’ then running ‘`lez --sort=size`’ with that alias will run ‘`lez --sort=ext --sort=size`’, and the sorting specified by the user will override the sorting specified by the alias.

In strict mode, the two options will not co-operate, and lez will error.

This option is intended for use with automated scripts and other situations where you want to be certain you’re typing in the right command.

## `LEZ_GRID_ROWS`, `EZA_GRID_ROWS`

Limits the grid-details view (‘`lez --grid --long`’) so it’s only activated when at least the given number of rows of output would be generated.

With widescreen displays, it’s possible for the grid to look very wide and sparse, on just one or two lines with none of the columns lining up.
By specifying a minimum number of rows, you can only use the view if it’s going to be worth using.

## `LEZ_ICON_SPACING`, `EZA_ICON_SPACING`

Specifies the number of spaces to print between an icon (see the ‘`--icons`’ option) and its file name.

Different terminals display icons differently, as they usually take up more than one character width on screen, so there’s no “standard” number of spaces that lez can use to separate an icon from text. One space may place the icon too close to the text, and two spaces may place it too far away. So the choice is left up to the user to configure depending on their terminal emulator.

## `LEZ_NO_EMPTY_DIR_ICON`, `EZA_NO_EMPTY_DIR_ICON`

Set to any value to give every directory the same icon, instead of a different one when it is empty.

Telling the two apart means asking the filesystem about each directory listed: its link count, and — when that does not settle the question — a read of its contents. On a local disk this is not worth thinking about. On a FUSE mount or a network share every one of those is a round trip, and a directory of a few thousand subdirectories can take long enough that lez looks like it has hung. This is the way to stop paying for a distinction you may not want.

## `LEZ_SIZE_DIGITS`, `EZA_SIZE_DIGITS`

Specifies the default number of digits (from 1 to 8) to display for formatted file sizes (default: `3`).

For example, setting `LEZ_SIZE_DIGITS=4` causes sizes like `2.3Gi` to be formatted with higher precision as `2.34Gi`.

## `NO_COLOR`

Disables colours in the output (regardless of its value). Can be overridden by `--color` option.

See `https://no-color.org/` for details.

## `LS_COLORS`, `LEZ_COLORS`, `EZA_COLORS`

Specifies the colour scheme used to highlight files based on their name and kind, as well as highlighting metadata and parts of the UI.

For more information on the format of these environment variables, see the **lez_colors**(5) manual page.

## `LEZ_OVERRIDE_GIT`, `EZA_OVERRIDE_GIT`

Overrides any `--git` or `--git-repos` argument.

## `LEZ_MIN_LUMINANCE`, `EZA_MIN_LUMINANCE`

Specifies the minimum luminance to use when color-scale is active. Its value can be between -100 to 100.

## `LEZ_MAX_LUMINANCE`, `EZA_MAX_LUMINANCE`

Specifies the maximum luminance to use when color-scale is active. Its value can be between -100 to 100.

## `LEZ_ICONS_AUTO`, `EZA_ICONS_AUTO`

If set, automates the same behavior as using `--icons` or `--icons=auto`. Useful for if you always want to have icons enabled.

Any explicit use of the `--icons=WHEN` flag overrides this behavior.

## `LEZ_STDIN_SEPARATOR`, `EZA_STDIN_SEPARATOR`

Specifies the separator to use when file names are piped from stdin. Defaults to newline.

## `LEZ_CONFIG_FILE`, `EZA_CONFIG_FILE`

Explicitly specifies the path to a configuration file to load (`.toml`, `.yaml`, or `.yml`). Overrides standard discovery.

## `LEZ_CONFIG_DIR`, `EZA_CONFIG_DIR`

Specifies the directory where lez will look for its configuration and theme files. Defaults to `$XDG_CONFIG_HOME/lez`, `$XDG_CONFIG_HOME/eza`, `$HOME/.config/lez`, or `$HOME/.config/eza` if `XDG_CONFIG_HOME` is not set.

## `LEZ_QUOTING_STYLE`, `EZA_QUOTING_STYLE`

Specifies when file names are quoted, as if `--quotes` had been given. Valid values are `always`, `auto`, and `never`; invalid or unset values fall back to `auto`. `--quotes=never` is equivalent to `ls -N`, and the command-line option overrides this variable.


CONFIGURATION FILES
===================

`lez` supports both global and per-directory configuration files written in TOML or YAML format.

## Discovery and Precedence

Configuration is evaluated in the following precedence order:
1. Command-line arguments
2. Environment variables (`LEZ_*`, `EZA_*`, `LS_COLORS`, etc.)
3. Local (per-directory) configuration file (`.lez.toml`, `.lez.yaml`, `.lez.yml`, `.eza.toml`, `.eza.yaml` in the current working directory)
4. Global configuration file (`$LEZ_CONFIG_FILE`, or `config.toml` / `lez.toml` in `$LEZ_CONFIG_DIR` or `~/.config/lez/`)
5. Built-in defaults

Passing `--no-config` disables loading both global and per-directory configuration files.

For full schema and example configuration options, refer to `docs/config.example.toml`.


EXIT STATUSES
=============

0
: If everything goes OK.

1
: If there was an I/O error during operation.

3
: If there was a problem with the command-line arguments.

13
: If permission is denied to access a path.


AUTHOR
======

lez is maintained by fxrdhan <https://github.com/fxrdhan>.

**Source code:** `https://github.com/fxrdhan/lez` \
**Contributors:** `https://github.com/fxrdhan/lez/graphs/contributors`

Lineage: `exa` (by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).


SEE ALSO
========

**lez_colors**(5), **lez_colors-explanation**(5)
