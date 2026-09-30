#!/bin/bash
# ViewOS ISO Build Script
# Run from repo root: ./build-iso.sh

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$REPO_ROOT/build"
ISO_DIR="$REPO_ROOT/iso"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[BUILD]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check for root (needed for live-build)
if [ "$EUID" -ne 0 ]; then
    error "This script must be run as root (live-build requirement)"
    exit 1
fi

# Check dependencies
for cmd in lb debootstrap squashfs-tools xorriso grub-efi-amd64; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        error "Missing dependency: $cmd"
        exit 1
    fi
done

log "Building ViewOS ISO..."

# Build custom packages first
log "Building custom packages..."
cd "$REPO_ROOT/packages"
./build-desktop.sh
./build-kwin-effect.sh
./build-face-daemon.sh

# Copy built packages to local repo for live-build
mkdir -p "$REPO_ROOT/config/packages.chroot"
cp "$REPO_ROOT/packages/build"/*.deb "$REPO_ROOT/config/packages.chroot/"

# Clean previous build
log "Cleaning previous build..."
lb clean --purge 2>/dev/null || true
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Configure live-build
log "Configuring live-build..."
cd "$REPO_ROOT"
lb config

# Build
log "Starting live-build (this will take 30-120 minutes)..."
lb build

# Move ISO to output directory
log "Moving ISO to output directory..."
mkdir -p "$ISO_DIR"
mv binary.hybrid.iso "$ISO_DIR/viewos-$(date +%Y%m%d)-amd64.iso" 2>/dev/null || \
mv live-image-amd64.hybrid.iso "$ISO_DIR/viewos-$(date +%Y%m%d)-amd64.iso" 2>/dev/null || \
mv *.iso "$ISO_DIR/" 2>/dev/null || true

# Generate checksums
cd "$ISO_DIR"
for iso in *.iso; do
    if [ -f "$iso" ]; then
        sha256sum "$iso" > "$iso.sha256"
        log "Generated $iso.sha256"
    fi
done

log "Build complete!"
log "ISO location: $ISO_DIR/"
ls -lh "$ISO_DIR"/*.iso 2>/dev/null || true