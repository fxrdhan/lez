# Nine flags take an optional value and only accept it with an equals sign, as
# `lez --absolute=on`. Offering those values after a space would build a
# command line the parser reads differently from what the user meant: the
# value lands as a path and the flag falls back to its default. fish has no
# equals-only completion, so gate the value lists on the token already
# carrying one; the bare flag is completed by a second, unconditional entry.
function __lez_value_follows_an_equals_sign
    string match -q -- '*=*' (commandline -ct)
end

# Meta-stuff
complete -c lez -l version -d "Show version of lez"
complete -c lez -s v -d "Sort numerically within names, as ls -v does (the default)"
complete -c lez -l help -d "Show list of command-line options"

# Display options
complete -c lez -s 1 -l oneline -d "Display one entry per line"
complete -c lez -s l -l long -d "Display extended file metadata as a table"
complete -c lez -s G -l grid -d "Display entries in a grid"
complete -c lez -s x -l across -d "Sort the grid across, rather than downwards"
complete -c lez -s R -l recurse -d "Recurse into directories"
complete -c lez -l json -d "Output file listing and metadata as structured JSON"
complete -c lez -l spacing -d "Number of spaces between columns in grid views" -r
complete -c lez -s T -l tree -d "Recurse into directories as a tree"
complete -c lez -s X -l dereference -d "Dereference symbolic links when displaying file information"
complete -c lez -s F -l classify -d "Display type indicator by file names"
complete -c lez -l classify -d "Display type indicator by file names" -x -n "__lez_value_follows_an_equals_sign" -a "
    always\t'Always display type indicators'
    auto\t'Display type indicators if standard output is a terminal'
    automatic\t'Display type indicators if standard output is a terminal'
    never\t'Never display type indicators'
"
complete -c lez -l color -l colour -d "When to use terminal colours"
complete -c lez -l color \
    -l colour -d "When to use terminal colours" -x -n "__lez_value_follows_an_equals_sign" -a "
    always\t'Always use colour'
    auto\t'Use colour if standard output is a terminal'
    automatic\t'Use colour if standard output is a terminal'
    never\t'Never use colour'
"
complete -c lez -l color-scale -l colour-scale -d "Highlight levels 'field' distinctly"
complete -c lez -l color-scale \
    -l colour-scale -d "Highlight levels 'field' distinctly" -x -n "__lez_value_follows_an_equals_sign" -a "
    all\t''
    age\t''
    size\t''
"
complete -c lez -l color-scale-mode \
    -l colour-scale-mode \
    -d "Use gradient or fixed colors in --color-scale" -x -a "
    fixed\t'Highlight based on fixed colors'
    gradient\t'Highlight based \'field\' in relation to other files'
"
complete -c lez -l icons -d "When to display icons"
complete -c lez -l icons -d "When to display icons" -x -n "__lez_value_follows_an_equals_sign" -a "
  always\t'Always display icons'
  auto\t'Display icons if standard output is a terminal'
  automatic\t'Display icons if standard output is a terminal'
  never\t'Never display icons'
"
complete -c lez -l quotes -d "When to quote filenames"
complete -c lez -l quotes -d "When to quote filenames" -x -n "__lez_value_follows_an_equals_sign" -a "
  always\t'Quote every filename'
  auto\t'Quote filenames that contain spaces or quotes'
  automatic\t'Quote filenames that contain spaces or quotes'
  never\t'Never quote filenames'
"
complete -c lez -l no-quotes -d "Don't quote file names with spaces"
complete -c lez -l short-nix -d "Abbreviate Nix store hashes in file names and paths"
complete -c lez -l no-symlink-targets -d "Do not show symlink targets"
complete -c lez -l summary -d "Display total summary statistics of entries"
complete -c lez -l hyperlink -d "When to display entries as hyperlinks"
complete -c lez -l hyperlink -d "When to display entries as hyperlinks" -x -n "__lez_value_follows_an_equals_sign" -a "
  always\t'Always display entries as hyperlinks'
  auto\t'Display hyperlinks if standard output is a terminal'
  automatic\t'Display hyperlinks if standard output is a terminal'
  never\t'Never display entries as hyperlinks'
"
complete -c lez -l follow-symlinks -d "Drill down into symbolic links that point to directories"
complete -c lez -l absolute -d "Display entries with their absolute path"
complete -c lez -l absolute -d "Display entries with their absolute path" -x -n "__lez_value_follows_an_equals_sign" -a "
  on\t'Show absolute path for listed entries'
  follow\t'Show absolute path with followed symlinks'
  off\t'Do not show the absolute path'
"
complete -c lez -l smart-group -d "Only show group if it has a different name from owner"
complete -c lez -l mime-types -d "Determine file MIME types to better inform styling decisions (unix only)"

# Filtering and sorting options
complete -c lez -l group-directories-first -d "Sort directories before other files"
complete -c lez -l group-directories-last -d "Sort directories after other files"
complete -c lez -l inspect-archives -d "List the contents of supported archives (.tar) in long view"
complete -c lez -l ignore-submodule-contents -d "Do not list contents of submodules"
complete -c lez -s W -l warn-hidden -d "Print a tally of hidden and ignored items; twice to always print"
complete -c lez -l no-extended -d "Do not show a marker if a file's extended attributes exist"
complete -c lez -l git-ignore -d "Ignore files mentioned in '.gitignore'"
complete -c lez -l cachedir-ignore -d "Ignore directories with a 'CACHEDIR.TAG' file"
complete -c lez -l since -d "Filter and display only files created or modified within duration window" -r
complete -c lez -s a -l all -d "Show hidden and 'dot' files. Use this twice to also show the '.' and '..' directories"
complete -c lez -s A -l almost-all -d "Equivalent to --all; included for compatibility with `ls -A`"
complete -c lez -l show-dotfiles -d "Show dot-prefixed files without showing other hidden files"
complete -c lez -s d -l treat-dirs-as-files -d "List directories like regular files"
complete -c lez -s L -l level -d "Limit the depth of recursion" -x -a "1 2 3 4 5 6 7 8 9"
complete -c lez -l code -d "Summarise lines of code by language"
complete -c lez -l code -d "Summarise lines of code by language" -x -n "__lez_value_follows_an_equals_sign" -a "lines percent both"
complete -c lez -s w -l width -d "Limits column output of grid, 0 implies auto-width"
complete -c lez -s r -l reverse -d "Reverse the sort order"
complete -c lez -s s -l sort -d "Which field to sort by" -x -a "
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
    lexicographic\t'Sort by filename, code point by code point'
    Lexicographic\t'Sort by filename, code point by code point (uppercase first)'
    lex\t'Sort by filename, code point by code point'
    Lex\t'Sort by filename, code point by code point (uppercase first)'
    lg\t'Sort by filename, code point by code point'
    Lg\t'Sort by filename, code point by code point (uppercase first)'
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

complete -c lez -s I -l ignore-glob -d "Ignore files that match these glob patterns" -r
complete -c lez -l ignore-glob-ci -d "Ignore files that match these glob patterns case-insensitively" -r
complete -c lez -s D -l only-dirs -d "List only directories"
complete -c lez -s f -l only-files -d "List only files"
complete -c lez -l show-symlinks -d "Explicitly show symbolic links (For use with --only-dirs | --only-files)"
complete -c lez -l no-symlinks -d "Do not show symbolic links"

# Long view options
complete -c lez -s b -l binary -d "List file sizes with binary prefixes"
complete -c lez -s B -l bytes -d "List file sizes in bytes, without any prefixes"
complete -c lez -s g -l group -d "List each file's group"
complete -c lez -s h -l header -d "Add a header row to each column"
complete -c lez -s H -l links -d "List each file's number of hard links"
complete -c lez -s i -l inode -d "List each file's inode number"
complete -c lez -l loc -d "Add lines-of-code and language columns"
complete -c lez -l loc -d "Add lines-of-code and language columns" -x -n "__lez_value_follows_an_equals_sign" -a "lines percent both"
complete -c lez -s S -l blocksize -d "List each file's size of allocated file system blocks"
complete -c lez -s t -l time -d "Which timestamp field to list" -x -a "
    modified\t'Display modified time'
    mod\t'Display modified time'
    m\t'Display modified time'
    changed\t'Display changed time'
    accessed\t'Display accessed time'
    created\t'Display created time'
"
complete -c lez -s m -l modified -d "Use the modified timestamp field"
complete -c lez -s n -l numeric -d "List numeric user and group IDs."
complete -c lez -l changed -d "Use the changed timestamp field"
complete -c lez -s u -l accessed -d "Use the accessed timestamp field"
complete -c lez -s U -l created -d "Use the created timestamp field"
complete -c lez -l utc -d "Show the time in the UTC timezone"
complete -c lez -l time-style -d "How to format timestamps" -x -a "
    default\t'Use the default time style'
    iso\t'Display brief ISO timestamps'
    long-iso\t'Display longer ISO timestamps, up to the minute'
    full-iso\t'Display full ISO timestamps, up to the nanosecond'
    relative\t'Display relative timestamps'
    +FORMAT\t'Use custom time style'
"
complete -c lez -l total-size -d "Show recursive directory size (unix only)"
complete -c lez -l no-permissions -d "Suppress the permissions field"
complete -c lez -s o -l octal-permissions -d "List each file's permission in octal format"
complete -c lez -l no-filesize -d "Suppress the filesize field"
complete -c lez -l size-digits -d "Number of digits to display for file sizes (1..=8)"
complete -c lez -l digits -d "Number of digits to display for file sizes (1..=8)"
complete -c lez -l no-user -d "Suppress the user field"
complete -c lez -l no-time -d "Suppress the time field"
complete -c lez -s M -l mounts -d "Show mount details"
complete -c lez -l stdin -d "When piping to lez. Read file names from stdin"
complete -c lez -l print-total -d "Display total number of entries"

# Optional extras
complete -c lez -l git -d "List each file's Git status, if tracked"
complete -c lez -l git-glyphs -d "Display Git status with Nerd Font glyphs / icons"
complete -c lez -l no-git -d "Suppress Git status"
complete -c lez -l git-repos -d "List each git-repos status and branch name"
complete -c lez -l git-repos-no-status -d "List each git-repos branch name (much faster)"
complete -c lez -s '@' -l extended -d "List each file's extended attributes and sizes"
complete -c lez -s Z -l context -d "List each file's security context"
complete -c lez -s O -l flags -d "List file flags (Mac, BSD, and Windows only)"
complete -c lez -s e -l tags -d "List each file's color tags stored in extended attributes"
