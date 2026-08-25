#!/bin/bash -e

REPO_URL="https://github.com/fxrdhan/lez"
NAME="lez"
DESTDIR=/usr/bin
DOCDIR=/usr/share/man/

COMMIT=$(git rev-parse --abbrev-ref HEAD)

TAG=$(git describe --tags "$(git rev-list --tags --max-count=1)")
if [ -n "$1" ]; then
    TAG=$1
fi

VERSION=${TAG:1}

echo "checkout tag ${TAG}"
git checkout --quiet "${TAG}"

echo "build man pages"
just man

declare -A TARGETS
TARGETS["amd64"]="x86_64-unknown-linux-musl"
TARGETS["arm64"]="aarch64-unknown-linux-gnu"
TARGETS["armhf"]="arm-unknown-linux-gnueabihf"

echo "download release notes"
RELEASE_NOTES=$(curl -s "${REPO_URL}/releases/tag/${TAG}")

for ARCH in "${!TARGETS[@]}"; do
    echo "building ${ARCH} package:"

    DEB_TMP_DIR="${NAME}_${VERSION}_${ARCH}"
    DEB_PACKAGE="${NAME}_${VERSION}_${ARCH}.deb"

    TARGET=${TARGETS[$ARCH]}
    echo " -> downloading ${TARGET} archive"
    wget -q -O "${ARCH}.tar.gz" "${REPO_URL}/releases/download/${TAG}/${NAME}_${TARGET}.tar.gz"

    echo " -> verifying ${TARGET} archive"
    CHECKSUM=$(md5sum "${ARCH}.tar.gz" | cut -d ' ' -f 1)
    echo "    checksum: ${CHECKSUM}"
    grep -q "${CHECKSUM}" <<< "${RELEASE_NOTES}" \
        || (echo "checksum mismatch" && exit 1)
    echo "    checksum ok"

    echo " -> creating directory structure"
    mkdir -p "${DEB_TMP_DIR}"
    mkdir -p "${DEB_TMP_DIR}${DESTDIR}"
    mkdir -p "${DEB_TMP_DIR}${DOCDIR}"
    mkdir -p "${DEB_TMP_DIR}${DOCDIR}/man1"
    mkdir -p "${DEB_TMP_DIR}${DOCDIR}/man5"
    mkdir -p "${DEB_TMP_DIR}/DEBIAN"
    mkdir -p "${DEB_TMP_DIR}/usr/share/doc/${NAME}"
    mkdir -p "${DEB_TMP_DIR}/usr/share/bash-completion/completions/"
    mkdir -p "${DEB_TMP_DIR}/usr/share/fish/vendor_completions.d/"
    mkdir -p "${DEB_TMP_DIR}/usr/share/zsh/vendor-completions/"
    chmod 755 -R "${DEB_TMP_DIR}"

    echo " -> extract executable"
    tar -xzf "${ARCH}.tar.gz"
    cp ${NAME} "${DEB_TMP_DIR}${DESTDIR}"
    chmod 755 "${DEB_TMP_DIR}${DESTDIR}/${NAME}"

    echo " -> compress man pages"
    gzip -cn9 target/man/lez.1 > "${DEB_TMP_DIR}${DOCDIR}man1/lez.1.gz"
    gzip -cn9 target/man/lez_colors.5 > "${DEB_TMP_DIR}${DOCDIR}man5/lez_colors.5.gz"
    gzip -cn9 target/man/lez_colors-explanation.5 > "${DEB_TMP_DIR}${DOCDIR}man5/lez_colors-explanation.5.gz"
    chmod 644 "${DEB_TMP_DIR}${DOCDIR}"/**/*.gz

    echo " -> copy completions"
    cp completions/bash/lez "${DEB_TMP_DIR}/usr/share/bash-completion/completions/"
    cp completions/fish/lez.fish "${DEB_TMP_DIR}/usr/share/fish/vendor_completions.d/"
    cp completions/zsh/_lez "${DEB_TMP_DIR}/usr/share/zsh/vendor-completions/"

    echo " -> create control file"
    touch "${DEB_TMP_DIR}/DEBIAN/control"
    cat > "${DEB_TMP_DIR}/DEBIAN/control" <<EOM
Package: ${NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: libc6
Maintainer: Firdaus Arif R <firdausarief65@gmail.com>
Description: Modern replacement for ls
 lez is a modern, fast, and feature-rich replacement for ls.  It uses colours
 for information by default, helping you distinguish between many types of
 files, such as whether you are the owner, or in the owning group.
 .
 It also has extra features not present in the original ls, such as viewing the
 Git status for a directory, lines-of-code counting with --code, structured
 JSON output with --json, and recursing into directories with a tree view.
EOM
    chmod 644 "${DEB_TMP_DIR}/DEBIAN/control"

    echo " -> copy changelog"
    cp CHANGELOG.md "${DEB_TMP_DIR}/usr/share/doc/${NAME}/changelog"
    gzip -cn9 "${DEB_TMP_DIR}/usr/share/doc/${NAME}/changelog" > "${DEB_TMP_DIR}/usr/share/doc/${NAME}/changelog.gz"
    rm "${DEB_TMP_DIR}/usr/share/doc/${NAME}/changelog"
    chmod 644 "${DEB_TMP_DIR}/usr/share/doc/${NAME}/changelog.gz"

    echo " -> create copyright file"
    touch "${DEB_TMP_DIR}/usr/share/doc/${NAME}/copyright"
    cat > "${DEB_TMP_DIR}/usr/share/doc/${NAME}/copyright" << EOM
Format: http://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: ${NAME}
Upstream-Contact: https://github.com/fxrdhan/lez/issues
Source: https://github.com/fxrdhan/lez/releases

Files: *
License: EUPL-1.2
Copyright: 2026 fxrdhan
           2023-2024 Christina Sørensen and eza contributors

Files: debian/*
License: EUPL-1.2
Copyright: 2026 fxrdhan

License: EUPL-1.2
 lez is a fork of eza, which is a fork of exa.  It is licensed under the
 European Union Public Licence v1.2; the full text is shipped in the source
 tree as LICENSE.txt and is also available at
 <https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12>.
 .
 Portions originating in exa (Copyright 2014 Benjamin Sago) remain under the
 MIT licence, as recorded in the SPDX headers of the individual files.
EOM
    chmod 644 "${DEB_TMP_DIR}/usr/share/doc/${NAME}/copyright"

    echo " -> build ${ARCH} package"
    dpkg-deb --build --root-owner-group "${DEB_TMP_DIR}" > /dev/null

    echo " -> cleanup"
    rm -rf "${DEB_TMP_DIR}" "${ARCH}.tar.gz" "${NAME}"

    # lintian is not available on every packaging host, and the package is
    # verified on the repo host anyway, hence the || true.
    echo " -> lint ${ARCH} package"
    lintian "${DEB_PACKAGE}" || true
done

echo "return to original commit"
git checkout --quiet "${COMMIT}"
