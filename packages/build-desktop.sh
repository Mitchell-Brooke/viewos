#!/bin/bash
# Build the viewos-desktop metapackage.
set -euo pipefail
# shellcheck source=lib/build-common.sh
source "$(dirname "$(readlink -f "$0")")/lib/build-common.sh"

require dpkg-deb

deb="$(build_package "${PACKAGE_SRC_DIR}/viewos-desktop")"
lint_package "$deb"

info "built deb at: ${deb}"
ls -la "${deb}"

# The metapackage ships no files; if that ever changes this should fail loudly
# rather than silently producing an empty package.
if [ -n "$(find "${PACKAGE_SRC_DIR}/viewos-desktop" -path '*/DEBIAN' -prune -o -type f -print -quit)" ]; then
    die "viewos-desktop is a metapackage but contains payload files"
fi
