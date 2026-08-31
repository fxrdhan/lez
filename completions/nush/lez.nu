# Nushell rewrites `--flag=value` into two arguments for any flag this file
# declares, and rejects it outright for a flag declared without a type. Nine
# of lez's flags only accept a value with an equals sign, so declaring them
# here is what breaks them: `lez --color=always` becomes a parse error, and
# `lez --loc=lines` reaches the binary as `--loc lines`, where `lines` is read
# as a path. Left undeclared they pass through untouched and work, at the cost
# of not being listed here. They are: --classify/-F, --color, --colour,
# --color-scale, --colour-scale, --icons, --quotes, --hyperlink, --absolute,
# --code and --loc.
export extern "lez" [
    --version                  # Show version of lez
    -v                         # Sort numerically within names, as ls -v does (the default)
    --help                     # Show list of command-line options
    --config: string           # Load options from specified configuration file
    --no-config                # Do not load any configuration files
    --oneline(-1)              # Display one entry per line
    --long(-l)                 # Display extended file metadata as a table
    --grid(-G)                 # Display entries in a grid
    --across(-x)               # Sort the grid across, rather than downwards
    --recurse(-R)              # Recurse into directories
    --json                     # Output file listing and metadata as structured JSON
    --spacing: string          # Number of spaces between columns in grid views
    --tree(-T)                 # Recurse into directories as a tree
    --dereference(-X)          # Dereference symbolic links when displaying file information
    --color-scale-mode: string # Use gradient or fixed colors in --color-scale
    --colour-scale-mode: string # Use gradient or fixed colors in --colour-scale
    --no-quotes                # Don't quote file names with spaces
    --short-nix                # Abbreviate Nix store hashes in file names and paths
    --no-symlink-targets       # Do not show symlink targets
    --summary                  # Display total summary statistics of entries
    --follow-symlinks          # Drill down into symbolic links that point to directories
    --group-directories-first  # Sort directories before other files
    --group-directories-last   # Sort directories after other files
    --inspect-archives         # List contents of supported archives (.tar) in long view
    --ignore-submodule-contents # Do not list contents of submodules
    --warn-hidden(-W)          # Print a tally of hidden and ignored items; twice to always print
    --no-extended              # Do not show a marker if a file's extended attributes exist
    --git-ignore               # Ignore files mentioned in '.gitignore'
    --cachedir-ignore          # Ignore directories with a 'CACHEDIR.TAG' file
    --since: string            # Filter and display only files created or modified within duration window
    --all(-a)                  # Show hidden and 'dot' files. Use this twice to also show the '.' and '..' directories
    --almost-all(-A)           # Equivalent to --all; included for compatibility with `ls -A`
    --show-dotfiles            # Show dot-prefixed files without showing other hidden files
    --treat-dirs-as-files(-d)  # List directories like regular files
    --level(-L): string        # Limit the depth of recursion
    --width(-w): string        # Limits column output of grid, 0 implies auto-width
    --reverse(-r)              # Reverse the sort order
    --sort(-s): string         # Which field to sort by
    --ignore-glob(-I): string  # Ignore files that match these glob patterns
    --ignore-glob-ci: string   # Ignore files that match these glob patterns case-insensitively
    --only-dirs(-D)            # List only directories
    --only-files(-f)           # List only files
    --show-symlinks            # Explicitly show symbolic links (for use with --only-dirs | --only-files)
    --no-symlinks              # Do not show symbolic links
    --binary(-b)               # List file sizes with binary prefixes
    --bytes(-B)                # List file sizes in bytes, without any prefixes
    --group(-g)                # List each file's group
    --header(-h)               # Add a header row to each column
    --links(-H)                # List each file's number of hard links
    --inode(-i)                # List each file's inode number
    --blocksize(-S)            # List the allocated size of each file, in bytes
    --blocks                   # List the allocated size of each file, in blocks
    --time(-t): string         # Which timestamp field to list
    --modified(-m)             # Use the modified timestamp field
    --numeric(-n)              # List numeric user and group IDs.
    --changed                  # Use the changed timestamp field
    --accessed(-u)             # Use the accessed timestamp field
    --created(-U)              # Use the created timestamp field
    --utc                      # Show the time in the UTC timezone
    --time-style: string       # How to format timestamps
    --total-size               # Show recursive directory size (unix only)
    --no-permissions           # Suppress the permissions field
    --octal-permissions(-o)    # List each file's permission in octal format
    --no-filesize              # Suppress the filesize field
    --size-digits: int         # Number of digits to display for file sizes (1..=8)
    --digits: int              # Number of digits to display for file sizes (1..=8)
    --no-user                  # Suppress the user field
    --no-time                  # Suppress the time field
    --mounts(-M)               # Show mount details
    --git                      # List each file's Git status, if tracked
    --git-glyphs               # Display Git status with Nerd Font glyphs / icons
    --no-git                   # Suppress Git status
    --git-repos                # List each git-repos status and branch name
    --git-repos-no-status      # List each git-repos branch name (much faster)
    --extended(-@)             # List each file's extended attributes and sizes
    --context(-Z)              # List each file's security context
    --flags(-O)                # List file flags (Mac, BSD, and Windows only)
    --tags(-e)                 # List each file's color tags stored in extended attributes
    --smart-group              # Only show group if it has a different name from owner
    --stdin                    # When piping to lez. Read file paths from stdin
    --print-total              # Display total number of entries
    --mime-types               # Determine file MIME types to better inform styling decisions (unix only)
]
