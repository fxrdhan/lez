#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
#
# Automated Linux syscall-budget regression guard.
# Verifies four O(1) vs O(N) syscall invariants under `strace -c`:
#   1. Plain flat listing of 500 regular files does not stat each entry.
#   2. Coloured + icon flat listing of 500 non-executable files (`.png`)
#      resolves extension styles before `is_executable_file` (PR #87).
#   3. Listing 200 directories with `LEZ_NO_EMPTY_DIR_ICON=1 --icons=always`
#      does not open/read each child directory (`getdents64`).
#   4. A listing that shows no time reads no directory but the one listed:
#      finding the local time zone's name walks every directory of the
#      zoneinfo database (about 40 `getdents64` calls), so it waits for a
#      view with a time column.

set -euo pipefail

BIN="${1:-target/debug/lez}"
if [ ! -x "$BIN" ]; then
    echo "error: lez binary not found or not executable at: $BIN" >&2
    exit 1
fi

if ! command -v strace >/dev/null 2>&1; then
    echo "error: strace is required to run syscall-budget checks" >&2
    exit 1
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/lez-syscall-budget.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/plain" "$WORK/images" "$WORK/dirs" "$WORK/empty-cfg"
for ((i = 1; i <= 500; i++)); do
    : >"$WORK/plain/file_${i}.txt"
    : >"$WORK/images/photo_${i}.png"
done
for ((i = 1; i <= 200; i++)); do
    mkdir "$WORK/dirs/dir_${i}"
done

sum_syscalls() {
    local pattern="$1"
    local log_file="$2"
    awk -v pat="$pattern" '
        $NF ~ pat && $4 ~ /^[0-9]+$/ { total += $4 }
        END { print total + 0 }
    ' "$log_file"
}

check_budget() {
    local label="$1"
    local pattern="$2"
    local max_calls="$3"
    shift 3

    local log_file="$WORK/strace.log"
    LEZ_CONFIG_DIR="$WORK/empty-cfg" \
        strace -c -f -o "$log_file" -- "$BIN" "$@" >/dev/null

    local actual
    actual="$(sum_syscalls "$pattern" "$log_file")"
    echo "==> [$label] matched($pattern) = ${actual} (budget <= ${max_calls})"
    if [ "$actual" -gt "$max_calls" ]; then
        echo "FAIL: [$label] exceeded syscall budget (${actual} > ${max_calls})" >&2
        cat "$log_file" >&2
        exit 1
    fi
}

STAT_PATTERN='^(stat|lstat|newfstatat|statx)$'
GETDENTS_PATTERN='^(getdents|getdents64)$'

check_budget \
    "plain-flat-500-files" \
    "$STAT_PATTERN" \
    30 \
    --no-config --color=never --icons=never "$WORK/plain"

check_budget \
    "plain-flat-500-files-getdents" \
    "$GETDENTS_PATTERN" \
    10 \
    --no-config --color=never --icons=never "$WORK/plain"

check_budget \
    "coloured-icons-500-png-files" \
    "$STAT_PATTERN" \
    30 \
    --no-config --color=always --icons=always "$WORK/images"

(
    export LEZ_NO_EMPTY_DIR_ICON=1
    check_budget \
        "no-empty-dir-icon-200-dirs-getdents" \
        "$GETDENTS_PATTERN" \
        10 \
        --no-config --color=never --icons=always "$WORK/dirs"
    check_budget \
        "no-empty-dir-icon-200-dirs-stat" \
        "$STAT_PATTERN" \
        30 \
        --no-config --color=never --icons=always "$WORK/dirs"
)

echo "All syscall-budget guards passed."
