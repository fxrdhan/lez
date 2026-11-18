#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
#
# Regenerates docs/images/screenshots.png, the image at the top of
# README.md: two terminal windows side by side.
#
#   dark, left    lez -l --icons --git --git-repos --header
#                 lez -l --icons --git -T --header lez/src/fs
#   light, right  lez -l --icons --git -o
#                 lez -l --icons --grid --time-style relative
#                 lez --icons
#
# capture.sh builds lez from HEAD and runs these commands in a throwaway
# Linux container, as the user `demo`, in a demo directory made from some of
# this repository's files. So no user, host or path of the machine that
# renders them can reach the image. ansi2svg.py draws the transcripts as
# windows, and resvg rasterizes them with JetBrains Mono and the Nerd Font
# symbols alone.
#
# Needs Docker (or Podman, with CONTAINER=podman), Python 3 and Nix, which
# provides resvg, oxipng and the fonts from the nixpkgs flake.lock pins.
# Run it from a checkout with history: each file is dated by the last commit
# that changed it.

set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"
work=target/screenshots
engine=${CONTAINER:-docker}
image=${LEZ_SCREENSHOTS_IMAGE:-rust:1-bookworm}
# The width of each window, in columns. The right one fits two columns of
# grid-details.
left=66
right=110

mkdir -p "$work/cargo-registry"

# The demo copy of lez: these paths as committed at HEAD.
paths=(.gitignore benches build.rs Cargo.lock Cargo.toml CHANGELOG.md completions devtools
    docs flake.lock flake.nix justfile LICENSE.txt man README.md
    rust-toolchain.toml src tests)
git archive --format=tar -o "$work/lez.tar" HEAD -- "${paths[@]}"
# The dates of what the screenshots show: the top level and the tree.
{
    printf '%s\n' "${paths[@]}"
    git ls-tree -r -t --name-only HEAD -- src/fs
} | while IFS= read -r path; do
    printf '%s\t%s\n' "$(git log -1 --format=%ct HEAD -- "$path")" "$path"
done > "$work/mtimes.tsv"

"$engine" run --rm \
    -v "$root:/src:ro" \
    -v "$root/$work:/work" \
    -v "$root/$work/cargo-registry:/usr/local/cargo/registry" \
    "$image" bash /src/devtools/screenshots/capture.sh "$left" "$right"

package() {
    nix build --inputs-from . "nixpkgs#$1" --no-link --print-out-paths
}
resvg=$(package resvg)/bin/resvg
oxipng=$(package oxipng)/bin/oxipng
mono=$(package jetbrains-mono)/share/fonts/truetype
symbols=$(package nerd-fonts.symbols-only)/share/fonts/truetype/NerdFonts/Symbols

# The last argument of each window is its title, as a terminal would show it.
# shellcheck disable=SC2088
python3 devtools/screenshots/ansi2svg.py \
    --window "$work/left.ansi" dark "$left" "~/projects" \
    --window "$work/right.ansi" light "$right" "~/projects/lez" \
    > "$work/screenshots.svg"
# Icons come from the symbols font, as a terminal falls back to it.
"$resvg" --quiet --skip-system-fonts \
    --use-font-file "$mono/JetBrainsMono-Regular.ttf" \
    --use-font-file "$mono/JetBrainsMono-Bold.ttf" \
    --use-font-file "$mono/JetBrainsMono-Italic.ttf" \
    --use-font-file "$symbols/SymbolsNerdFont-Regular.ttf" \
    --font-family "JetBrains Mono" --monospace-family "JetBrains Mono" \
    --zoom 1.5 "$work/screenshots.svg" docs/images/screenshots.png
"$oxipng" --quiet --opt max --strip safe docs/images/screenshots.png
