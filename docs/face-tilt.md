# Face Tilt Feature

ViewOS's signature feature: **windows rotate in 3D to face your head**, using a local webcam, screen geometry, and frustum mathematics.

## How It Works

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌─────────────────┐
│  Webcam     │────▶│  YuNet       │────▶│  PnP Solver │────▶│  Frustum Math   │
│  (V4L2)     │     │  Detection   │     │  (5 pts)    │     │  → Tilt Angles  │
└─────────────┘     └──────────────┘     └─────────────┘     └────────┬────────┘
                                                                       │
                                                                       ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌─────────────────┐
│  KWin       │◀────│  Unix Socket │◀────│  Rust       │◀────│  Tilt Data      │
│  Effect     │     │  (/run/...)  │     │  Daemon     │     │  (JSON/line)    │
└─────────────┘     └──────────────┘     └─────────────┘     └─────────────────┘
```

1. **Capture** — V4L2 frames from webcam at 30fps
2. **Detect** — YuNet (MIT, 75K params, ~1.6ms @ 320×320) finds face + 5 landmarks
3. **Solve** — Perspective-n-Point (SolvePnP) computes 3D head pose (yaw, pitch, roll)
4. **Project** — Screen dimensions + camera offset → frustum → per-window tilt angles
5. **Render** — KWin effect applies `WindowPaintData` rotation to each window

## Privacy

**All processing is 100% local.**

- ✅ No network code in the daemon (verifiable by source inspection)
- ✅ No video frames written to disk
- ✅ Only calibration data stored on disk (`~/.config/viewos/calibration.toml`)
- ✅ Persistent system tray indicator when camera is active
- ✅ Global toggle: `Meta+Shift+F` or tray icon
- ✅ Default-on, but explicit opt-out available

## Calibration

### First-Run Wizard

On first login, the wizard guides you through:

1. **Screen dimensions** — Measure viewable area in millimeters (not diagonal!)
2. **Camera offset** — X/Y/Z offset from screen center (mm)
3. **Neutral pose** — Sit normally, look at center dot
4. **Test** — Move head left/right/up/down, verify windows follow

### Manual Configuration

Edit `~/.config/viewos/calibration.toml` or `/etc/viewos/face-daemon.toml`:

```toml
[pose]
screen_width_mm = 530.0    # Your screen's horizontal viewable area
screen_height_mm = 300.0   # Your screen's vertical viewable area
camera_offset_x_mm = 0.0
camera_offset_y_mm = -50.0 # Camera above screen center
camera_offset_z_mm = 0.0
focal_length_mm = 3.6      # Your webcam's focal length
```

### Accurate Measurement Tips

- Use a ruler or calipers on the **viewable panel area** (not bezel)
- Camera offset: measure from screen center to camera lens center
- Y-offset is negative if camera is **above** screen center
- Focal length: check webcam specs (common: 2.8mm, 3.6mm, 4.0mm)

## Controls

| Action | Shortcut |
|---|---|
| Toggle face-tilt | `Meta+Shift+F` |
| Open calibration | `Meta+Shift+C` |
| Reset to neutral | `Meta+Shift+R` |
| Tray icon | Right-click for menu |

## Excluded Windows

These windows are **never tilted** (correctness/safety):

- Fullscreen windows (video playback)
- Maximized windows
- Dialogs, popups, menus, tooltips
- Notifications, splash screens
- KWin's own UI
- Minimized/shaded windows
- Windows on inactive virtual desktops

## Performance

### Battery/GPU Cost

The effect forces **full-screen repaint every frame** (via `PAINT_WINDOW_WITH_TRANSFORMED_WINDOWS`). At 20fps on a laptop:

| Scenario | Estimated GPU | Battery Impact |
|---|---|---|
| Desktop, 1-2 windows | Low | ~5-10% |
| Desktop, 10+ windows | Medium | ~15-25% |
| Laptop, integrated GPU | Higher | ~20-30% |
| Laptop, NVIDIA dGPU | Medium | ~10-20% |

### Mitigations

- Max FPS capped at 20 (configurable)
- Auto-disable on battery (planned)
- Only visible windows transformed
- Smoothing reduces jitter

### Monitoring

```bash
# Check daemon status
systemctl --user status viewos-face-daemon

# View logs
journalctl --user -u viewos-face-daemon -f

# Check GPU usage
intel_gpu_top  # Intel
nvidia-smi -l 1  # NVIDIA
```

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Windows don't tilt | Daemon not running | `systemctl --user start viewos-face-daemon` |
| Tilt wrong direction | Inverted axes | Toggle `invert_yaw`/`invert_pitch` in config |
| Tilt too sensitive | Deadzone too small | Increase `deadzone_deg` |
| Tilt too sluggish | Smoothing too high | Decrease `smoothing_factor` |
| Camera not detected | Permissions | Add user to `video` group, re-login |
| High CPU | Model too large | Use 320×320 input, CPU backend |
| Click offset | Transform + input | Exclusions working? Check fullscreen apps |

## Technical Details

### YuNet Model

- **License:** MIT (OpenCV Zoo)
- **Params:** 75,856
- **Input:** 320×320 (default) or 640×640
- **Output:** bbox + 5 landmarks (eye corners, nose, mouth corners)
- **AP_hard:** ~0.75 on WIDER FACE

### Frustum Math

```
Screen plane: z = 0 (screen center)
Camera: (cx, cy, cz) in screen coordinates
Head: (hx, hy, hz) from PnP

For each window center (wx, wy, 0):
  View vector = normalize((wx, wy, 0) - (hx, hy, hz))
  Window normal = (0, 0, 1) rotated to face view vector
  → Yaw = atan2(view.x, view.z)
  → Pitch = asin(-view.y)
```

### Coordinate Systems

| System | Origin | Axes |
|---|---|---|
| Screen | Center | X=right, Y=up, Z=toward user |
| Camera | Lens center | X=right, Y=down, Z=forward |
| Head (PnP) | Face center | X=right, Y=up, Z=forward |
| KWin | Window center | X=right, Y=down, Z=forward |

## Development

### Daemon (Rust)

```bash
cd face-tilt/daemon
cargo run -- --config /etc/viewos/face-daemon.toml
```

### KWin Effect (C++)

```bash
cd face-tilt/kwin-effect
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
# Install to ~/.local/share/kwin/effects/viewosfacetilt/
```

### Testing Without Hardware

```bash
# Mock tilt data
echo '{"yaw": 5.0, "pitch": -3.0, "roll": 0.0, "confidence": 0.9, "timestamp": 1234567890}' | \
    nc -U /run/viewos/face-tilt.sock
```

## FAQ

**Q: Does it work on Wayland?**
A: Yes — KWin effects are compositor-side and work identically on X11 and Wayland.

**Q: Can I use it with multiple monitors?**
A: Currently only the primary monitor is supported. Multi-monitor support is planned.

**Q: Does it work with external webcams?**
A: Yes, any V4L2-compatible camera. Select with `--camera N` (default 0).

**Q: Can I run it on another distro?**
A: The packages are Debian-specific, but the daemon and effect can be built on any KDE Plasma 6 system.

**Q: What if I don't have a webcam?**
A: Face-tilt gracefully disables. The daemon exits cleanly; KWin effect stays idle.