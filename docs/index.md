# ViewOS Documentation

Welcome to the ViewOS documentation. ViewOS is a Debian-based Linux distribution featuring **head-tracked window perspective** — windows rotate in 3D to face your head using a local webcam.

## Quick Links

- [Installation Guide](installation.md)
- [Face Tilt Feature](face-tilt.md)
- [Hardware Compatibility](hardware.md)
- [Building from Source](building.md)
- [Package Repository](repository.md)
- [Governance](governance.md)

## What is ViewOS?

ViewOS is a Debian 13 (Trixie) based distribution with KDE Plasma 6, featuring a unique **face-tilt** feature:

- **Local-only processing** — your webcam frames never leave your machine
- **Per-window 3D transforms** — each window tilts independently toward your face
- **Calibration wizard** — first-run setup for screen dimensions and camera position
- **Privacy-first** — no network code, persistent camera indicator, explicit opt-out

## System Requirements

| Component | Minimum | Recommended |
|---|---|---|
| CPU | x86_64, 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Storage | 20 GB | 50+ GB |
| GPU | OpenGL 3.3+ | Vulkan 1.2+, NVIDIA 570+ |
| Camera | 720p webcam | 1080p, 30fps |

## Support

- **Email:** help@jmultimate.com
- **Issues:** GitHub Issues
- **Ideas:** ideas@jmultimate.com

## License

ViewOS is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

Individual packages retain their own licenses (Debian, KDE, Rust crates, etc.).