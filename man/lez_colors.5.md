% lez_colors(5) $version

<!-- This is the lez_colors(5) man page, written in Markdown. -->
<!-- To generate the roff version, run `just man`, -->
<!-- and the man page will appear in the ‘target’ directory. -->


NAME
====

lez_colors — customising the file and UI colours of lez


SYNOPSIS
========

The `LEZ_COLORS` or `EZA_COLORS` environment variable can be used to customise the colours that `lez` uses to highlight file names, file metadata, and parts of the UI.

You can use the `dircolors` program to generate a script that sets the variable from an input file, or if you don’t mind editing long strings of text, you can just type it out directly. These variables have the following structure:

- A list of key-value pairs separated by ‘`=`’, such as ‘`*.txt=32`’.
- Multiple ANSI formatting codes are separated by ‘`;`’, such as ‘`*.txt=32;1;4`’.
- Finally, multiple pairs are separated by ‘`:`’, such as ‘`*.txt=32:*.mp3=1;35`’.

The key half of the pair can either be a two-letter code or a file glob, and anything that’s not a valid code will be treated as a glob, including keys that happen to be two letters long.

For backwards compatibility `EZA_COLORS` and `EXA_COLORS` environment variables are checked if `LEZ_COLORS` is unset.


EXAMPLES
========

`LEZ_COLORS="uu=0:gu=0"`
: Disable the “current user” highlighting

`LEZ_COLORS="da=32"`
: Turn the date column green

`LEZ_COLORS="Vagrantfile=1;4;33"`
: Highlight Vagrantfiles

`LEZ_COLORS="*.zip=38;5;125"`
: Override the existing zip colour

`LEZ_COLORS="*.md=38;5;121:*.log=38;5;248"`
: Markdown files a shade of green, log files a shade of grey


LIST OF CODES
=============

`LS_COLORS` can use these eleven codes:

`di`
: directories

`ex`
: executable files

`fi`
: regular files

`pi`
: named pipes

`so`
: sockets

`bd`
: block devices

`cd`
: character devices

`ln`
: symlinks

`or`
: symlinks with no target

`ca`
: files carrying Linux capabilities

`mh`
: regular files with more than one hard link

Note: `ca` is unset by default, and only when it is set does lez ask the filesystem whether a file has capabilities — one extra syscall per file. GNU `ls` stopped colouring these by default in coreutils 8.31 for the same reason. The attribute is Linux's alone; the code is accepted and ignored elsewhere.

Note: `mh` is unset by default, matching GNU `ls`, so multiply-linked files are coloured like any other file until it is given a style. It applies to regular files only — directories are linked from each of their subdirectories, so colouring those would light up almost everything. Unix only.


`LEZ_COLORS` (and `EZA_COLORS`) can use many more:

`oc`
: the permissions displayed as octal

`ur`
: the user-read permission bit

`uw`
: the user-write permission bit

`ux`
: the user-execute permission bit for regular files

`ue`
: the user-execute for other file kinds

`gr`
: the group-read permission bit

`gw`
: the group-write permission bit

`gx`
: the group-execute permission bit

`tr`
: the others-read permission bit

`tw`
: the others-write permission bit

`tx`
: the others-execute permission bit

`su`
: setuid, setgid, and sticky permission bits for files

`sf`
: setuid, setgid, and sticky for other file kinds

`xa`
: the extended attribute indicator

`sn`
: the numbers of a file’s size (sets `nb`, `nk`, `nm`, `ng` and `nt`)

`nb`
: the numbers of a file’s size if it is lower than 1 KB/Kib

`nk`
: the numbers of a file’s size if it is between 1 KB/KiB and 1 MB/MiB

`nm`
: the numbers of a file’s size if it is between 1 MB/MiB and 1 GB/GiB

`ng`
: the numbers of a file’s size if it is between 1 GB/GiB and 1 TB/TiB

`nt`
: the numbers of a file’s size if it is 1 TB/TiB or higher

`sb`
: the units of a file’s size (sets `ub`, `uk`, `um`, `ug` and `ut`)

`ub`
: the units of a file’s size if it is lower than 1 KB/Kib

`uk`
: the units of a file’s size if it is between 1 KB/KiB and 1 MB/MiB

`um`
: the units of a file’s size if it is between 1 MB/MiB and 1 GB/GiB

`ug`
: the units of a file’s size if it is between 1 GB/GiB and 1 TB/TiB

`ut`
: the units of a file’s size if it is 1 TB/TiB or higher

`df`
: a device’s major ID

`ds`
: a device’s minor ID

`uu`
: a user that’s you

`uR`
: a user that's root

`un`
: a user that’s someone else

`gu`
: a group that you belong to

`gR`
: a group related to root

`gn`
: a group you aren’t a member of

`lc`
: a number of hard links

`lm`
: a number of hard links for a regular file with at least two

`ga`
: a new flag in Git

`gm`
: a modified flag in Git

`gd`
: a deleted flag in Git

`gv`
: a renamed flag in Git

`gt`
: a modified metadata flag in Git

`gi`
: an ignored flag in Git

`gc`
: a conflicted flag in Git

`Gm`
: main branch of repo

`Go`
: other branch of repo

`Gc`
: clean branch of repo

`Gd`
: dirty branch of repo

`Gw`
: worktree branch of repo

`xx`
: “punctuation”, including many background UI elements

`da`
: a file’s date

`in`
: a file’s inode number

`bl`
: a file’s allocated size, as shown by `--blocksize`

`hd`
: the header row of a table

`lp`
: the path of a symlink

`qu`
: quotation marks around filenames with spaces or special characters

`cc`
: an escaped character in a filename

`bO`
: the overlay style for broken symlink paths

`sp`
: special (not file, dir, mount, exec, pipe, socket, block device, char device, or link)

`mp`
: a mount point

`sv`
: a Btrfs subvolume (Linux only)

`im`
: a regular file that is an image

`vi`
: a regular file that is a video

`mu`
: a regular file that is lossy music

`lo`
: a regular file that is lossless music

`cr`
: a regular file that is related to cryptography (ex: key or certificate)

`do`
: a regular file that is a document (ex: office suite document or PDF)

`co`
: a regular file that is compressed

`tm`
: a regular file that is temporary (ex: a text editor's backup file)

`cm`
: a regular file that is a compilation artifact (ex: Java class file)

`bu`
: a regular file that is used to build a project (ex: Makefile)

`sc`
: a regular file that is source code

`dt`
: a regular file that is data (ex: CSV, Parquet, HDF5, SQLite, or NumPy array)

`ic`
: the icon (this is optional, if not set the icon color matches the file name's)

`Sn`
: No security context on a file

`Su`
: SELinux user

`Sr`
: SELinux role

`St`
: SELinux type

`Sl`
: SELinux level

`ff`
: BSD file flags

Values in `LEZ_COLORS` or `EZA_COLORS` override those given in `LS_COLORS`, so you don’t need to re-write an existing `LS_COLORS` variable with proprietary extensions.


LIST OF STYLES
==============

Unlike some versions of `ls`, the given ANSI values must be valid colour codes: lez won’t just print out whichever characters are given.

The codes accepted by lez are:

`1`
: for bold

`2`
: for dimmed

`3`
: for italic

`4`
: for underline

`31`
: for red text

`32`
: for green text

`33`
: for yellow text

`34`
: for blue text

`35`
: for purple text

`36`
: for cyan text

`37`
: for white text

`90`
: for dark gray text

`91`
: for bright red text

`92`
: for bright green text

`93`
: for bright yellow text

`94`
: for bright blue text

`95`
: for bright purple text

`96`
: for bright cyan text

`97`
: for bright text

`38;5;nnn`
: for a colour from 0 to 255 (replace the `nnn` part)

Many terminals will treat bolded text as a different colour, or at least provide the option to.

lez provides its own built-in set of file extension mappings that cover a large range of common file extensions, including documents, archives, media, and temporary files.
Any mappings in the environment variables will override this default set: running lez with `LS_COLORS="*.zip=32"` will turn zip files green but leave the colours of other compressed files alone.

You can also disable this built-in set entirely by including a `reset` entry at the beginning of `LEZ_COLORS` or `EZA_COLORS`.
So setting `LEZ_COLORS="reset:*.txt=31"` will highlight only text files in addition to any styles in `LS_COLORS`; setting `LEZ_COLORS="reset"` will highlight nothing.


AUTHOR
======

lez is maintained by fxrdhan <https://github.com/fxrdhan>.

**Source code:** `https://github.com/fxrdhan/lez` \
**Contributors:** `https://github.com/fxrdhan/lez/graphs/contributors`

Lineage: `exa` (by Benjamin Sago) ➔ `eza` (community fork) ➔ `lez` (by fxrdhan).


SEE ALSO
========

**lez**(1), **lez_colors-explanation**(5)
