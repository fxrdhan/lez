#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
#
# Standalone universal installer for lez (Linux & macOS)
# Usage: curl -fsSL https://raw.githubusercontent.com/fxrdhan/lez/main/packaging/install.sh | bash

set -euo pipefail

REPO="fxrdhan/lez"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM="unknown-linux-gnu"; EXT="tar.gz" ;;
    Darwin*)    PLATFORM="apple-darwin"; EXT="tar.gz" ;;
    *)          echo "Error: Unsupported operating system: ${OS}" >&2; exit 1 ;;
esac

# Detect Architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64|amd64)   ARCH_TARGET="x86_64" ;;
    aarch64|arm64)  ARCH_TARGET="aarch64" ;;
    *)              echo "Error: Unsupported CPU architecture: ${ARCH}" >&2; exit 1 ;;
esac

TARGET="${ARCH_TARGET}-${PLATFORM}"

echo "==> Fetching latest release for ${TARGET}..."
RELEASE_JSON=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" || echo "")
LATEST_TAG=$(grep '"tag_name":' <<< "${RELEASE_JSON}" | sed -E 's/.*"([^"]+)".*/\1/' || echo "")

if [ -z "${LATEST_TAG}" ]; then
    # Fallback to Cargo if no releases found yet
    if command -v cargo >/dev/null 2>&1; then
        echo "==> No release assets found, building via cargo install..."
        cargo install --git "https://github.com/${REPO}.git" --locked
        echo "==> Successfully installed lez via cargo!"
        exit 0
    else
        echo "Error: Could not determine latest release tag and cargo is not installed." >&2
        exit 1
    fi
fi

ARCHIVE_NAME="lez_${TARGET}.${EXT}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST_TAG}/${ARCHIVE_NAME}"

# A release carries prebuilt binaries for some targets only (see INSTALL.md);
# Linux on ARM64, for one, has none. Build the release's tag with Cargo there,
# rather than fail on a download that is not there.
if ! grep -qF "\"${DOWNLOAD_URL}\"" <<< "${RELEASE_JSON}"; then
    if command -v cargo >/dev/null 2>&1; then
        echo "==> ${LATEST_TAG} has no prebuilt binary for ${TARGET}, building it via cargo install..."
        cargo install --git "https://github.com/${REPO}.git" --tag "${LATEST_TAG}" --locked
        echo "==> Successfully installed lez via cargo!"
        exit 0
    fi
    echo "Error: ${LATEST_TAG} has no prebuilt binary for ${TARGET}, and cargo is not installed." >&2
    echo "Install Rust from https://rustup.rs and run this installer again, or see" >&2
    echo "https://github.com/${REPO}/blob/main/INSTALL.md for other ways to install lez." >&2
    exit 1
fi

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "==> Downloading ${DOWNLOAD_URL}..."
curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}"

echo "==> Extracting binary..."
tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "${TMP_DIR}"

mkdir -p "${INSTALL_DIR}"
install -m 755 "${TMP_DIR}/lez" "${INSTALL_DIR}/lez"

echo "==> Successfully installed lez to ${INSTALL_DIR}/lez!"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "Note: Make sure ${INSTALL_DIR} is in your PATH. You can add it with:"
    echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
fi
