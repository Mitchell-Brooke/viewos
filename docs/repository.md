# Package Repository

ViewOS maintains a small signed APT repository layered on top of Debian's archive.

## Repository URL

```
https://jmultimate.github.io/viewos-apt/
```

## Adding the Repository

### On ViewOS (pre-configured)
Already configured in `/etc/apt/sources.list.d/viewos.list`.

### On Other Debian/Ubuntu Systems
```bash
# 1. Download and install signing key
curl -fsSL https://jmultimate.github.io/viewos-apt/signing-key.gpg | \
    sudo gpg --dearmor -o /usr/share/keyrings/viewos-apt.gpg

# 2. Add repository
echo "deb [signed-by=/usr/share/keyrings/viewos-apt.gpg] \
    https://jmultimate.github.io/viewos-apt trixie main" | \
    sudo tee /etc/apt/sources.list.d/viewos.list

# 3. Update
sudo apt update
```

## Available Packages

| Package | Description |
|---|---|
| `viewos-desktop` | Complete desktop metapackage |
| `viewos-kwin-effect` | KWin face-tilt effect |
| `viewos-face-daemon` | Rust face tracking daemon |

## Repository Structure

```
viewos-apt/
├── dists/
│   └── trixie/
│       ├── InRelease          # Signed index
│       ├── Release
│       ├── Release.gpg
│       └── main/
│           ├── binary-amd64/
│           │   ├── Packages
│           │   ├── Packages.gz
│           │   └── Release
│           └── source/
│               ├── Sources
│               └── Release
├── pool/
│   └── main/
│       └── v/
│           ├── viewos-desktop/
│           ├── viewos-kwin-effect/
│           └── viewos-face-daemon/
├── signing-key.gpg            # Public key
└── signing-key.asc            # ASCII armored
```

## Key Management

### Master Key (Offline)
- Generated offline, stored securely
- Used only to sign subkeys
- **Never** on any networked machine

### Signing Subkey (CI)
- Used by GitHub Actions for repo signing
- Stored in GitHub Actions secrets (`GPG_PRIVATE_KEY`)
- Rotated annually

### Verification
```bash
# Verify package signature
dpkg-sig --verify viewos-*.deb

# Verify repo signature
apt update  # Shows "The following signatures were valid..."
```

## Publishing Process

1. **Tag release:** `git tag v0.1.0 && git push origin v0.1.0`
2. **GitHub Actions triggers:**
   - Builds packages in Docker
   - Imports GPG key from secrets
   - Runs `reprepro includedeb`
   - Deploys to `gh-pages` branch
3. **Verify:** `apt update && apt policy viewos-desktop`

## Mirroring

The repository is small (<10 MB). To mirror:
```bash
rsync -avz --delete \
    https://jmultimate.github.io/viewos-apt/ \
    /path/to/local/mirror/
```

Then serve with nginx/apache and point clients to your mirror.

## Security

- All packages signed with subkey
- `InRelease` signed with master key (offline)
- Reproducible builds: same source → same binary
- Transparency: build logs in GitHub Actions
- Key rotation documented in `KEY_ROTATION.md` (internal)

## Removing

```bash
sudo rm /etc/apt/sources.list.d/viewos.list
sudo rm /usr/share/keyrings/viewos-apt.gpg
sudo apt update
```