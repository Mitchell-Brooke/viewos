# Installation Guide

## Download

Download the latest ISO from the [releases page](https://github.com/Mitchell-Brooke/viewos/releases) or from [Internet Archive](https://archive.org/details/viewos).

Verify the SHA256 checksum:
```bash
sha256sum -c viewos-YYYYMMDD-amd64.iso.sha256
```

## Writing to USB

### Linux/macOS
```bash
# Identify your USB device (e.g., /dev/sdX)
lsblk

# Write ISO (replace /dev/sdX with your device)
sudo dd if=viewos-YYYYMMDD-amd64.iso of=/dev/sdX bs=4M status=progress oflag=sync
```

### Windows (Rufus)
1. Download [Rufus](https://rufus.ie/)
2. Select the ViewOS ISO
3. Choose **ISO Image** mode (recommended) or **DD Image** mode
4. Click Start

> **Note:** ViewOS ISOs are hybrid and work in both Rufus modes.

## Booting

1. Insert USB and reboot
2. Enter firmware boot menu (usually F12, F2, or Del)
3. Select the USB device (UEFI preferred)
4. ViewOS GRUB menu appears:
   - **ViewOS Live** — boots to live desktop
   - **Install ViewOS** — starts Calamares installer
   - **Advanced options** — memtest, rescue mode, etc.

### Secure Boot

ViewOS does **not** support Secure Boot. Disable Secure Boot in your firmware settings if the USB fails to boot.

## Live Session

The live session boots into a KDE Plasma desktop with:
- User: `user` (no password)
- Root: no password (use `sudo`)
- Face-tilt: **enabled by default** (webcam light will indicate activity)

### Disable Face-Tilt Temporarily
- Press `Meta+Shift+F` (global toggle)
- Or click the system tray icon → "Disable Face Tilt"

## Installation

1. Boot into **Install ViewOS** or run `calamares` from the live session
2. Follow the Calamares wizard:
   - Language → English (UK)
   - Location → Your timezone
   - Keyboard → Your layout
   - Partitioning → **Erase disk** (recommended) or **Manual**
   - User setup → Your name, username, password
   - Summary → Review and Install

3. Installation takes 5-15 minutes depending on hardware
4. Reboot when prompted, remove USB

## Post-Install

### Face-Tilt Calibration
On first login, the **Face Tilt Calibration Wizard** launches automatically:
1. Measure and enter your screen's physical width/height (mm)
2. Enter camera position relative to screen center
3. Sit at your normal position and look at the calibration dot
4. Save — face-tilt is now calibrated for your setup

### NVIDIA Drivers
If you have an NVIDIA GPU, the proprietary drivers are installed by default. To switch to nouveau:
```bash
sudo apt install --reinstall xserver-xorg-video-nouveau
sudo update-initramfs -u
# Reboot
```

### Updates
```bash
sudo apt update && sudo apt full-upgrade
```

### APT Repository
ViewOS packages are served from a signed APT repository. This is already
configured on an installed ViewOS system; to add it to another Debian or
Ubuntu machine:
```bash
curl -fsSL https://mitchell-brooke.github.io/viewos/signing-key.gpg | \
    sudo gpg --dearmor -o /usr/share/keyrings/viewos-archive-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/viewos-archive-keyring.gpg] \
    https://mitchell-brooke.github.io/viewos trixie main" | \
    sudo tee /etc/apt/sources.list.d/viewos.list

sudo apt update
```

## Troubleshooting

| Issue | Solution |
|---|---|
| Won't boot (Secure Boot) | Disable Secure Boot in UEFI |
| No Wi-Fi | Check `ip link`, install missing firmware |
| No audio | Run `pavucontrol`, check profile |
| Face-tilt not working | Check webcam permissions, run calibration wizard |
| NVIDIA black screen | Boot with `nomodeset`, reinstall nvidia-driver |

## Dual Boot

ViewOS installer supports dual boot with Windows/Linux. Choose **Manual Partitioning** in Calamares and create partitions alongside existing OS. GRUB will detect other OSes automatically.