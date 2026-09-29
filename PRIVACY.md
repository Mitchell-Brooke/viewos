# Privacy Policy

## Overview

ViewOS includes a **face-tilt** feature that uses your webcam to rotate windows toward your face. This document explains exactly what data is collected, how it's used, and your controls.

**TL;DR:** All processing is 100% local. No video leaves your machine. No analytics. No tracking.

## What Face-Tilt Does

1. **Captures** webcam frames (V4L2, user-session only)
2. **Detects** face landmarks using YuNet (local model)
3. **Computes** 3D head position (yaw, pitch, roll)
4. **Publishes** tilt angles to KWin via local Unix socket
5. **KWin rotates** windows to face your head

## Data Flows

```
Webcam → V4L2 → YuNet (RAM) → PnP Solver (RAM) → Unix Socket → KWin (GPU)
                    ↓
            No disk, no network
```

### What Is Processed (RAM Only)
- Raw video frames (ephemeral, ~30fps, discarded after detection)
- Face bounding box + 5 landmarks (eye corners, nose, mouth corners)
- Computed head pose (3 floats: yaw, pitch, roll)

### What Is Stored (Disk)
| File                                | Contents                               | Purpose                   |
|-------------------------------------|----------------------------------------|---------------------------|
| `/etc/viewos/face-daemon.toml`      | Screen dims, camera offset, thresholds | Calibration (system-wide) |
| `~/.config/viewos/calibration.toml` | User-specific calibration              | Per-user override         |
| `~/.config/viewos/preferences.toml` | Enabled/disabled, smoothing            | User preferences          |

**No video frames, no images, no face embeddings are ever written to disk.**

### What Leaves the Machine
**Nothing.** The daemon:
- Has no network code (verifiable in source)
- Binds only to `/run/viewos/face-tilt.sock` (Unix socket, local only)
- Runs as your user, not root
- Cannot access files outside its config directory

## Camera Indicator

When the daemon is active:
- **Webcam LED is ON** (hardware-level, not software-controlled)
- **System tray icon** shows "Face Tilt: Active" with camera symbol
- **Global toggle:** `Meta+Shift+F` instantly stops/starts

## Default State

**Face-tilt is enabled by default** on first boot.

Rationale: The feature is the distribution's signature capability. Users who don't want it can:
1. Disable via `Meta+Shift+F` (instant)
2. Disable via tray icon
3. Uninstall `viewos-face-daemon` package
4. Run calibration wizard → "Disable permanently"

## User Controls

| Action           | Method                                        |
|------------------|-----------------------------------------------|
| Toggle on/off    | `Meta+Shift+F` or tray icon                   |
| Recalibrate      | `Meta+Shift+C` or Settings → Face Tilt        |
| Reset to neutral | `Meta+Shift+R`                                |
| Uninstall        | `sudo apt remove viewos-face-daemon`          |
| Disable at boot  | `systemctl --user disable viewos-face-daemon` |

## Third-Party Components

| Component          | License    | Data Handling        |
|--------------------|------------|----------------------|
| YuNet (OpenCV Zoo) | MIT        | Local inference only |
| OpenCV DNN         | Apache 2.0 | Local inference only |
| KWin Effect        | GPL-3.0+   | Local rendering only |
| Rust std/crates    | MIT/Apache | No telemetry         |

**None of these components have network functionality in our usage.**

## Verification

You can verify the privacy claims:

```bash
# 1. Check daemon has no network code
grep -r "TcpStream\|UdpSocket\|reqwest\|curl" face-tilt/daemon/src/

# 2. Check open sockets (should only show Unix socket)
ss -lp | grep viewos-face-daemon

# 3. Check file access
strace -f -e trace=openat systemctl --user start viewos-face-daemon

# 4. Audit source
cargo audit  # in face-tilt/daemon/
```

## Children's Privacy

ViewOS is not directed at children under 16. Face-tilt processes biometric data (face geometry) locally. If you are a parent/guardian, you can disable the feature before a child uses the system.

## Changes

If this policy changes:
1. Updated in Git with clear commit message
2. Announced in release notes
3. User notification on next boot (if material change)

## Contact

Privacy questions: privacy@jmultimate.com (or help@jmultimate.com)

---

*Last updated: 2026-09-29*
*Version: 0.1 (initial)*