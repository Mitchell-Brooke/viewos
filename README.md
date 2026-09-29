# ViewOS

> **Head-tracked desktop Linux distribution** — Windows rotate to face your face using a local webcam.

[![Build Status](https://img.shields.io/github/actions/workflow/status/jmultimate/viewos/apt-repo.yml)](https://github.com/jmultimate/viewos/actions)
[![License](https://img.shields.io/badge/license-GPL--3.0+-blue.svg)](LICENSE)
[![Debian Base](https://img.shields.io/badge/base-Debian%2013%20(Trixie)-A81D33)](https://www.debian.org/)
[![Desktop](https://img.shields.io/badge/desktop-KDE%20Plasma%206-3598DC)](https://kde.org/plasma-desktop/)

## What is ViewOS?

ViewOS is a **Debian 13 (Trixie) based** Linux distribution featuring **KDE Plasma 6** and a unique **face-tilt** feature: your windows rotate in 3D to face your head, using a local webcam, screen geometry, and frustum mathematics.

### Key Features

| Feature             | Description                                                        |
|---------------------|--------------------------------------------------------------------|
|🎯 **Face Tilt**     | Per-window 3D perspective transforms toward your head              |
|🔒 **Privacy-First** | 100% local processing, no network code, persistent camera indicator|
|🎮 **KDE Plasma 6**  | Modern, configurable desktop on Qt6                                |
|📦 **Debian Base**   | Rock-solid stability, 5-year LTS                                   |
|🛡️ **NVIDIA Ready**  | Proprietary drivers by default, nouveau fallback                   |
|📀 **Monthly ISOs**  | Fresh installs monthly, continuous updates via APT                 |

## Quick Start

### Download
```bash
# Latest ISO from Internet Archive
wget https://archive.org/download/viewos/viewos-20260929-amd64.iso
sha256sum -c viewos-20260929-amd64.iso.sha256
```

### Write to USB
```bash
# Linux/macOS
sudo dd if=viewos-20260929-amd64.iso of=/dev/sdX bs=4M status=progress oflag=sync

# Windows: Use Rufus (ISO or DD mode)
```

### Boot & Install
1. Boot USB (disable Secure Boot in UEFI)
2. Choose "ViewOS Live" or "Install ViewOS"
3. Follow Calamares installer
4. Reboot, run Face Tilt Calibration wizard

## Face Tilt

The signature feature — windows rotate to face you.

```
Webcam → YuNet (MIT) → PnP Solver → Frustum Math → KWin Effect → Rotated Windows
```

- **Default ON** — toggle with `Meta+Shift+F`
- **Calibration wizard** on first login
- **Exclusions:** fullscreen, dialogs, maximized, KWin UI
- **Performance:** ~20fps, ~5-30% GPU depending on hardware

## Documentation

- [Installation Guide](docs/installation.md)
- [Face Tilt Deep Dive](docs/face-tilt.md)
- [Hardware Compatibility](docs/hardware.md)
- [Building from Source](docs/building.md)
- [Package Repository](docs/repository.md)
- [Governance](docs/governance.md)

## Governance

- [Security Policy](SECURITY.md)
- [Privacy Policy](PRIVACY.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Trademarks](TRADEMARKS.md)
- [Contributing](CONTRIBUTING.md)

## Support

- **Email:** help@jmultimate.com
- **Ideas:** ideas@jmultimate.com
- **Issues:** GitHub Issues
- **Security:** security@jmultimate.com

## License

ViewOS is free software under **GPL-3.0-or-later**. Individual packages retain their licenses (Debian, KDE, Rust crates, etc.).

---

*ViewOS — See your desktop from a new angle.*