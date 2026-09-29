#!/bin/bash
# Build ViewOS desktop metapackage

set -e

PACKAGE_DIR="packages/viewos-desktop"
BUILD_DIR="build"
VERSION=$(grep '^Version:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
ARCH=$(grep '^Architecture:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
PACKAGE_NAME="viewos-desktop_${VERSION}_${ARCH}.deb"

mkdir -p "$BUILD_DIR"

# Build the package
fakeroot dpkg-deb --build "$PACKAGE_DIR" "$BUILD_DIR/$PACKAGE_NAME"

# Verify
dpkg-deb -I "$BUILD_DIR/$PACKAGE_NAME"
dpkg-deb -c "$BUILD_DIR/$PACKAGE_NAME"

echo "Built: $BUILD_DIR/$PACKAGE_NAME"