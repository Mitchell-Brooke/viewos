# Building from Source

## Prerequisites

### Build Machine
- Debian 13 (Trixie) or derivative
- 50+ GB free disk space
- 16+ GB RAM (32 GB recommended)
- Root access (for live-build)

### Install Build Dependencies
```bash
sudo apt update
sudo apt install -y \
    live-build debootstrap squashfs-tools xorriso \
    grub-efi-amd64 grub-efi-ia32 \
    dpkg-dev devscripts equivs \
    cmake ninja-build \
    cargo rustc \
    qt6-base-dev qt6-base-dev-tools \
    libkf6config-dev libkf6coreaddons-dev libkf6windowsystem-dev \
    libopencv-dev \
    git curl wget
```

## Repository Structure

```
viewos/
├── config/              # live-build configuration
│   ├── auto/config      # lb config defaults
│   ├── package-lists/   # .list.chroot files
│   ├── hooks/           # chroot/binary/live hooks
│   └── packages.chroot/ # Local .debs for ISO
├── packages/            # Custom Debian packages
│   ├── viewos-desktop/      # Metapackage
│   ├── viewos-kwin-effect/  # KWin effect
│   ├── viewos-face-daemon/  # Rust daemon
│   ├── build-*.sh           # Package build scripts
│   └── build/               # Output .debs
├── face-tilt/           # Face-tilt source code
│   ├── daemon/          # Rust daemon (Cargo)
│   └── kwin-effect/     # KWin effect (CMake)
├── repo/                # APT repository (published to GitHub Pages)
│   └── conf/distributions
├── docs/                # Documentation (GitHub Pages)
├── .github/workflows/   # CI for package publishing
├── build-iso.sh         # Main ISO build script
└── iso/                 # Output ISOs (gitignored)
```

## Building Packages

### ViewOS Desktop Metapackage
```bash
cd packages
./build-desktop.sh
# Output: build/viewos-desktop_1.0.0_amd64.deb
```

### KWin Effect
```bash
cd packages
./build-kwin-effect.sh
# Requires: KWin 6 dev headers, Qt6, KF6
# Output: build/viewos-kwin-effect_0.1.0_amd64.deb
```

### Face Daemon
```bash
cd packages
./build-face-daemon.sh
# Requires: Rust 1.75+, OpenCV 4.8+
# Output: build/viewos-face-daemon_0.1.0_amd64.deb
```

## Building the ISO

### Quick Build (requires root)
```bash
sudo ./build-iso.sh
# Output: iso/viewos-YYYYMMDD-amd64.iso
```

### Step-by-Step

```bash
# 1. Build packages first
cd packages && ./build-desktop.sh && ./build-kwin-effect.sh && ./build-face-daemon.sh

# 2. Copy to live-build local repo
mkdir -p ../config/packages.chroot
cp build/*.deb ../config/packages.chroot/

# 3. Configure live-build
cd ..
lb config

# 4. Build (30-120 min)
sudo lb build

# 5. Find ISO
ls -lh binary.hybrid.iso  # or live-image-amd64.hybrid.iso
```

## Building in a VM

For reproducible builds without root on host:

```bash
# Create VM (QEMU example)
qemu-img create -f qcow2 viewos-build.qcow2 100G
virt-install --name viewos-build --ram 16384 --vcpus 8 \
    --disk viewos-build.qcow2 --cdrom debian-13-amd64-netinst.iso \
    --os-variant debian13

# Inside VM: install build deps, clone repo, run build-iso.sh
```

## GitHub Actions CI

The repo includes `.github/workflows/apt-repo.yml` which:
1. Triggers on tag push (`v*`)
2. Builds packages in Docker
3. Publishes to GitHub Pages APT repo (signed)

**Required secrets:**
- `GPG_PRIVATE_KEY` — ASCII-armored GPG private key
- `GPG_PASSPHRASE` — Key passphrase (or empty)
- `PAGES_TOKEN` — Fine-grained PAT for gh-pages deploy

## Customizing

### Add Packages
Edit `config/package-lists/desktop.list.chroot` or create new `.list.chroot` files.

### Modify Defaults
Edit hooks in `config/hooks/`:
- `chroot/` — runs inside chroot (users, services, configs)
- `binary/` — runs on binary image (GRUB, Plymouth, ISO metadata)
- `live/` — runs on live boot (rarely needed)

### Branding
- `config/hooks/binary/01-viewos-branding.binary` — GRUB/Plymouth themes
- `config/hooks/chroot/01-viewos-defaults.chroot` — locale, SDDM, Plasma defaults
- Wallpapers → `/usr/share/wallpapers/viewos/`

### Face-Tilt Models
The YuNet model is downloaded at build time or runtime. To vendor:
1. Download `face_detection_yunet_2023mar.onnx` from OpenCV Zoo
2. Place in `packages/viewos-face-daemon/usr/share/viewos/models/`
3. Update `face-daemon.toml` model_path

## Troubleshooting

| Error | Fix |
|---|---|
| `lb: command not found` | `sudo apt install live-build` |
| `debootstrap failed` | Check network, try `--mirror http://deb.debian.org/debian` |
| `No space left` | Need 50+ GB, clean with `lb clean --purge` |
| `kwin effect not found` | Install `kwin-dev` `libkf6config-dev` etc. |
| `cargo: not found` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` |
| `opencv not found` | `sudo apt install libopencv-dev` (needs 4.8+) |

## Releasing

```bash
# 1. Update versions in package DEBIAN/control files
# 2. Tag release
git tag v0.1.0
git push origin v0.1.0

# 3. CI builds and publishes packages
# 4. Build ISO locally
sudo ./build-iso.sh

# 5. Upload ISO to Internet Archive
# 6. Create GitHub Release with changelog
```