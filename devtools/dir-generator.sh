#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2024 Christina Sørensen
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <output_dir>"
    exit 1
fi

TARGET="$1"
rm -rf "$TARGET"
mkdir -p "$TARGET"
cd "$TARGET" || exit 1

# Portable epoch touch (works on both GNU and BSD/macOS touch)
touch_epoch() {
    TZ=UTC touch -t 197001010000.00 "$@"
}

# Optional groupadd for environments with root / groupadd
if [ "$(id -u)" -eq 0 ] && command -v groupadd >/dev/null 2>&1; then
    groupadd -f eza_test 2>/dev/null || true
fi

# BEGIN grid
mkdir -p grid
(
    cd grid || exit 1
    mkdir $(seq -f '%04g' 1 1000)
    seq 0001 1000 | split -l 1 -a 3 -d - file_
    touch_epoch ./*
)
# END grid

# BEGIN git
mkdir -p git
(
    cd git || exit 1
    mkdir $(seq -f '%03g' 1 10)
    for f in ./*; do
        (
            cd "$f" || exit 1
            git init -q -b master 2>/dev/null || git init -q
            seq 01 10 | split -l 1 -a 3 -d - file_
            touch_epoch ./*
        )
    done
)
# END git

# BEGIN test_root
if [ "$(id -u)" -eq 0 ]; then
    mkdir -p root/empty
    chmod 777 root
fi
# END test_root

# BEGIN mknod
mkdir -p specials
if [ "$(id -u)" -eq 0 ] && command -v mknod >/dev/null 2>&1; then
    mknod specials/block-device b 3 60 2>/dev/null || true
    mknod specials/char-device c 14 40 2>/dev/null || true
    mknod specials/named-pipe p 2>/dev/null || true
fi
# END mknod

# BEGIN test_symlinks
mkdir -p symlinks
touch symlinks/file
touch_epoch symlinks/file
ln -sf file symlinks/symlink
ln -sf symlink symlinks/symlink2
mkdir -p symlinks/dir
ln -sf dir symlinks/symlink3
ln -sf pipitek symlinks/symlink4
touch "symlinks/ lorem ipsum"
touch_epoch "symlinks/ lorem ipsum"
# END test_symlinks

# BEGIN test_perms
mkdir -p perms
touch perms/file perms/file2
touch_epoch perms/file perms/file2
chmod 777 perms/file
chmod 001 perms/file2
# END test_perms

# BEGIN test_group
mkdir -p group
touch group/file
touch_epoch group/file
if [ "$(id -u)" -eq 0 ] && command -v chgrp >/dev/null 2>&1; then
    chgrp eza_test group/file 2>/dev/null || true
fi
# END test_group

# BEGIN test_size
mkdir -p size
touch size/1M size/1K size/1B size/1337
touch_epoch size/1M size/1K size/1B size/1337
dd if=/dev/zero of=size/1M bs=1 count=0 seek=1048576 2>/dev/null || dd if=/dev/zero of=size/1M bs=1 count=0 seek=1M 2>/dev/null
dd if=/dev/zero of=size/1K bs=1 count=0 seek=1024 2>/dev/null || dd if=/dev/zero of=size/1K bs=1 count=0 seek=1K 2>/dev/null
dd if=/dev/zero of=size/1B bs=1 count=0 seek=1 2>/dev/null
dd if=/dev/zero of=size/1337 bs=1 count=0 seek=1337 2>/dev/null
# END test_size

# BEGIN test_time
mkdir -p time
TZ=UTC touch -t 197001010000.00 time/epoch
TZ=UTC touch -t 197001010000.01 time/1s
TZ=UTC touch -t 197001010001.00 time/1m
TZ=UTC touch -t 197001010100.00 time/1h
TZ=UTC touch -t 197001020000.00 time/1d
TZ=UTC touch -t 197101010000.00 time/1y
# END test_time

# BEGIN test_icons
mkdir -p icons
touch icons/file icons/go.go icons/rust.rs icons/c.c icons/c++.cpp \
      icons/python.py icons/java.java icons/javascript.js icons/html.html \
      icons/css.css icons/php.php icons/ruby.rb icons/shell.sh \
      icons/unknown.unknown icons/man.1 icons/marked.md
touch_epoch icons/*
# END test_icons

# BEGIN test_dirs-ext
mkdir -p dirs-ext
mkdir -p dirs-ext/test dirs-ext/abc dirs-ext/01.city dirs-ext/02.apple
touch dirs-ext/a.txt dirs-ext/abc.mp3 dirs-ext/ab
touch_epoch dirs-ext/*
# END test_dirs-ext

# BEGIN set date
touch_epoch ./*
# END set date

# BEGIN clean xattrs
if command -v xattr >/dev/null 2>&1; then
    xattr -rc "$TARGET" 2>/dev/null || true
fi
# END clean xattrs
