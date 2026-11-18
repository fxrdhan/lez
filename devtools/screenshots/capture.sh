#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
#
# The part of devtools/screenshots/render.sh that runs in the container. As
# root, it builds lez from /src and adds the user `demo`. As `demo`, it
# builds the demo directory and records each window's transcript, colors
# included, by running its commands in a pseudo-terminal.
#
# Takes the width in columns of the left and the right window, and reads
# /work/lez.tar and /work/mtimes.tsv, which render.sh writes. Writes
# /work/left.ansi and /work/right.ansi.

set -euo pipefail

if [ "$(id -u)" -eq 0 ]; then
    left=$1
    right=$2
    # rust-toolchain.toml asks for the MSRV, which the image may not have.
    # RUSTUP_TOOLCHAIN outranks it, and RUST_VERSION is the image's own.
    CARGO_TARGET_DIR=/work/target RUSTUP_TOOLCHAIN="$RUST_VERSION" \
        cargo build --locked --quiet --manifest-path /src/Cargo.toml
    install -m 0755 /work/target/debug/lez /usr/local/bin/lez
    useradd --create-home --shell /bin/bash demo
    # What the `result` symlink of a `nix build` points to.
    version=$(sed -n 's/^version = "\(.*\)"$/\1/p' /src/Cargo.toml | head -n 1)
    hash=$(printf 'lez-%s' "$version" | sha256sum | head -c 32 | tr e w)
    store="/nix/store/$hash-lez-$version"
    mkdir -p "$store/bin"
    ln -s /usr/local/bin/lez "$store/bin/lez"
    exec runuser -u demo -- env -i HOME=/home/demo PATH=/usr/local/bin:/usr/bin:/bin \
        LANG=C.UTF-8 TERM=xterm-256color TZ=UTC \
        bash "$0" record "$store" "$left" "$right"
fi

[ "${1:-}" = record ] || { echo "usage: run as root" >&2; exit 2; }
store=$2
left=$3
right=$4

git() {
    command git -c user.name=demo -c user.email=demo@example.com \
        -c init.defaultBranch=main -c advice.detachedHead=false "$@"
}

# Dates a path relative to now, e.g. `ago 3h README.md`.
ago() {
    local amount=${1%[mhd]} unit
    case $1 in
        *m) unit=minutes ;;
        *h) unit=hours ;;
        *d) unit=days ;;
    esac
    shift
    touch -h -d "-$amount $unit" -- "$@"
}

repo() {
    mkdir -p "$1"
    git -C "$1" init -q
}

commit() {
    git -C "$1" add -A
    git -C "$1" commit -q -m "${2:-Update}"
}

cd "$HOME"
mkdir projects
cd projects

# lez: some of this repository's files, as a fresh repository with work in
# progress.
repo lez
tar -xf /work/lez.tar -C lez
commit lez "Import lez"
printf '\n<!-- Draft: a section on themes. -->\n' >> lez/README.md
printf '\n// Cache the directory listing.\n' >> lez/src/fs/dir.rs
git -C lez add src/fs/dir.rs
printf '\n// Read the index once per repository.\n' >> lez/src/fs/feature/git.rs
printf -- '- Screenshots for the README.\n' > lez/TODO.md
mkdir -p lez/target/debug
cp /usr/local/bin/lez lez/target/debug/lez
ln -s "$store" lez/result
# The dates git last changed each file, then the work in progress.
while IFS=$'\t' read -r when path; do
    touch -h -d "@$when" -- "lez/$path"
done < /work/mtimes.tsv
ago 2h lez/README.md
ago 50m lez/src/fs/dir.rs
ago 20m lez/src/fs/feature/git.rs
ago 5m lez/TODO.md
ago 1h lez/target/debug/lez lez/target/debug lez/target lez/result

# The repositories next to it: one clean, one with changes on a branch.
repo dotfiles
mkdir -p dotfiles/nvim
printf 'set -o vi\n' > dotfiles/bashrc
printf -- '-- Neovim\n' > dotfiles/nvim/init.lua
commit dotfiles "Add dotfiles"

repo website
mkdir -p website/content
printf '# Hello\n' > website/content/index.md
commit website "Add the home page"
git -C website switch -q -c redesign
printf 'body { color: #333; }\n' > website/style.css

mkdir notes
printf '# Ideas\n' > notes/ideas.md

ago 5m lez
ago 4h website/style.css website
ago 9d dotfiles/nvim/init.lua dotfiles/bashrc dotfiles/nvim dotfiles
ago 2d notes/ideas.md notes

# Records each command after a prompt, as an interactive shell would show
# it, then ends with an empty prompt and the cursor. The prompt is in the
# style of Starship's default one, with its Nerd Font folder and branch
# icons.
prompt() {
    printf '\e[1;36m\uf07b %s\e[0m' "$1"
    if [ -n "$2" ]; then
        printf ' on \e[1;35m\ue0a0 %s\e[0m' "$2"
    fi
    printf ' \e[1;32m\u276f\e[0m '
}

record() {
    local columns=$1 dir=$2 label=$3 branch=$4 command
    shift 4
    for command in "$@"; do
        prompt "$label" "$branch"
        printf '%s\n' "$command"
        (cd "$dir" && script -qefc "stty cols $columns rows 50; $command" /dev/null) | tr -d '\r'
        printf '\n'
    done
    prompt "$label" "$branch"
    printf '\e[7m \e[0m\n'
}

record "$left" "$HOME/projects" projects "" \
    "lez -l --icons --git --git-repos --header" \
    "lez -l --icons --git -T --header lez/src/fs" \
    > /work/left.ansi

record "$right" "$HOME/projects/lez" lez main \
    "lez -l --icons --git -o" \
    "lez -l --icons --grid --time-style relative" \
    "lez --icons" \
    > /work/right.ansi
