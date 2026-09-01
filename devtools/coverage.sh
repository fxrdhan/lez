#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2

set -euo pipefail

# 1. Discover LLVM Tools across macOS (Xcode/CommandLineTools), Linux, and Nix
if [ -z "${LLVM_COV:-}" ]; then
    if command -v xcrun >/dev/null 2>&1 && xcrun -find llvm-cov >/dev/null 2>&1; then
        LLVM_COV="$(xcrun -find llvm-cov)"
        export LLVM_COV
    elif command -v llvm-cov >/dev/null 2>&1; then
        LLVM_COV="$(command -v llvm-cov)"
        export LLVM_COV
    fi
fi

if [ -z "${LLVM_PROFDATA:-}" ]; then
    if command -v xcrun >/dev/null 2>&1 && xcrun -find llvm-profdata >/dev/null 2>&1; then
        LLVM_PROFDATA="$(xcrun -find llvm-profdata)"
        export LLVM_PROFDATA
    elif command -v llvm-profdata >/dev/null 2>&1; then
        LLVM_PROFDATA="$(command -v llvm-profdata)"
        export LLVM_PROFDATA
    fi
fi

# 2. Validate cargo-llvm-cov installation
if ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "Error: cargo-llvm-cov is not installed."
    echo "Install via:"
    echo "  macOS (Homebrew): brew install cargo-llvm-cov"
    echo "  Cargo:            cargo install cargo-llvm-cov --locked"
    echo "  Nix:              nix develop (included in devShell)"
    exit 1
fi

ACTION="${1:-report}"
shift || true

case "$ACTION" in
    report|run)
        echo "==> Running test suite with LLVM source-based code coverage..."
        cargo llvm-cov nextest --features git,inspect-archives --workspace "$@"
        cargo llvm-cov report
        ;;
    html)
        echo "==> Generating HTML code coverage report..."
        cargo llvm-cov nextest --features git,inspect-archives --workspace --html --output-dir target/llvm-cov/html "$@"
        echo "==> HTML report available at: target/llvm-cov/html/index.html"
        ;;
    lcov)
        echo "==> Generating LCOV coverage report (lcov.info)..."
        cargo llvm-cov nextest --features git,inspect-archives --workspace --lcov --output-path lcov.info "$@"
        echo "==> LCOV report written to: lcov.info"
        ;;
    summary)
        echo "==> Generating Markdown summary..."
        REPORT=$(cargo llvm-cov report)
        echo "$REPORT"
        if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
            {
                echo "## 📊 Code Coverage Summary"
                echo '```text'
                echo "$REPORT" | tail -n 25
                echo '```'
            } >> "$GITHUB_STEP_SUMMARY"
        fi
        ;;
    gate)
        THRESHOLD="${1:-70}"
        echo "==> Verifying code coverage gate (minimum: ${THRESHOLD}%)..."
        REPORT="$(cargo llvm-cov report)"
        TOTAL_LINE=$(echo "$REPORT" | grep -E '^TOTAL[[:space:]]+')
        
        if [ -z "$TOTAL_LINE" ]; then
            echo "Error: Could not find TOTAL line in coverage report."
            exit 1
        fi
        
        # Extract the line coverage percentage (first percentage column)
        PERCENT=$(echo "$TOTAL_LINE" | awk '{print $4}' | tr -d '%')
        echo "==> Current Total Line Coverage: ${PERCENT}% (Threshold: ${THRESHOLD}%)"
        
        # Compare using python or awk floating point comparison
        PASSED=$(python3 -c "print(1 if float('${PERCENT}') >= float('${THRESHOLD}') else 0)")
        if [ "$PASSED" -eq 1 ]; then
            echo "✅ Coverage gate passed: ${PERCENT}% >= ${THRESHOLD}%"
        else
            echo "❌ Coverage gate failed: ${PERCENT}% is below ${THRESHOLD}% threshold!"
            exit 1
        fi
        ;;
    *)
        echo "Usage: $0 [report|html|lcov|summary|gate <threshold>] [extra cargo-llvm-cov args...]"
        exit 1
        ;;
esac
