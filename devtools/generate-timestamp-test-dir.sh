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

# generate files of various age
touch ./now

if date -v-13m >/dev/null 2>&1; then
    # BSD date (macOS)
    touch -d "$(date -v-13m "+%Y-%m-%dT%H:%M:%S")" ./13_month
elif date -d "13 month ago" >/dev/null 2>&1; then
    # GNU date (Linux)
    touch -d "$(date -d "13 month ago" "+%Y-%m-%dT%H:%M:%S")" ./13_month
else
    python3 -c "import time, os; t = time.time(); os.utime(\"./13_month\", (t - 395*86400, t - 395*86400))"
fi
