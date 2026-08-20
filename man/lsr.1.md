% lsr(1) $version

<!-- This is the lsr(1) man page, written in Markdown. -->
<!-- To generate the roff version, run `just man`, -->
<!-- and the man page will appear in the ‘target’ directory. -->


NAME
====

lsr — a modern, fast, and feature-rich replacement for ls written in Rust


SYNOPSIS
========

`lsr [options] [files...]`

**lsr** is a fast, modern replacement for `ls` written in Rust.
It uses colours for information by default, helping you distinguish between many types of files, such as whether you are the owner, or in the owning group.

It also has extra features not present in the original `ls`, such as viewing the Git status for a directory, or recursing into directories with a tree view, Lines of Code statistics (`--code`), and disk block allocation sorting (`--sort=blocks`).


EXAMPLES
========

`lsr`
: Lists the contents of the current directory in a grid.

`lsr --oneline --reverse --sort=size`
: Displays a list of files with the largest at the top.

`lsr --long --header --inode --git`
: Displays a table of files with a header, showing each file’s metadata, inode, and Git status.

`lsr --long --tree --level=3`
: Displays a tree of files, three levels deep, as well as each file’s metadata.

`lsr --code`
: Displays a summary of lines of code by language across the tree.


META OPTIONS
===============

`--help`, `-?`
: Show list of command-line options with syntax-highlighted ANSI colors.

`-v`, `--version`
: Show version of lsr.


DISPLAY OPTIONS
===============

`-1`, `--oneline`
: Display one entry per line.

`--absolute[=WHEN]`
: Display entries with their absolute path.

Valid settings are '`on`', '`follow`', and '`off`'.
When used without a value, defaults to '`on`'.

'`on`': Show absolute paths for all entries.
'`follow`': Show absolute paths and resolve symbolic links to their targets.
'`off`': Show relative paths (default behavior).

`-F`, `--classify[=WHEN]`
: Display file kind indicators next to file names.

Valid settings are ‘`always`’, ‘`automatic`’ (or ‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’.

`automatic` or `auto` will display file kind indicators only when the standard output is connected to a real terminal. If `lsr` is run while in a `tty`, or the output of `lsr` is either redirected to a file or piped into another program, file kind indicators will not be used. Setting this option to ‘`always`’ causes `lsr` to always display file kind indicators, while ‘`never`’ disables the use of file kind indicators.

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

`--follow-symlinks`
: Drill down into symbolic links that point to directories.

`-X`, `--dereference`
: Dereference symbolic links when displaying information and sorting (e.g. resolving target file size with `--sort=size`).

`-x`, `--across`
: Sort the grid across, rather than downwards.

`-F`, `--classify[=WHEN]`
: Display type indicator by file names.

Valid settings are '`always`', '`automatic`' ('`auto`' for short), and '`never`'.
When used without a value, defaults to '`automatic`'.

`--color=WHEN`, `--colour=WHEN`
: When to use terminal colours.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’.

`--color-scale[=FIELD]`, `--colour-scale[=FIELD]`
: Highlight levels of ‘`field`’ distinctly.

Valid fields are ‘`all`’, ‘`age`’, and ‘`size`’.
When used without a value, defaults to ‘`all`’.

`--color-scale-mode=MODE`, `--colour-scale-mode=MODE`
: Mode of color scale.

Valid options are `fixed` or `gradient`.
When used without a value, defaults to `gradient`.

`--icons[=WHEN]`
: Display icons next to file names.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’. Note: when providing an explicit value, an equals sign is required (`--icons=WHEN`).

`automatic` or `auto` will display icons only when the standard output is connected to a real terminal. If `lsr` is run while in a `tty`, or the output of `lsr` is either redirected to a file or piped into another program, icons will not be used. Setting this option to ‘`always`’ causes `lsr` to always display icons, while ‘`never`’ disables the use of icons.

`--no-quotes`
: Don't quote file names with spaces.

`--short-nix`
: Abbreviate Nix store hashes in file names and paths.

: A path component beginning with a Nix store hash — exactly 32 characters of Nix’s base32 alphabet followed by a dash, like `vlkia5wk0svsikwv50554mh06iayg2m2-source.drv` — is displayed with the hash shortened to its first 8 characters and an ellipsis, painted dim so the name stands out: `vlkia5wk…-source.drv`. This applies to listed names, symbolic link targets, and absolute paths.

`--hyperlink[=WHEN]`
: Display entries as hyperlinks.

Valid settings are ‘`always`’, ‘`automatic`’ (‘`auto`’ for short), and ‘`never`’.
When used without a value, defaults to ‘`automatic`’. Note: when providing an explicit value, an equals sign is required (`--hyperlink=WHEN`).

`-w`, `--width=COLS`
: Set screen width in columns (clamped to the safe range `1..65535` to prevent integer overflow and division-by-zero).


FILTERING AND SORTING OPTIONS
=============================

`-a`, `--all`
: Show hidden and “dot” files.
Use this twice to also show the ‘`.`’ and ‘`..`’ directories.

`-A`, `--almost-all`
: Equivalent to --all; included for compatibility with `ls -A`.

`-d`, `--treat-dirs-as-files`
: This flag, inherited from `ls`, changes how `lsr` handles directory arguments.

: Instead of recursing into directories and listing their contents (the default behavior), it treats directories as regular files and lists information about the directory entry itself.

: This is useful when you want to see metadata about the directory (e.g., permissions, size, modification time) rather than its contents.

: For simply listing only directories and not files, consider using the `--only-dirs` (`-D`) option as an alternative.

`-L`, `--level=DEPTH`
: Limit the depth of recursion.

`-r`, `--reverse`
: Reverse the sort order.

`-s`, `--sort=SORT_FIELD`
: Which field to sort by.

Valid sort fields are ‘`name`’, ‘`Name`’, ‘`extension`’, ‘`Extension`’, ‘`size`’, ‘`block`’ [Unix only], ‘`modified`’, ‘`changed`’, ‘`accessed`’, ‘`created`’, ‘`inode`’, ‘`type`’, and ‘`none`’.

The ‘`block`’ sort field has the aliases ‘`blocks`’ and ‘`blocksize`’.

The `modified` sort field has the aliases ‘`date`’, ‘`time`’, and ‘`newest`’, and its reverse order has the aliases ‘`age`’ and ‘`oldest`’.

Sort fields starting with a capital letter will sort uppercase before lowercase: ‘A’ then ‘B’ then ‘a’ then ‘b’. Fields starting with a lowercase letter will mix them: ‘A’ then ‘a’ then ‘B’ then ‘b’.

`-I`, `--ignore-glob=GLOBS`
: Glob patterns, pipe-separated, of files to ignore.

`--git-ignore` [if lsr was built with git support]
: Do not list files that are ignored by Git.

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


LONG VIEW OPTIONS
=================

These options are available when running with `--long` (`-l`):

`-b`, `--binary`
: List file sizes with binary prefixes. Overrides preceding `-B`/`--bytes` flags.

`-B`, `--bytes`
: List file sizes in bytes, without any prefixes. Overrides preceding `-b`/`--binary` flags.

`--changed`
: Use the changed timestamp field.

`-g`, `--group`
: List each file’s group.

`--smart-group`
: Only show group if it has a different name from owner.

`-h`, `--header`
: Add a header row to each column.

`-H`, `--links`
: List each file’s number of hard links.

`-i`, `--inode`
: List each file’s inode number.

`--loc[=MODE]`
: Add a language column and a lines-of-code column to the long view.

: Only regular files in a recognised programming language (including Odin, Rust, C/C++, Python, Go, and 100+ others) are counted; counting is comment-aware, so the code column excludes comment and blank lines.

: Valid modes are ‘`lines`’ (the count of code lines), ‘`percent`’ (each file’s share of the code in the whole tree), and ‘`both`’ (the default). In `percent` and `both` modes the denominator is the total code across the recursed tree, or the git repository if one is present.

`-m`, `--modified`
: Use the modified timestamp field.

`-M`, `--mounts`
: Show mount details (Linux and macOS only).

`-n`, `--numeric`
: List numeric user and group IDs.

`-O`, `--flags`
: List file flags on macOS and BSD systems and file attributes on Windows systems. By default, Windows attributes are displayed in a long form. To display in attributes as single character set the environment variable `LSR_WINDOWS_ATTRIBUTES=short`. On BSD systems see chflags(1) for a list of file flags and their meanings.

`-S`, `--blocksize`
: List each file’s size of allocated file system blocks.

`-t`, `--time=WORD`
: Which timestamp field to list.

: Valid timestamp fields are ‘`modified`’, ‘`changed`’, ‘`accessed`’, and ‘`created`’.

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
: When you wish to pipe directories to lsr/read from stdin. Separate one per line or define custom separation char in `LSR_STDIN_SEPARATOR` / `EZA_STDIN_SEPARATOR` env variable.

`-@`, `--extended`
: List each file’s extended attributes and sizes.

`-Z`, `--context`
: List each file's security context.

`--git` [if lsr was built with git support]
: List each file’s Git status, if tracked.
This adds a two-character column indicating the staged and unstaged statuses respectively. The status character can be ‘`-`’ for not modified, ‘`M`’ for a modified file, ‘`N`’ for a new file, ‘`D`’ for deleted, ‘`R`’ for renamed, ‘`T`’ for type-change, ‘`I`’ for ignored, and ‘`U`’ for conflicted. Directories will be shown to have the status of their contents, which is how ‘deleted’ is possible if a directory contains a file that has a certain status, it will be shown to have that status.

`--git-repos` [if lsr was built with git support]
: List each directory’s Git status, if tracked.
Symbols shown are `|`= clean, `+`= dirty, and `~`= for unknown.

`--git-repos-no-status` [if lsr was built with git support]
: List if a directory is a Git repository, but not its status.
All Git repository directories will be shown as (themed) `-` without status indicated.

`--no-git`
: Don't show Git status (always overrides `--git`, `--git-repos`, `--git-repos-no-status`).


ENVIRONMENT VARIABLES
=====================

If an environment variable prefixed with `LSR_` is not set, for backward compatibility, it will default to its counterpart starting with `EZA_` or `EXA_`.

lsr responds to the following environment variables:

## `COLUMNS`

Overrides the width of the terminal, in characters, however, `-w` takes precedence.

For example, ‘`COLUMNS=80 lsr`’ will show a grid view with a maximum width of 80 characters.

This option won’t do anything when lsr’s output doesn’t wrap, such as when using the `--long` view.

## `LSR_STRICT`, `EZA_STRICT`

Enables _strict mode_, which will make lsr error when two command-line options are incompatible.

Usually, options can override each other going right-to-left on the command line, so that lsr can be given aliases: creating an alias ‘`lsr=lsr --sort=ext`’ then running ‘`lsr --sort=size`’ with that alias will run ‘`lsr --sort=ext --sort=size`’, and the sorting specified by the user will override the sorting specified by the alias.

In strict mode, the two options will not co-operate, and lsr will error.

This option is intended for use with automated scripts and other situations where you want to be certain you’re typing in the right command.

## `LSR_GRID_ROWS`, `EZA_GRID_ROWS`

Limits the grid-details view (‘`lsr --grid --long`’) so it’s only activated when at least the given number of rows of output would be generated.

With widescreen displays, it’s possible for the grid to look very wide and sparse, on just one or two lines with none of the columns lining up.
By specifying a minimum number of rows, you can only use the view if it’s going to be worth using.

## `LSR_ICON_SPACING`, `EZA_ICON_SPACING`

Specifies the number of spaces to print between an icon (see the ‘`--icons`’ option) and its file name.

Different terminals display icons differently, as they usually take up more than one character width on screen, so there’s no “standard” number of spaces that lsr can use to separate an icon from text. One space may place the icon too close to the text, and two spaces may place it too far away. So the choice is left up to the user to configure depending on their terminal emulator.

## `NO_COLOR`

Disables colours in the output (regardless of its value). Can be overridden by `--color` option.

See `https://no-color.org/` for details.

## `LS_COLORS`, `LSR_COLORS`, `EZA_COLORS`

Specifies the colour scheme used to highlight files based on their name and kind, as well as highlighting metadata and parts of the UI.

For more information on the format of these environment variables, see the [lsr_colors.5.md](lsr_colors.5.md) manual page.

## `LSR_OVERRIDE_GIT`, `EZA_OVERRIDE_GIT`

Overrides any `--git` or `--git-repos` argument.

## `LSR_MIN_LUMINANCE`, `EZA_MIN_LUMINANCE`

Specifies the minimum luminance to use when color-scale is active. Its value can be between -100 to 100.

## `LSR_ICONS_AUTO`, `EZA_ICONS_AUTO`

If set, automates the same behavior as using `--icons` or `--icons=auto`. Useful for if you always want to have icons enabled.

Any explicit use of the `--icons=WHEN` flag overrides this behavior.

## `LSR_STDIN_SEPARATOR`, `EZA_STDIN_SEPARATOR`

Specifies the separator to use when file names are piped from stdin. Defaults to newline.

## `LSR_CONFIG_DIR`, `EZA_CONFIG_DIR`

Specifies the directory where lsr will look for its configuration and theme files. Defaults to `$XDG_CONFIG_HOME/lsr`, `$XDG_CONFIG_HOME/eza`, `$HOME/.config/lsr`, or `$HOME/.config/eza` if `XDG_CONFIG_HOME` is not set.


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

lsr is maintained by fxrdhan <https://github.com/fxrdhan>.

**Source code:** `https://github.com/fxrdhan/lsr` \
**Contributors:** `https://github.com/fxrdhan/lsr/graphs/contributors`

Lineage: `exa` (by Benjamin Sago) ➔ `eza` (community fork) ➔ `lsr` (by fxrdhan).


SEE ALSO
========

- [**lsr_colors**(5)](lsr_colors.5.md)
- [**lsr_colors-explanation**(5)](lsr_colors-explanation.5.md)
