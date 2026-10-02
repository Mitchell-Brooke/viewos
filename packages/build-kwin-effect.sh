#!/bin/bash
# Build the KWin effect, then assemble the viewos-kwin-effect package.
#
# Requires a Plasma 6 development environment. On Debian that means the
# kwin-dev package; the build is otherwise self-contained.
set -euo pipefail
# shellcheck source=lib/build-common.sh
source "$(dirname "$(readlink -f "$0")")/lib/build-common.sh"

require cmake dpkg-deb

EFFECT_SRC_DIR="${REPO_ROOT}/face-tilt/kwin-effect"
PKG_SRC_DIR="${PACKAGE_SRC_DIR}/viewos-kwin-effect"
BUILD_DIR="${EFFECT_SRC_DIR}/build"

info "configuring the KWin effect"
rm -rf "$BUILD_DIR"
cmake -S "$EFFECT_SRC_DIR" -B "$BUILD_DIR" \
      -DCMAKE_BUILD_TYPE=RelWithDebInfo \
      -DCMAKE_INSTALL_PREFIX=/usr

info "building the KWin effect"
cmake --build "$BUILD_DIR" --parallel "$(nproc)"

info "installing into a staging directory"
STAGE_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGE_DIR"' EXIT
cmake --install "$BUILD_DIR" --prefix "$STAGE_DIR"

# Also copy the built plugin to a known location for debugging
find "$BUILD_DIR" -name 'viewosfacetilt*.so' -exec cp {} "$STAGE_DIR/" \;

# The plugin has to be a MODULE, not a shared library. A wrongly linked object
# still loads, but then carries an SONAME that makes KWin's plugin loader skip
# it with no useful diagnostic, so the symptom is "the effect silently does
# not appear". Checking here turns that into a build failure.
# KWin plugins are installed without the "lib" prefix.
plugin_so="$(find "$STAGE_DIR" -name 'viewosfacetilt*.so' -print -quit)"
[ -n "$plugin_so" ] || die "cmake did not install anything called viewosfacetilt.so"

if readelf -d "$plugin_so" 2>/dev/null | grep -q '(SONAME)'; then
    die "$(basename "$plugin_so") has a SONAME. It must be built as a CMake MODULE library, \
otherwise KWin's plugin loader will not find it."
fi
info "plugin is a loadable module: ${plugin_so#"$STAGE_DIR"}"

# Stage into the package tree so the .deb is built from exactly what was
# compiled, rather than from a hand-maintained file list that can drift.
info "populating ${PKG_SRC_DIR}"
rm -rf "${PKG_SRC_DIR}/usr"
install -d "${PKG_SRC_DIR}/usr"
cp -a "$STAGE_DIR/." "${PKG_SRC_DIR}/usr/"

find "${PKG_SRC_DIR}/usr" -type f -printf '%M %p\n' | sed "s|${PKG_SRC_DIR}/||"

deb="$(build_package "$PKG_SRC_DIR")"
lint_package "$deb"

info "built deb at: ${deb}"
ls -la "${deb}"
