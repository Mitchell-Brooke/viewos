#!/bin/bash
# Build ViewOS KWin Effect package

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EFFECT_DIR="$SCRIPT_DIR/../face-tilt/kwin-effect"
BUILD_DIR="$SCRIPT_DIR/build-kwin-effect"
PACKAGE_DIR="$SCRIPT_DIR/viewos-kwin-effect"
OUTPUT_DIR="$SCRIPT_DIR/build"

mkdir -p "$BUILD_DIR" "$OUTPUT_DIR"

# Build the effect
cd "$EFFECT_DIR"
cmake -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr
cmake --build "$BUILD_DIR" -j$(nproc)

# Install to package directory
DESTDIR="$PACKAGE_DIR" cmake --install "$BUILD_DIR"

# Build Debian package
VERSION=$(grep '^Version:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
ARCH=$(grep '^Architecture:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
PACKAGE_NAME="viewos-kwin-effect_${VERSION}_${ARCH}.deb"

fakeroot dpkg-deb --build "$PACKAGE_DIR" "$OUTPUT_DIR/$PACKAGE_NAME"

echo "Built: $OUTPUT_DIR/$PACKAGE_NAME"
dpkg-deb -I "$OUTPUT_DIR/$PACKAGE_NAME"