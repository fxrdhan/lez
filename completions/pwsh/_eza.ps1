using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'eza' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $ArrayWhen           = @('always', 'auto', 'never')
    $ArraySort           = @('name', 'Name', 'lexicographic', 'Lexicographic', 'lex', 'Lex', 'lg', 'Lg', 'extension', 'Extension', 'path', 'Path', 'relative-path', 'Relative-path', 'Relative-Path', 'relpath', 'Relpath', 'relative_path', 'Relative_path', 'size', 'block', 'blocks', 'blocksize', 'type', 'created', 'modified', 'mod', 'date', 'time', 'old', 'oldest', 'age', 'new', 'newest', 'accessed', 'changed', 'inode', 'none')
    $ArrayColorScaleMode = @('fixed', 'gradient')
    $ArrayColorScale     = @('all', 'age', 'size')
    $ArrayAbsolute       = @('on', 'follow', 'off')
    $ArrayTime           = @('modified', 'mod', 'm', 'accessed', 'acc', 'changed', 'ch', 'created', 'cr')
    $ArrayTimeStyle      = @('default', 'iso', 'long-iso', 'full-iso', 'relative', '+%Y-%m-%d %H:%M', '+%Y.%m.%d %H:$M:$s')
    $ArrayCodeMode       = @('lines', 'percent', 'both')

    # Nine flags take an optional value and only accept it with an equals sign,
    # as `eza --absolute=on`. Offering these after a space would build a command
    # line the parser reads differently from what the user meant: the value
    # lands as a path and the flag falls back to its default. PowerShell keeps
    # `--absolute=on` as one token, so they are completed as whole words.
    $EqualsFlags = [ordered]@{
        '--absolute'     = $ArrayAbsolute
        '--classify'     = $ArrayWhen
        '--code'         = $ArrayCodeMode
        '--color'        = $ArrayWhen
        '--colour'       = $ArrayWhen
        '--color-scale'  = $ArrayColorScale
        '--colour-scale' = $ArrayColorScale
        '--hyperlink'    = $ArrayWhen
        '--icons'        = $ArrayWhen
        '--loc'          = $ArrayCodeMode
        '--quotes'       = $ArrayWhen
    }

    $commandElements = $commandAst.CommandElements
    $command = @(
        'eza'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(if ($wordToComplete -like '*=*') {
        foreach ($flag in $EqualsFlags.Keys) {
            foreach ($value in $EqualsFlags[$flag]) {
                [CompletionResult]::new("$flag=$value", "$flag=$value", [CompletionResultType]::ParameterValue, "$flag=$value")
            }
        }
    }
    else { switch  -Wildcard ($command) {
        '*;--help' {
            break
        }
        '*;--version' {
            break
        }
        '*;--sort' {
            $ArraySort | 
            ForEach-Object {[System.Management.Automation.CompletionResult]::new($_, $_, "ParameterValue", $_)}
            break
        }
        '*;--color-scale-mode' {
            $ArrayColorScaleMode | 
            ForEach-Object {[System.Management.Automation.CompletionResult]::new($_, $_, "ParameterValue", $_)}
            break
        }
        '*--long;*--time-style' {
            $ArrayTimeStyle | 
            ForEach-Object {[System.Management.Automation.CompletionResult]::new($_, $_, "ParameterValue", $_)}
            break
        }
        '*--long;*--time' {
            $ArrayTime | 
            ForEach-Object {[System.Management.Automation.CompletionResult]::new($_, $_, "ParameterValue", $_)}
            break
        }
        '*;--all' {
            [CompletionResult]::new('--show-symlinks'            ,'listfilessyl'        , [CompletionResultType]::ParameterName, 'explicitly show symbolic links (for use with --only-dirs | --only-files)')
            [CompletionResult]::new('--no-symlinks'              ,'listfilessyl'        , [CompletionResultType]::ParameterName, 'do not show symbolic links')
            break
        }
        '*long*' {
        #   [CompletionResult]::new('-b'                         ,'binary'              , [CompletionResultType]::ParameterName, 'list file sizes with binary prefixes')
            [CompletionResult]::new('--binary'                   ,'binary'              , [CompletionResultType]::ParameterName, 'list file sizes with binary prefixes')
        #   [CompletionResult]::new('-B'                         ,'bytes'               , [CompletionResultType]::ParameterName, 'list file sizes in bytes, without any prefixes')
            [CompletionResult]::new('--bytes'                    ,'bytes'               , [CompletionResultType]::ParameterName, 'list file sizes in bytes, without any prefixes') 
        #   [CompletionResult]::new('-g'                         ,'group'               , [CompletionResultType]::ParameterName, 'list each file''s group')
            [CompletionResult]::new('--smart-group'              ,'smart-group'         , [CompletionResultType]::ParameterName, 'only show group if it has a different name from owner') 
            [CompletionResult]::new('--group'                    ,'group'               , [CompletionResultType]::ParameterName, 'list each file''s group')
        #   [CompletionResult]::new('-h'                         ,'header'              , [CompletionResultType]::ParameterName, 'add a header row to each column')
            [CompletionResult]::new('--header'                   ,'header'              , [CompletionResultType]::ParameterName, 'add a header row to each column') 
        #   [CompletionResult]::new('-H'                         ,'links'               , [CompletionResultType]::ParameterName, 'list each file''s number of hard links')
            [CompletionResult]::new('--links'                    ,'links'               , [CompletionResultType]::ParameterName, 'list each file''s number of hard links') 
        #   [CompletionResult]::new('-i'                         ,'inode'               , [CompletionResultType]::ParameterName, 'list each file''s inode number')
            [CompletionResult]::new('--inode'                    ,'inode'               , [CompletionResultType]::ParameterName, 'list each file''s inode number')
            [CompletionResult]::new('--loc'                      ,'loc'                 , [CompletionResultType]::ParameterName, 'add lines-of-code and language columns')
            [CompletionResult]::new('--code'                     ,'code'                , [CompletionResultType]::ParameterName, 'summarise lines of code by language')
        #   [CompletionResult]::new('-M'                         ,'mounts'              , [CompletionResultType]::ParameterName, 'show mount details (Linux and Mac only)')
        #   [CompletionResult]::new('--mounts'                   ,'mounts'              , [CompletionResultType]::ParameterName, 'show mount details (Linux and Mac only)') 
        #   [CompletionResult]::new('-n'                         ,'numeric'             , [CompletionResultType]::ParameterName, 'list numeric user and group IDs')
            [CompletionResult]::new('--numeric'                  ,'numeric'             , [CompletionResultType]::ParameterName, 'list numeric user and group IDs') 
        #   [CompletionResult]::new('-O'                         ,'flags'               , [CompletionResultType]::ParameterName, 'list file flags (Mac, BSD, and Windows only)')
            [CompletionResult]::new('--flags'                    ,'flags'               , [CompletionResultType]::ParameterName, 'list file flags (Mac, BSD, and Windows only)') 
        #   [CompletionResult]::new('-S'                         ,'blocksize'           , [CompletionResultType]::ParameterName, 'list the allocated size of each file, in bytes')
            [CompletionResult]::new('--blocksize'                ,'blocksize'           , [CompletionResultType]::ParameterName, 'list the allocated size of each file, in bytes') 
            [CompletionResult]::new('--blocks'                   ,'blocks'              , [CompletionResultType]::ParameterName, 'list the allocated size of each file, in blocks') 
        #   [CompletionResult]::new('-t'                         ,'time'                , [CompletionResultType]::ParameterName, 'which timestamp field to list (modified, accessed, created)')
            [CompletionResult]::new('--time'                     ,'time'                , [CompletionResultType]::ParameterName, 'which timestamp field to list (modified, accessed, created)') 
        #   [CompletionResult]::new('-m'                         ,'modified'            , [CompletionResultType]::ParameterName, 'use the modified timestamp field')
            [CompletionResult]::new('--modified'                 ,'modified'            , [CompletionResultType]::ParameterName, 'use the modified timestamp field') 
        #   [CompletionResult]::new('-u'                         ,'accessed'            , [CompletionResultType]::ParameterName, 'use the accessed timestamp field')
            [CompletionResult]::new('--accessed'                 ,'accessed'            , [CompletionResultType]::ParameterName, 'use the accessed timestamp field') 
        #   [CompletionResult]::new('-U'                         ,'created'             , [CompletionResultType]::ParameterName, 'use the created timestamp field')
            [CompletionResult]::new('--changed'                  ,'changed'             , [CompletionResultType]::ParameterName, 'use the changed timestamp field') 
            [CompletionResult]::new('--created'                  ,'created'             , [CompletionResultType]::ParameterName, 'use the created timestamp field')
            [CompletionResult]::new('--utc'                      ,'utc'                  , [CompletionResultType]::ParameterName, 'show the time in the UTC timezone')
            [CompletionResult]::new('--time-style'               ,'time-style'          , [CompletionResultType]::ParameterName, 'how to format timestamps (default, iso, long-iso,full-iso, relative, or a custom style ''+<FORMAT>'' like ''+%Y-%m-%d %H:%M'')')
        #   [CompletionResult]::new('--total-size'               ,'total-size'          , [CompletionResultType]::ParameterName, 'show the size of a directory as the size of all files and directories inside (unix only)')
        #   [CompletionResult]::new('-o'                         ,'octal-permissions'   , [CompletionResultType]::ParameterName, 'list each file''s permission in octal format')
            [CompletionResult]::new('--no-permissions'           ,'no-permissions'      , [CompletionResultType]::ParameterName, 'suppress the permissions field') 
            [CompletionResult]::new('--octal-permissions'        ,'octal-permissions'   , [CompletionResultType]::ParameterName, 'list each file''s permission in octal format')
            [CompletionResult]::new('--no-filesize'              ,'no-filesize'         , [CompletionResultType]::ParameterName, 'suppress the filesize field')
            [CompletionResult]::new('--size-digits'              ,'size-digits'         , [CompletionResultType]::ParameterName, 'number of digits to display for file sizes (1..=8, default: 3)')
            [CompletionResult]::new('--digits'                   ,'digits'              , [CompletionResultType]::ParameterName, 'number of digits to display for file sizes (1..=8, default: 3)')
            [CompletionResult]::new('--no-user'                  ,'no-user'             , [CompletionResultType]::ParameterName, 'suppress the user field')
            [CompletionResult]::new('--no-time'                  ,'no-time'             , [CompletionResultType]::ParameterName, 'suppress the time field')
            [CompletionResult]::new('--stdin'                    ,'stdin'               , [CompletionResultType]::ParameterName, 'read file names from stdin, one per line or other separator specified in environment')
            [CompletionResult]::new('--git'                      ,'git'                 , [CompletionResultType]::ParameterName, 'list each file''s Git status, if tracked or ignored')
            [CompletionResult]::new('--git-glyphs'               ,'git-glyphs'          , [CompletionResultType]::ParameterName, 'display Git status with Nerd Font glyphs / icons')
            [CompletionResult]::new('--no-git'                   ,'no-git'              , [CompletionResultType]::ParameterName, 'suppress Git status (always overrides -git, --git-repos, --git-repos-no-status)')
            [CompletionResult]::new('--git-repos'                ,'git-repos'           , [CompletionResultType]::ParameterName, 'list root of git-tree status')
            [CompletionResult]::new('--git-repos-no-status'      ,'git-repos-no-status' , [CompletionResultType]::ParameterName, 'list each git-repos branch name (much faster)')
            [CompletionResult]::new('--print-total'              ,'print-total'         , [CompletionResultType]::ParameterName, 'display total number of entries')

            break
        }
        default {
        #   [CompletionResult]::new('-?'                         ,'help'                , [CompletionResultType]::ParameterName, 'show list of command-line options')
            [CompletionResult]::new('--help'                     ,'help'                , [CompletionResultType]::ParameterName, 'show list of command-line options')
            [CompletionResult]::new('-v'                         ,'v'                   , [CompletionResultType]::ParameterName, 'sort numerically within names, as ls -v does (the default)')
            [CompletionResult]::new('--version'                  ,'version'             , [CompletionResultType]::ParameterName, 'show version of eza')
        #   [CompletionResult]::new('-1'                         ,'oneline'             , [CompletionResultType]::ParameterName, 'display one entry per line')
            [CompletionResult]::new('--oneline'                  ,'oneline'             , [CompletionResultType]::ParameterName, 'display one entry per line')
        #   [CompletionResult]::new('-l'                         ,'long'                , [CompletionResultType]::ParameterName, 'display extended file metadata as a table')
            [CompletionResult]::new('--long'                     ,'long'                , [CompletionResultType]::ParameterName, 'display extended file metadata as a table')
        #   [CompletionResult]::new('-G'                         ,'grid'                , [CompletionResultType]::ParameterName, 'display entries as a grid (default)')
            [CompletionResult]::new('--grid'                     ,'grid'                , [CompletionResultType]::ParameterName, 'display entries as a grid (default)')
        #   [CompletionResult]::new('-x'                         ,'across'              , [CompletionResultType]::ParameterName, 'sort the grid across, rather than downwards')
            [CompletionResult]::new('--across'                   ,'across'              , [CompletionResultType]::ParameterName, 'sort the grid across, rather than downwards')
        #   [CompletionResult]::new('-R'                         ,'recurse'             , [CompletionResultType]::ParameterName, 'recurse into directories')
            [CompletionResult]::new('--recurse'                  ,'recurse'             , [CompletionResultType]::ParameterName, 'recurse into directories')
        #   [CompletionResult]::new('-T'                         ,'tree'                , [CompletionResultType]::ParameterName, 'recurse into directories as a tree')
            [CompletionResult]::new('--tree'                     ,'tree'                , [CompletionResultType]::ParameterName, 'recurse into directories as a tree')
            [CompletionResult]::new('--json'                   ,'json'                  , [CompletionResultType]::ParameterName, 'output file listing and metadata as structured JSON')
            [CompletionResult]::new('--spacing'                ,'spacing'               , [CompletionResultType]::ParameterName, 'number of spaces between columns in grid views')
        #   [CompletionResult]::new('-X'                         ,'dereference'         , [CompletionResultType]::ParameterName, 'dereference symbolic links when displaying information')
            [CompletionResult]::new('--dereference'              ,'dereference'         , [CompletionResultType]::ParameterName, 'dereference symbolic links when displaying information')
        #   [CompletionResult]::new('-F'                         ,'classify'            , [CompletionResultType]::ParameterName, 'display type indicator by file names (always, auto, never)')
            [CompletionResult]::new('--classify'                 ,'classify'            , [CompletionResultType]::ParameterName, 'display type indicator by file names (always, auto, never)')
            [CompletionResult]::new('--color'                    ,'color'               , [CompletionResultType]::ParameterName, 'when to use terminal colours (always, auto, never)')
        #   [CompletionResult]::new('--colour'                   ,'color'               , [CompletionResultType]::ParameterName, 'when to use terminal colours (always, auto, never)')
            [CompletionResult]::new('--color-scale'              ,'colorscale'          , [CompletionResultType]::ParameterName, 'highlight levels of ''field'' distinctly(all, age, size)')
        #   [CompletionResult]::new('--colour-scale'             ,'colorscale'          , [CompletionResultType]::ParameterName, 'highlight levels of ''field'' distinctly(all, age, size)')
            [CompletionResult]::new('--color-scale-mode'         ,'colorscalemode'      , [CompletionResultType]::ParameterName, 'use gradient or fixed colors in --color-scale (fixed, gradient)')
        #   [CompletionResult]::new('--colour-scale-mode'        ,'colorscalemode'      , [CompletionResultType]::ParameterName, 'use gradient or fixed colors in --color-scale (fixed, gradient)')
            [CompletionResult]::new('--icons'                    ,'icons'               , [CompletionResultType]::ParameterName, 'when to display icons (always, auto, never)')
            [CompletionResult]::new('--quotes'                  ,'quotes'             , [CompletionResultType]::ParameterName, 'when to quote filenames (always, auto, never)')
            [CompletionResult]::new('--no-quotes'                ,'noquotes'            , [CompletionResultType]::ParameterName, 'don''t quote file names with spaces')
            [CompletionResult]::new('--short-nix'                ,'shortnix'            , [CompletionResultType]::ParameterName, 'abbreviate Nix store hashes in file names and paths')
            [CompletionResult]::new('--no-symlink-targets'       ,'nosymlinktargets'    , [CompletionResultType]::ParameterName, 'do not show symlink targets')
            [CompletionResult]::new('--summary'                  ,'summary'             , [CompletionResultType]::ParameterName, 'display total summary statistics of entries')
            [CompletionResult]::new('--hyperlink'                ,'hyperlink'           , [CompletionResultType]::ParameterName, 'when to display entries as hyperlinks (always, auto, never)')
            [CompletionResult]::new('--absolute'                 ,'absolute'            , [CompletionResultType]::ParameterName, 'display entries with their absolute path (on, follow, off)')
            [CompletionResult]::new('--follow-symlinks'          ,'followsymlinks'      , [CompletionResultType]::ParameterName, 'drill down into symbolic links that point to directories')
        #   [CompletionResult]::new('-w'                         ,'widths'              , [CompletionResultType]::ParameterName, 'set screen width in columns')
            [CompletionResult]::new('--width'                    ,'widths'              , [CompletionResultType]::ParameterName, 'set screen width in columns')
        #   [CompletionResult]::new('-a'                         ,'filter'              , [CompletionResultType]::ParameterName, 'show hidden and ''dot'' files. Use this twice to also show the ''.'' and ''..'' directories')
            [CompletionResult]::new('--all'                      ,'filter'              , [CompletionResultType]::ParameterName, 'show hidden and ''dot'' files. Use this twice to also show the ''.'' and ''..'' directories')
        #   [CompletionResult]::new('-A'                         ,'filter'              , [CompletionResultType]::ParameterName, 'equivalent to --all; included for compatibility with `ls -A`')
        #   [CompletionResult]::new('--almost-all'               ,'filter'              , [CompletionResultType]::ParameterName, 'equivalent to --all; included for compatibility with `ls -A`')
            [CompletionResult]::new('--show-dotfiles'            ,'filter'              , [CompletionResultType]::ParameterName, 'show dot-prefixed files without showing other hidden files')
            [CompletionResult]::new('--no-system'                ,'filter'              , [CompletionResultType]::ParameterName, 'do not show system files (Windows only)')
            [CompletionResult]::new('--no-hidden-attrib'         ,'filter'              , [CompletionResultType]::ParameterName, 'do not show entries with the Windows hidden attribute (Windows only)')
            [CompletionResult]::new('--no-hidden-links'          ,'filter'              , [CompletionResultType]::ParameterName, 'do not show Windows hidden profile links / junctions (Windows only)')
        #   [CompletionResult]::new('-d'                         ,'filter'              , [CompletionResultType]::ParameterName, 'list directories as files; don''t list their contents')
            [CompletionResult]::new('--treat-dirs-as-files'      ,'filter'              , [CompletionResultType]::ParameterName, 'list directories as files; don''t list their contents')
        #   [CompletionResult]::new('-D'                         ,'filter'              , [CompletionResultType]::ParameterName, 'list only directories')
            [CompletionResult]::new('--only-dirs'                ,'filter'              , [CompletionResultType]::ParameterName, 'list only directories')
        #   [CompletionResult]::new('-f'                         ,'filter'              , [CompletionResultType]::ParameterName, 'list only files')
            [CompletionResult]::new('--only-files'               ,'filter'              , [CompletionResultType]::ParameterName, 'list only files')
        #   [CompletionResult]::new('-L'                         ,'level'               , [CompletionResultType]::ParameterName, 'limit the depth of recursion')
            [CompletionResult]::new('--level'                    ,'level'               , [CompletionResultType]::ParameterName, 'limit the depth of recursion')
        #   [CompletionResult]::new('-r'                         ,'reverse'             , [CompletionResultType]::ParameterName, 'reverse the sort order')
            [CompletionResult]::new('--reverse'                  ,'reverse'             , [CompletionResultType]::ParameterName, 'reverse the sort order')
        #   [CompletionResult]::new('-s'                         ,'sort'                , [CompletionResultType]::ParameterName, 'which field to sort by SORT_FIELD')
            [CompletionResult]::new('--sort'                     ,'sort'                , [CompletionResultType]::ParameterName, 'which field to sort by SORT_FIELD')
            [CompletionResult]::new('--group-directories-first'  ,'gdf'                 , [CompletionResultType]::ParameterName, 'list directories before other files')
            [CompletionResult]::new('--group-directories-last'   ,'gdl'                 , [CompletionResultType]::ParameterName, 'list directories after other files')
        #   [CompletionResult]::new('-I'                         ,'ignore-glob'         , [CompletionResultType]::ParameterName, 'glob patterns (pipe-separated) of files to ignore GLOBS')
            [CompletionResult]::new('--ignore-glob'              ,'ignore-glob'         , [CompletionResultType]::ParameterName, 'glob patterns (pipe-separated) of files to ignore GLOBS')
            [CompletionResult]::new('--ignore-glob-ci'         ,'ignore-glob-ci'        , [CompletionResultType]::ParameterName, 'glob patterns (pipe-separated) of files to ignore, case-insensitively GLOBS')
            [CompletionResult]::new('--cachedir-ignore'           ,'cachedir-ignore'     , [CompletionResultType]::ParameterName, 'ignore directories with a ''CACHEDIR.TAG'' file')
            [CompletionResult]::new('--inspect-archives'         ,'inspectarchives'     , [CompletionResultType]::ParameterName, 'list contents of supported archives in long view')
            [CompletionResult]::new('--ignore-submodule-contents' ,'ignore-submodule-contents', [CompletionResultType]::ParameterName, 'do not list contents of submodules')
            [CompletionResult]::new('-W'                        ,'W'                  , [CompletionResultType]::ParameterName, 'print a tally of hidden and ignored items')
            [CompletionResult]::new('--warn-hidden'            ,'warn-hidden'           , [CompletionResultType]::ParameterName, 'print a tally of hidden and ignored items')
            [CompletionResult]::new('--extended'               ,'extended'              , [CompletionResultType]::ParameterName, 'list each file''s extended attributes and sizes')
            [CompletionResult]::new('--context'                ,'context'               , [CompletionResultType]::ParameterName, 'list each file''s security context')
            [CompletionResult]::new('--tags'                   ,'tags'                  , [CompletionResultType]::ParameterName, 'list each file''s color tags stored in extended attributes')
            [CompletionResult]::new('--no-extended'             ,'no-extended'         , [CompletionResultType]::ParameterName, 'do not show a marker if extended attributes exist')
            [CompletionResult]::new('--git-ignore'               ,'git-ignore'          , [CompletionResultType]::ParameterName, 'ignore files mentioned in ''.gitignore''')
            [CompletionResult]::new('--since'                    ,'since'               , [CompletionResultType]::ParameterName, 'filter and display only files created or modified within duration window')
            [CompletionResult]::new('--print-total'              ,'print-total'         , [CompletionResultType]::ParameterName, 'display total number of entries')
            [CompletionResult]::new('--mime-types'               ,'mimetypes'           , [CompletionResultType]::ParameterName, 'determine file MIME types to better inform styling decisions (unix only)')
            [CompletionResult]::new('--config'                   ,'config'              , [CompletionResultType]::ParameterName, 'load options from specified configuration file')
            [CompletionResult]::new('--no-config'                ,'no-config'           , [CompletionResultType]::ParameterName, 'do not load any configuration files')
            break
        }
        
    } })
        $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property completionText
}
