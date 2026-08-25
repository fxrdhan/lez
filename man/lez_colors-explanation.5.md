% lez_colors-explanation(5) $version

<!-- This is the lez_colors-explanation(5) man page, written in Markdown. -->
<!-- To generate the roff version, run `just man`, -->
<!-- and the man page will appear in the ‘target’ directory. -->

# Name

lez_colors-explanation — more details on customizing lez colors and themes

# lez Color Explanation

lez provides its own built\-in set of file extension mappings that cover a large range of common file extensions, including documents, archives, media, and temporary files. 
Any mappings in the environment variables will override this default set: running lez with `LS_COLORS="*.zip=32"` will turn zip files green but leave the colours of other compressed files alone.

You can also disable this built\-in set entirely by including a
`reset` entry at the beginning of `LEZ_COLORS` or `EZA_COLORS`.
So setting `LEZ_COLORS="reset:*.txt=31"` will highlight only text
files in addition to any styles in `LS_COLORS`; setting `LEZ_COLORS="reset"` will highlight nothing.

## Examples

- Disable the "current user" highlighting: `LEZ_COLORS="uu=0:gu=0"`
- Turn the date column green: `LEZ_COLORS="da=32"`
- Highlight Vagrantfiles: `LEZ_COLORS="Vagrantfile=1;4;33"`
- Override the existing zip colour: `LEZ_COLORS="*.zip=38;5;125"`
- Markdown files a shade of green, log files a shade of grey:
`LEZ_COLORS="*.md=38;5;121:*.log=38;5;248"`

## BUILT\-IN EXTENSIONS

- lez supports bright colours! As supported by most modern 256\-colour terminals, you can now choose from `bright` colour codes when selecting your custom colours in your `LEZ_COLORS` / `EZA_COLORS` environment variable.

- Build (Makefile, Cargo.toml, package.json) are yellow and underlined.
- Images (png, jpeg, gif) are purple.
- Videos (mp4, ogv, m2ts) are a slightly purpler purple.
- Music (mp3, m4a, ogg) is a faint blue.
- Lossless music (flac, alac, wav) is a less faint blue.
- Cryptographic files (asc, enc, p12) are bright green.
- Documents (pdf, doc, dvi) are a fainter green.
- Compressed files (zip, tgz, Z) are red.
- Temporary files (tmp, swp, ~) are dimmed default foreground color.
- Compiled files (class, o, pyc) are yellow. A file is also counted as compiled if it uses a common extension and is
in the same directory as one of its source files: styles.css will count as compiled when next to styles.less or styles.sass, and scripts.js when next to scripts.ts or scripts.coffee.
- Source files (cpp, js, java) are bright yellow.


## Theme Configuration file

Now you can specify these options and more in a `theme.yml` file with convenient syntax for defining your styles.

Set `LEZ_CONFIG_DIR` or `EZA_CONFIG_DIR` to specify which directory you would like lez to look for your `theme.yml` file,
otherwise lez will look for `$XDG_CONFIG_HOME/lez/theme.yml` or `$XDG_CONFIG_HOME/eza/theme.yml`.


These are the available options:

LIST OF THEME OPTIONS
=====================

```yaml
filekinds:
  normal
  directory
  symlink
  pipe
  block_device
  char_device
  socket
  special
  executable
  mount_point

perms:
  user_read
  user_write
  user_execute_file
  user_execute_other
  group_read
  group_write
  group_execute
  other_read
  other_write
  other_execute
  special_user_file
  special_other
  attribute

size:
  major
  minor
  number_byte
  number_kilo
  number_mega
  number_giga
  number_huge
  unit_byte
  unit_kilo
  unit_mega
  unit_giga
  unit_huge

users:
  user_you
  user_root
  user_other
  group_yours
  group_other
  group_root

links:
  normal
  multi_link_file

git:
  new
  modified
  deleted
  renamed
  ignored
  conflicted

git_repo:
  branch_main
  branch_other
  git_clean
  git_dirty

security_context:
  none:
  selinux:
    colon
    user
    role
    typ
    range

file_type:
  image
  video
  music
  crypto
  document
  compressed
  temp
  compiled
  build
  source

punctuation:

date:

inode:

blocks:

header:

octal:

flags:

control_char:

broken_symlink:

broken_path_overlay:

```

Each of those fields/sub fields can have the following styling properties defined beneath it:

```yaml
    foreground: Blue
    background: null
    is_bold: false
    is_dimmed: false
    is_italic: false
    is_underline: false
    is_blink: false
    is_reverse: false
    is_hidden: false
    is_strikethrough: true
    prefix_with_reset: false
```

Example:

```yaml

file_type:
  image:
    foreground: Blue
    is_italic: true
date:
  foreground: White

security_context:
  selinux:
    role:
      is_hidden: true
```

Icons can now be customized as well in the `filenames`, `extensions`, `directorynames`, and `mimetypes` fields:

```yaml

filenames:
  # Just change the icon glyph
  Cargo.toml: {icon: {glyph: 🦀}}
  Cargo.lock: {icon: {glyph: 🦀}}

extensions:
  rs: {  filename: {foreground: Red}, icon: {glyph: 🦀}}
  # Default fallback icon overrides (dot-prefixed sentinel keys)
  .default_file: { icon: {glyph: 📄} }
  .default_file_unknown: { icon: {glyph: ❓} }
  .default_directory: { icon: {glyph: 📁} }
  .default_directory_empty: { icon: {glyph: 📂} }

mimetypes:
  application/pdf: { filename: {foreground: Red}, icon: {glyph: 📕} }
  text/x-rust: { filename: {foreground: Yellow} }

```

You can customize default fallback icons for unmapped files and directories under the `extensions` section using reserved dot-prefixed keys:
- `.default_file`: Default glyph and style for files with an unmapped extension.
- `.default_file_unknown`: Default glyph and style for extensionless files (falls back to `.default_file` if unset).
- `.default_directory`: Default glyph and style for non-empty or generic directories.
- `.default_directory_empty`: Default glyph and style for empty directories (falls back to `.default_directory` if unset).

**NOTES:** 

Not all glyphs support changing colors.

If your theme is not working properly, double check the syntax in the config file, as
a syntax issue can cause multiple properties to not be applied.

You must name the file `theme.yml`, no matter the directory you specify.


## See also

**lez**(1), **lez_colors**(5)
