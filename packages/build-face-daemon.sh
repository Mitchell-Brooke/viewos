#!/bin/bash
# Build ViewOS Face Daemon package

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SCRIPT_DIR/../face-tilt/daemon"
PACKAGE_DIR="$SCRIPT_DIR/viewos-face-daemon"
OUTPUT_DIR="$SCRIPT_DIR/build"

mkdir -p "$OUTPUT_DIR"

# Build the Rust binary
cd "$DAEMON_DIR"
cargo build --release

# Install binary
mkdir -p "$PACKAGE_DIR/usr/bin"
cp target/release/viewos-face-daemon "$PACKAGE_DIR/usr/bin/"

# Install config
mkdir -p "$PACKAGE_DIR/etc/viewos"
cp face-daemon.toml "$PACKAGE_DIR/etc/viewos/face-daemon.toml"

# Model file will be downloaded separately (too large for package)
# Create placeholder
mkdir -p "$PACKAGE_DIR/usr/share/viewos/models"
cat > "$PACKAGE_DIR/usr/share/viewos/models/README.md" << 'EOF'
# YuNet Model Files

The YuNet model files are not included in this package due to size.
They will be downloaded on first run or during ISO build.

Expected file: face_detection_yunet_2023mar.onnx

Download from:
https://github.com/opencv/opencv_zoo/raw/main/models/face_detection_yunet/face_detection_yunet_2023mar.onnx
EOF

# Build Debian package
VERSION=$(grep '^Version:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
ARCH=$(grep '^Architecture:' "$PACKAGE_DIR/DEBIAN/control" | awk '{print $2}')
PACKAGE_NAME="viewos-face-daemon_${VERSION}_${ARCH}.deb"

fakeroot dpkg-deb --build "$PACKAGE_DIR" "$OUTPUT_DIR/$PACKAGE_NAME"

echo "Built: $OUTPUT_DIR/$PACKAGE_NAME"
dpkg-deb -I "$OUTPUT_DIR/$PACKAGE_NAME"