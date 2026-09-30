# Package Repository

ViewOS maintains a small signed APT repository layered on top of Debian's
archive. It holds ViewOS's own packages; everything else comes from
`deb.debian.org` as usual.

## Repository URL

```
https://mitchell-brooke.github.io/viewos/
```

## Adding the Repository

### On ViewOS

Already configured in `/etc/apt/sources.list.d/viewos.list`.

### On another Debian or Ubuntu system

```sh
# 1. Install the signing key. apt wants a binary keyring, so the exported
#    key is dearmored rather than used as-is.
curl -fsSL https://mitchell-brooke.github.io/viewos/signing-key.gpg \
    | sudo gpg --dearmor -o /usr/share/keyrings/viewos-archive-keyring.gpg

# 2. Add the source
echo "deb [signed-by=/usr/share/keyrings/viewos-archive-keyring.gpg] \
    https://mitchell-brooke.github.io/viewos trixie main" \
    | sudo tee /etc/apt/sources.list.d/viewos.list

# 3. Update
sudo apt update
```

Confirm it worked:

```sh
apt policy viewos-desktop
```

### Why `signed-by`

`apt` only requires a signature for a source listed in a file whose name ends
in `.list` or `.sources`; a key placed in the legacy trusted keyring applies to
every source at once. Naming the keyring per-source means adding ViewOS cannot
weaken the verification of any other repository, and removing it later is a
single file deletion.

## Available Packages

| Package | Description |
|---|---|
| `viewos-desktop` | Desktop metapackage: Plasma 6, drivers, and the default application set |
| `viewos-kwin-effect` | KWin effect that rotates windows to face the viewer |
| `viewos-face-daemon` | Face tracking daemon that feeds the effect |

## Repository Structure

Produced by reprepro, and served as static files by GitHub Pages.

```
dists/
└── trixie/
    ├── InRelease              # signed index; apt prefers this
    ├── Release
    ├── Release.gpg            # detached signature, for older apt
    └── main/
        ├── binary-amd64/
        │   ├── Packages
        │   ├── Packages.gz
        │   └── Release
        └── source/
            ├── Sources
            └── Release
pool/
└── main/
    └── <letter>/<source>/
        └── <package>_<version>_amd64.deb
conf/
└── distributions              # how the index is built
signing-key.gpg                # binary public key
signing-key.asc                # ASCII-armored public key
```

## Key Management

The intended arrangement is a two-key hierarchy:

| Key | Where it lives | What it signs |
|-----|----------------|---------------|
| Master | Offline, on removable media | Subkey certifications only. Never on a networked machine |
| Signing subkey | GitHub Actions secret `GPG_PRIVATE_KEY` | The `InRelease` index |

**This is not yet implemented.** As of v0.1.0 the repository is signed by a
single RSA-4096 key with no expiry, whose private half is stored as the
`GPG_PRIVATE_KEY` secret in the `viewos` repository. The passphrase is the
literal string `1234`.

That is acceptable while the project has one person and no users, and it is not
acceptable once anyone else depends on the repository. The gap is tracked as an
open item in `PLAN`. Until it is closed, treat compromise of that secret as
compromise of the repository, and do not build anything on top of it that would
be expensive to move.

### Verifying

```sh
# The index signature, which is what actually matters. apt reports
# "The following signatures were valid" during update.
apt update

# Inspect the key itself
gpg --show-keys /usr/share/keyrings/viewos-archive-keyring.gpg

# Check what a repository claims for itself
apt-get indextargets --format '$(SITE) $(RELEASE) $(COMPONENT)' \
    | grep mitchell-brooke
```

Individual `.deb` files are **not** signed separately, and are not meant to be.
Debian's own archive does not do this, and `dpkg-sig --verify` against a
Debian package will fail. Integrity of the packages is established by the
checksums in `Packages`, which are covered by the `InRelease` signature: if the
index is authentic, the packages it names are the ones upstream published.

## Publishing Process

1. Push to `master`. The workflow builds all three packages, signs the
   repository and deploys it to the `gh-pages` branch.
2. Tag a release with `git tag v0.1.0 && git push origin v0.1.0` to build from
   a fixed point.

The build is visible at
<https://github.com/Mitchell-Brooke/viewos/actions>.

### Packages are built where they run

`viewos-kwin-effect` and `viewos-face-daemon` build inside a `debian:trixie`
container rather than on the Ubuntu runner. The runner has no KF6 packages at
all, and building against a different distribution's Plasma would defeat the
point of testing on the distribution being targeted.

## Mirroring

The repository is small, under 10 MB. To mirror it:

```sh
rsync -avz --delete https://mitchell-brooke.github.io/viewos/ /srv/mirror/viewos/
```

Serve `/srv/mirror/viewos/` over HTTPS from a host of your choosing and point
clients at it instead. The signing key travels with the mirror, so no
re-signing is needed — which is the point of publishing the key alongside the
index.

## Security

- The `InRelease` index is signed, and `apt` verifies it on every update.
- The signing key is published alongside the repository so a mirror is
  self-contained.
- Build logs are public, so the provenance of any published package can be
  checked.
- Packages are built from the source in the same commit as the index.

**Known gaps**, in the order they should be closed:

- Single signing key, stored in CI secrets, with a weak passphrase. See above.
- Builds are not yet reproducible: timestamps and build paths vary, so two
  builds of the same commit produce different bytes.
- No `-dbgsym` packages are produced, so crash reports from users cannot be
  symbolised.

## Removing

```sh
sudo rm /etc/apt/sources.list.d/viewos.list
sudo rm /usr/share/keyrings/viewos-archive-keyring.gpg
sudo apt update
```
