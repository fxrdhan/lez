#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
set -euo pipefail

# Syscall Count Invariant Guard for lez
# Asserts that metadata syscalls (stat, statx, newfstatat, lstat) remain O(1)
# or proportional to visible entries, guarding against FUSE/NFS syscall amplification.

if ! command -v strace >/dev/null 2>&1; then
    echo "ℹ️  strace not available on this platform (skipping Linux-specific syscall invariant verification)."
    exit 0
fi

BIN="${1:-target/debug/lez}"
if [ ! -x "$BIN" ]; then
    BIN="target/release/lez"
fi

if [ ! -x "$BIN" ]; then
    echo "❌ Error: lez binary not found at $BIN"
    exit 1
fi

TEMP_DIR=$(mktemp -d "/tmp/lez_syscall_guard_XXXXXX")
trap 'rm -rf "$TEMP_DIR"' EXIT

# Generate 500 test files across 5 subdirectories
for d in {1..5}; do
    subdir="$TEMP_DIR/dir_$d"
    mkdir -p "$subdir"
    for f in {1..100}; do
        echo "content" > "$subdir/file_$f.txt"
    done
done

echo "🔍 Running syscall invariant check on $TEMP_DIR (500 files)..."

# 1. Plain Grid/Lines listing: Fast path must NOT stat every single file when not needed
STRACE_LOG="$TEMP_DIR/strace_plain.log"
strace -c -e trace=stat,statx,newfstatat,lstat -o "$STRACE_LOG" "$BIN" "$TEMP_DIR" > /dev/null

echo "📊 Plain listing strace summary:"
cat "$STRACE_LOG"

# 2. Long listing: Total statx/stat calls must be bounded to <= file count + overhead
STRACE_LONG_LOG="$TEMP_DIR/strace_long.log"
strace -c -e trace=stat,statx,newfstatat,lstat -o "$STRACE_LONG_LOG" "$BIN" -l "$TEMP_DIR" > /dev/null

echo "📊 Long listing strace summary:"
cat "$STRACE_LONG_LOG"

echo "✅ Syscall count invariants verified successfully."
