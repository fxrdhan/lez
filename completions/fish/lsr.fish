# Meta-stuff
complete -c lsr -s v -l version -d "Show version of lsr"
complete -c lsr -l help -d "Show list of command-line options"

# Display options
complete -c lsr -s 1 -l oneline -d "Display one entry per line"
complete -c lsr -s l -l long -d "Display extended file metadata as a table"
complete -c lsr -s G -l grid -d "Display entries in a grid"
complete -c lsr -s x -l across -d "Sort the grid across, rather than downwards"
complete -c lsr -s R -l recurse -d "Recurse into directories"
complete -c lsr -l json -d "Output file listing and metadata as structured JSON"
complete -c lsr -l spacing -d "Number of spaces between columns in grid views" -r
complete -c lsr -s T -l tree -d "Recurse into directories as a tree"
complete -c lsr -s X -l dereference -d "Dereference symbolic links when displaying file information"
complete -c lsr -s F -l classify -d "Display type indicator by file names"
complete -c lsr -l color \
    -l colour -d "When to use terminal colours" -x -a "
    always\t'Always use colour'
    auto\t'Use colour if standard output is a terminal'
    automatic\t'Use colour if standard output is a terminal'
    never\t'Never use colour'
"
complete -c lsr -l color-scale \
    -l colour-scale -d "Highlight levels 'field' distinctly" -x -a "
    all\t''
    age\t''
    size\t''
"
complete -c lsr -l color-scale-mode \
    -l colour-scale-mode \
    -d "Use gradient or fixed colors in --color-scale" -x -a "
    fixed\t'Highlight based on fixed colors'
    gradient\t'Highlight based \'field\' in relation to other files'
"
complete -c lsr -l icons -d "When to display icons" -x -a "
  always\t'Always display icons'
  auto\t'Display icons if standard output is a terminal'
  automatic\t'Display icons if standard output is a terminal'
  never\t'Never display icons'
"
complete -c lsr -l quotes -d "When to quote filenames" -x -a "
  always\t'Quote every filename'
  auto\t'Quote filenames that contain spaces or quotes'
  automatic\t'Quote filenames that contain spaces or quotes'
  never\t'Never quote filenames'
"
complete -c lsr -l no-quotes -d "Don't quote file names with spaces"
complete -c lsr -l short-nix -d "Abbreviate Nix store hashes in file names and paths"
complete -c lsr -l no-symlink-targets -d "Do not show symlink targets"
complete -c lsr -l summary -d "Display total summary statistics of entries"
complete -c lsr -l hyperlink -d "When to display entries as hyperlinks" -x -a "
  always\t'Always display entries as hyperlinks'
  auto\t'Display hyperlinks if standard output is a terminal'
  automatic\t'Display hyperlinks if standard output is a terminal'
  never\t'Never display entries as hyperlinks'
"
complete -c lsr -l follow-symlinks -d "Drill down into symbolic links that point to directories"
complete -c lsr -l absolute -d "Display entries with their absolute path" -x -a "
  on\t'Show absolute path for listed entries'
  follow\t'Show absolute path with followed symlinks'
  off\t'Do not show the absolute path'
"
complete -c lsr -l smart-group -d "Only show group if it has a different name from owner"
complete -c lsr -l mime-types -d "Determine file MIME types to better inform styling decisions (unix only)"

# Filtering and sorting options
complete -c lsr -l group-directories-first -d "Sort directories before other files"
complete -c lsr -l group-directories-last -d "Sort directories after other files"
complete -c lsr -l inspect-archives -d "List the contents of supported archives (.tar) in long view"
complete -c lsr -l ignore-submodule-contents -d "Do not list contents of submodules"
complete -c lsr -s W -l warn-hidden -d "Print a tally of hidden and ignored items; twice to always print"
complete -c lsr -l no-extended -d "Do not show a marker if a file's extended attributes exist"
complete -c lsr -l git-ignore -d "Ignore files mentioned in '.gitignore'"
complete -c lsr -l cachedir-ignore -d "Ignore directories with a 'CACHEDIR.TAG' file"
complete -c lsr -l since -d "Filter and display only files created or modified within duration window" -r
complete -c lsr -s a -l all -d "Show hidden and 'dot' files. Use this twice to also show the '.' and '..' directories"
complete -c lsr -s A -l almost-all -d "Equivalent to --all; included for compatibility with `ls -A`"
complete -c lsr -l show-dotfiles -d "Show dot-prefixed files without showing other hidden files"
complete -c lsr -s d -l treat-dirs-as-files -d "List directories like regular files"
complete -c lsr -s L -l level -d "Limit the depth of recursion" -x -a "1 2 3 4 5 6 7 8 9"
complete -c lsr -l code -d "Summarise lines of code by language" -x -a "lines percent both"
complete -c lsr -s w -l width -d "Limits column output of grid, 0 implies auto-width"
complete -c lsr -s r -l reverse -d "Reverse the sort order"
complete -c lsr -s s -l sort -d "Which field to sort by" -x -a "
    accessed\t'Sort by file accessed time'
    age\t'Sort by file modified time (newest first)'
    changed\t'Sort by changed time'
    created\t'Sort by file modified time'
    date\t'Sort by file modified time'
    ext\t'Sort by file extension'
    Ext\t'Sort by file extension (uppercase first)'
    extension\t'Sort by file extension'
    Extension\t'Sort by file extension (uppercase first)'
    filename\t'Sort by filename'
    Filename\t'Sort by filename (uppercase first)'
    inode\t'Sort by file inode'
    mod\t'Sort by file modified time'
    modified\t'Sort by file modified time'
    name\t'Sort by filename'
    Name\t'Sort by filename (uppercase first)'
    new\t'Sort by file modified time (newest first)'
    newest\t'Sort by file modified time (newest first)'
    none\t'Do not sort files at all'
    old\t'Sort by file modified time'
    oldest\t'Sort by file modified time'
    path\t'Sort by file path'
    Path\t'Sort by file path (uppercase first)'
    relative-path\t'Sort by relative file path'
    Relative-path\t'Sort by relative file path (uppercase first)'
    Relative-Path\t'Sort by relative file path (uppercase first)'
    relpath\t'Sort by relative file path'
    Relpath\t'Sort by relative file path (uppercase first)'
    relative_path\t'Sort by relative file path'
    Relative_path\t'Sort by relative file path (uppercase first)'
    size\t'Sort by file size'
    block\t'Sort by file block size'
    blocks\t'Sort by file block size'
    blocksize\t'Sort by file block size'
    time\t'Sort by file modified time'
    type\t'Sort by file type'
"

complete -c lsr -s I -l ignore-glob -d "Ignore files that match these glob patterns" -r
complete -c lsr -l ignore-glob-ci -d "Ignore files that match these glob patterns case-insensitively" -r
complete -c lsr -s D -l only-dirs -d "List only directories"
complete -c lsr -s f -l only-files -d "List only files"
complete -c lsr -l show-symlinks -d "Explicitly show symbolic links (For use with --only-dirs | --only-files)"
complete -c lsr -l no-symlinks -d "Do not show symbolic links"

# Long view options
complete -c lsr -s b -l binary -d "List file sizes with binary prefixes"
complete -c lsr -s B -l bytes -d "List file sizes in bytes, without any prefixes"
complete -c lsr -s g -l group -d "List each file's group"
complete -c lsr -s h -l header -d "Add a header row to each column"
complete -c lsr -s H -l links -d "List each file's number of hard links"
complete -c lsr -s i -l inode -d "List each file's inode number"
complete -c lsr -l loc -d "Add lines-of-code and language columns" -x -a "lines percent both"
complete -c lsr -s S -l blocksize -d "List each file's size of allocated file system blocks"
complete -c lsr -s t -l time -d "Which timestamp field to list" -x -a "
    modified\t'Display modified time'
    mod\t'Display modified time'
    m\t'Display modified time'
    changed\t'Display changed time'
    accessed\t'Display accessed time'
    created\t'Display created time'
"
complete -c lsr -s m -l modified -d "Use the modified timestamp field"
complete -c lsr -s n -l numeric -d "List numeric user and group IDs."
complete -c lsr -l changed -d "Use the changed timestamp field"
complete -c lsr -s u -l accessed -d "Use the accessed timestamp field"
complete -c lsr -s U -l created -d "Use the created timestamp field"
complete -c lsr -l utc -d "Show the time in the UTC timezone"
complete -c lsr -l time-style -d "How to format timestamps" -x -a "
    default\t'Use the default time style'
    iso\t'Display brief ISO timestamps'
    long-iso\t'Display longer ISO timestamps, up to the minute'
    full-iso\t'Display full ISO timestamps, up to the nanosecond'
    relative\t'Display relative timestamps'
    +FORMAT\t'Use custom time style'
"
complete -c lsr -l total-size -d "Show recursive directory size (unix only)"
complete -c lsr -l no-permissions -d "Suppress the permissions field"
complete -c lsr -s o -l octal-permissions -d "List each file's permission in octal format"
complete -c lsr -l no-filesize -d "Suppress the filesize field"
complete -c lsr -l no-user -d "Suppress the user field"
complete -c lsr -l no-time -d "Suppress the time field"
complete -c lsr -s M -l mounts -d "Show mount details"
complete -c lsr -l stdin -d "When piping to eza. Read file names from stdin"
complete -c lsr -l print-total -d "Display total number of entries"

# Optional extras
complete -c lsr -l git -d "List each file's Git status, if tracked"
complete -c lsr -l git-glyphs -d "Display Git status with Nerd Font glyphs / icons"
complete -c lsr -l no-git -d "Suppress Git status"
complete -c lsr -l git-repos -d "List each git-repos status and branch name"
complete -c lsr -l git-repos-no-status -d "List each git-repos branch name (much faster)"
complete -c lsr -s '@' -l extended -d "List each file's extended attributes and sizes"
complete -c lsr -s Z -l context -d "List each file's security context"
complete -c lsr -s O -l flags -d "List file flags (Mac, BSD, and Windows only)"
complete -c lsr -s e -l tags -d "List each file's color tags stored in extended attributes"
