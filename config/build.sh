#!/bin/bash
# ViewOS live-build configuration
# Run from the repo root: lb config

set -e

lb config noauto \
    --architectures amd64 \
    --distribution trixie \
    --debian-installer live \
    --binary-images iso-hybrid \
    --bootloaders grub-efi \
    --bootappend-live "boot=live components quiet splash username=user hostname=viewos" \
    --bootappend-install "quiet splash" \
    --iso-application "ViewOS" \
    --iso-publisher "ViewOS Project <help@jmultimate.com>" \
    --iso-volume "ViewOS $(date +%Y%m%d)" \
    --linux-flavours amd64 \
    --linux-packages "linux-image linux-headers" \
    --apt apt \
    --apt-recommends true \
    --apt-secure true \
    --apt-source-archives false \
    --archive-areas "main contrib non-free non-free-firmware" \
    --cache true \
    --cache-indices true \
    --cache-packages true \
    --chroot-filesystem squashfs \
    --compression gzip \
    --debootstrap-options "--variant=minbase" \
    --firmware-binary true \
    --firmware-chroot true \
    --grub-splash "/usr/share/images/desktop-base/grub-splash.png" \
    --iso-publisher "ViewOS Project" \
    --iso-application "ViewOS" \
    --keyring-packages "debian-archive-keyring" \
    --memtest none \
    --mode debian \
    --parent-archive-areas "main contrib non-free non-free-firmware" \
    --security true \
    --system normal \
    --updates true \
    --verbose \
    "${@}"