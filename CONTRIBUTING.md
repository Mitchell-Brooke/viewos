# Contributing to ViewOS

Thank you for contributing! This document covers the main contribution workflows.

## Table of Contents

1. [Development Environment](#development-environment)
2. [Package Contributions](#package-contributions)
3. [Face-Tilt Development](#face-tilt-development)
4. [ISO Build Testing](#iso-build-testing)
5. [Pull Request Process](#pull-request-process)
6. [Commit Conventions](#commit-conventions)

## Development Environment

### Minimal Setup (Package Development)
```bash
# Debian Trixie or derivative
sudo apt install -y dpkg-dev devscripts equivs git-buildpackage

# For Rust packages
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# For KWin effect
sudo apt install -y cmake ninja-build qt6-base-dev \
    libkf6config-dev libkf6coreaddons-dev libkf6windowsystem-dev \
    libkwin-dev
```

### Full ISO Build Environment
```bash
# Requires root, 50+ GB disk, 16+ GB RAM
sudo apt install -y live-build debootstrap squashfs-tools \
    xorriso grub-efi-amd64
```

## Package Contributions

### Adding a Package to the ISO

1. Add to appropriate `.list.chroot` in `config/package-lists/`
2. If not in Debian, create a package in `packages/`
3. Build and test locally
4. Submit PR

### Creating a New Package

```bash
# 1. Create package directory
mkdir -p packages/my-package/DEBIAN

# 2. Write DEBIAN/control
cat > packages/my-package/DEBIAN/control << 'EOF'
Package: my-package
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Your Name <you@example.com>
Depends: ${shlibs:Depends}, ${misc:Depends}
Description: Short description
 Long description...
EOF

# 3. Add files under usr/, etc/
# 4. Build
fakeroot dpkg-deb --build packages/my-package packages/build/

# 5. Test install
sudo dpkg -i packages/build/my-package_1.0.0_amd64.deb
```

### Package Guidelines
- Follow Debian Policy Manual
- Use `debhelper`/`dh` for complex packages
- Include `DEBIAN/postinst`/`prerm` for service management
- Sign packages (CI handles this)
- Keep `Installed-Size` accurate

## Face-Tilt Development

### Daemon (Rust)

```bash
cd face-tilt/daemon

# Run with debug logging
RUST_LOG=debug cargo run -- --config ../config/face-daemon.toml

# Run tests
cargo test

# Check for issues
cargo clippy
cargo audit

# Build release
cargo build --release
```

#### Adding a New Detection Backend

1. Implement `Detector` trait in `detector.rs`
2. Add config struct in `config.rs`
3. Wire up in `main.rs`
3. Update `face-daemon.toml` with new options

### KWin Effect (C++)

```bash
cd face-tilt/kwin-effect

# Configure
cmake -B build -DCMAKE_BUILD_TYPE=Debug -DCMAKE_INSTALL_PREFIX=/usr

# Build
cmake --build build -j$(nproc)

# Install locally for testing
DESTDIR=$HOME/.local cmake --install build

# Restart KWin to load
kwin_x11 --replace &
```

#### Effect Development Tips

- Use `qCDebug(KWIN_EFFECTS)` for debug output
- Test with `WAYLAND_DISPLAY=` to force X11
- Check `kwin --replace` logs for effect loading
- Use `kwriteconfig6` to toggle effect config

### Testing Without Hardware

```bash
# Mock tilt data via socket
socat -u OPEN:/dev/stdin UNIX-CONNECT:/run/viewos/face-tilt.sock << 'EOF'
{"yaw": 5.0, "pitch": -3.0, "roll": 0.0, "confidence": 0.9, "timestamp": 1234567890}
EOF
```

## ISO Build Testing

### Quick VM Test (QEMU)
```bash
# UEFI
qemu-system-x86_64 \
    -bios /usr/share/ovmf/OVMF.fd \
    -cdrom iso/viewos-YYYYMMDD-amd64.iso \
    -m 4G -smp 4 -enable-kvm

# BIOS (legacy)
qemu-system-x86_64 \
    -cdrom iso/viewos-YYYYMMDD-amd64.iso \
    -m 4G -smp 4 -enable-kvm
```

### Test Checklist

- [ ] Boots to GRUB menu
- [ ] Live session starts (Plasma desktop)
- [ ] Network works (Wi-Fi if laptop)
- [ ] Audio works
- [ ] Installer (Calamares) completes
- [ ] Installed system boots
- [ ] Face-tilt daemon starts
- [ ] Calibration wizard works
- [ ] Windows tilt correctly
- [ ] Exclusions work (fullscreen, dialogs)
- [ ] Toggle shortcut works

### Hardware Test Matrix

| Test | Desktop NVIDIA | Desktop AMD | Laptop Intel | Laptop NVIDIA |
|---|---|---|---|---|
| UEFI boot | ☐ | ☐ | ☐ | ☐ |
| Live session | ☐ | ☐ | ☐ | ☐ |
| Install | ☐ | ☐ | ☐ | ☐ |
| Post-install boot | ☐ | ☐ | ☐ | ☐ |
| NVIDIA drivers | ☐ | N/A | N/A | ☐ |
| Face-tilt | ☐ | ☐ | ☐ | ☐ |

## Pull Request Process

1. **Fork** the repository
2. **Create branch:** `git checkout -b feature/your-feature`
3. **Commit** with conventional commits
4. **Test** locally (package build, ISO if applicable)
5. **Push** and open PR
6. **CI runs** (package builds, linting)
7. **Review** by maintainer
8. **Merge** after approval

### PR Requirements
- [ ] Clear description of changes
- [ ] Related issue linked
- [ ] Tests pass (CI)
- [ ] No unrelated changes
- [ ] Documentation updated if needed

## Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types
| Type | Meaning |
|---|---|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `test` | Adding tests |
| `chore` | Maintenance |
| `build` | Build system |
| `ci` | CI configuration |

### Examples
```
feat(daemon): add support for YuNet 640x640 model

fix(kwin-effect): handle window close during transform

docs: update face-tilt calibration guide

build: update live-build to debian 13.7
```

### Scope Examples
- `daemon` — Rust face daemon
- `kwin-effect` — C++ KWin plugin
- `packages` — Debian packaging
- `iso` — live-build configuration
- `docs` — Documentation
- `ci` — GitHub Actions

## Getting Help

- **Discord/IRC:** Not yet established
- **GitHub Discussions:** For questions
- **GitHub Issues:** For bugs/features
- **Email:** help@jmultimate.com

---

*First-time contributors welcome! Look for "good first issue" labels.*