#!/bin/bash
# Shared helpers for building ViewOS Debian packages by hand.
#
# These packages are small enough that we do not need debhelper. This file
# centralises the fiddly bits so every build script does the same thing.

set -euo pipefail

# Root of the repository, regardless of where this file is sourced from.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE_SRC_DIR="${REPO_ROOT}/packages"
PACKAGE_OUT_DIR="${PACKAGE_SRC_DIR}/build"

# die <message...>
die() {
    printf '\033[1;31merror:\033[0m %s\n' "$*" >&2
    exit 1
}

# info <message...>
#
# Goes to stderr, not stdout, because callers do
#
#     deb="$(build_package "$dir")"
#
# and anything a build function writes to stdout is captured as its return
# value. A progress line on stdout would be handed back as if it were a file
# path, and the next command would fail on it.
info() {
    printf '\033[1;32m==>\033[0m %s\n' "$*" >&2
}

# require <command...>  -- abort with a helpful message if anything is missing.
require() {
    local missing=()
    local c
    for c in "$@"; do
        command -v "$c" >/dev/null 2>&1 || missing+=("$c")
    done
    if [ ${#missing[@]} -gt 0 ]; then
        die "missing required tool(s): ${missing[*]}"
    fi
}

# field <control-file> <field-name>  -- print a field from a DEBIAN/control.
field() {
    sed -n "s/^$2: *//p" "$1" | head -n1
}

# normalise-permissions <package-dir>
#
# dpkg-deb is strict about this: DEBIAN must be 0755, control 0644, and every
# other file under DEBIAN must be executable if and only if it is a script
# dpkg will run. Getting this wrong yields an un-installable package, so we
# fix it up rather than relying on whatever the checkout happened to produce.
normalise_permissions() {
    local pkgdir="$1"
    chmod 0755 "${pkgdir:?}/DEBIAN"
    find "${pkgdir}/DEBIAN" -type f \
        -not -name control \
        -not -name md5sums \
        -not -name conffiles \
        -not -name templates \
        -not -name shlibs \
        -exec chmod 0755 {} +
    chmod 0644 "${pkgdir}/DEBIAN/control"
}

# build_package <package-dir> [output-dir]
#
# Builds a .deb from an unpacked package directory and prints its path.
build_package() {
    local pkgdir="$1"
    local outdir="${2:-$PACKAGE_OUT_DIR}"

    [ -f "${pkgdir}/DEBIAN/control" ] || die "no DEBIAN/control in ${pkgdir}"

    local name version arch
    name="$(field "${pkgdir}/DEBIAN/control" Package)"
    version="$(field "${pkgdir}/DEBIAN/control" Version)"
    arch="$(field "${pkgdir}/DEBIAN/control" Architecture)"
    [ -n "$name" ] && [ -n "$version" ] && [ -n "$arch" ] \
        || die "could not read Package/Version/Architecture from ${pkgdir}/DEBIAN/control"

    mkdir -p "$outdir"
    normalise_permissions "$pkgdir"

    local output="${outdir}/${name}_${version}_${arch}.deb"
    rm -f "$output"

    # --root-owner-group makes the build reproducible regardless of who runs
    # it; without it, files end up owned by the invoking user and dpkg refuses
    # to install them.
    dpkg-deb --root-owner-group --build "$pkgdir" "$output" >/dev/null

    info "built ${output}"
    printf '%s\n' "$output"
}

# lint_package <deb-path>
#
# lintian is advisory, so a missing or noisy lintian must not fail the build.
lint_package() {
    local deb="$1"
    if command -v lintian >/dev/null 2>&1; then
        # shellcheck disable=SC2016
        LINTIAN_PROFILE="${LINTIAN_PROFILE:-viewos}" lintian \
            --no-tag-display-limit --quiet "$deb" || true
    fi
}
