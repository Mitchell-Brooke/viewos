# Hardware Compatibility

This document tracks tested hardware configurations. **Help expand it** by filing issues with your results.

## Test Status Legend

| Status | Meaning |
|---|---|
| ✅ **Working** | All features work out of the box |
| ⚠️ **Partial** | Works with caveats (see notes) |
| ❌ **Broken** | Major features fail |
| ❓ **Untested** | No reports yet |

---

## Desktops (Primary Target)

| GPU | CPU | Motherboard | Status | Notes |
|---|---|---|---|---|
| NVIDIA RTX 4080 | AMD Ryzen 7950X | X670E | ❓ | |
| NVIDIA RTX 3080 | Intel i7-12700K | Z690 | ❓ | |
| AMD RX 7900 XTX | AMD Ryzen 7950X | X670E | ❓ | |
| AMD RX 6800 XT | Intel i5-12600K | B660 | ❓ | |
| Intel Arc A770 | Intel i5-13600K | Z790 | ❓ | |
| Intel UHD 770 | Intel i5-12400 | B660 | ❓ | |

## Laptops (Secondary)

| Model | GPU | CPU | Wi-Fi | Status | Notes |
|---|---|---|---|---|---|
| Framework 13 (AMD) | Radeon 780M | Ryzen 7 7840U | MT7922 | ❓ | |
| Framework 13 (Intel) | Iris Xe | i7-1360P | AX210 | ❓ | |
| ThinkPad T14s Gen 4 | Radeon 780M | Ryzen 7 Pro | MT7922 | ❓ | |
| ThinkPad P14s Gen 4 | RTX A500 | Ryzen 7 Pro | MT7922 | ❓ | |
| Dell XPS 13 9320 | Iris Xe | i7-1250U | AX211 | ❓ | |
| ASUS ROG Zephyrus G14 | RTX 4060 | Ryzen 9 7940HS | MT7922 | ❓ | |
| System76 Lemur Pro | Iris Xe | i7-1260P | AX210 | ❓ | |

## Webcams

| Model | Resolution | Interface | Status | Notes |
|---|---|---|---|---|
| Logitech C920 | 1080p30 | USB 2.0 | ❓ | Standard UVC |
| Logitech Brio | 4K30 | USB 3.0 | ❓ | UVC, IR for Windows Hello |
| Razer Kiyo Pro | 1080p60 | USB 3.0 | ❓ | UVC |
| Elgato Facecam | 1080p60 | USB 3.0 | ❓ | UVC, fixed focus |
| Generic laptop cam | 720p30 | Internal | ❓ | Most common |

## Known Issues

### NVIDIA + Wayland
- **Issue:** Some NVIDIA driver versions have Wayland instability
- **Workaround:** Use X11 session (default in ViewOS)
- **Fixed in:** NVIDIA 570+ with explicit sync

### Wi-Fi Firmware
- **Issue:** Some Intel AX210/AX211 need newer firmware
- **Fix:** `sudo apt install firmware-iwlwifi` (included by default)

### Face-Tilt Performance
- **Issue:** High GPU usage on integrated graphics with many windows
- **Mitigation:** Reduce max FPS, disable on battery

---

## How to Report

File an issue with:
```
Hardware: [e.g., "Custom desktop: RTX 3080, i7-12700K"]
Installation: [ISO date, live vs installed]
Working: [Wi-Fi, audio, GPU acceleration, face-tilt]
Not working: [specific issues]
dmesg/lspci/lsusb: [attach relevant logs]
```