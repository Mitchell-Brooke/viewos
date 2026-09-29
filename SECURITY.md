# Security Policy

## Reporting a Vulnerability

**Email:** (or help@jmultimate.com with "SECURITY" in subject)

Do **not** file public issues for security vulnerabilities.

### What to Include
- Description of the vulnerability
- Steps to reproduce
- Affected versions (ISO date, package versions)
- Potential impact
- Any suggested fix

## Response Timeline

| Severity                             | Initial Response | Fix Target   |
|--------------------------------------|------------------|--------------|
| Critical (RCE, privilege escalation) | 24 hours         | 7 days       |
| High (auth bypass, data leak)        | 48 hours         | 14 days      |
| Medium (DoS, info disclosure)        | 1 week           | 30 days      |
| Low (minor issues)                   | 2 weeks          | Next release |

## Supported Versions

| Release              | Security Support              |
|----------------------|-------------------------------|
| Latest monthly ISO   | ✅ Yes                        |
| Previous monthly ISO | ✅ Yes (30 days)              |
| Older ISOs           | ❌ No (reinstall recommended) |

**Base CVEs** are tracked by **Debian**. ViewOS only tracks:
- Custom packages (`viewos-*`)
- Configuration defaults that affect security
- Face-tilt daemon and KWin effect

## Disclosure Policy

1. Vulnerability reported privately
2. Maintainer acknowledges within timeline
3. Fix developed and tested
4. Package updated in APT repo
5. New ISO built if critical
6. **Public disclosure** after fix is available (or 90 days, whichever first)
7. Credit given to reporter (unless anonymous requested)

## Scope

### In Scope
- `viewos-face-daemon` (Rust daemon)
- `viewos-kwin-effect` (KWin C++ plugin)
- `viewos-desktop` metapackage configuration
- Live-build hooks and ISO generation
- APT repository signing infrastructure

### Out of Scope (Debian Responsibility)
- Kernel, systemd, glibc, etc.
- KDE Plasma, Qt, system libraries
- Firefox, LibreOffice, etc.
- Debian archive packages

## Face-Tilt Specific

The face-tilt daemon runs as **user service** (systemd `--user`), not root:
- Camera access via `video` group
- No network capability
- No filesystem access beyond config
- Sandboxed by systemd hardening

Report any:
- Camera access without indicator
- Data written outside config directory
- Network connections from daemon
- Privilege escalation via socket

## Contact

- **Security email:** security@jmultimate.com
- **General support:** help@jmultimate.com
- **PGP key:** Available at https://jmultimate.github.io/viewos-apt/signing-key.asc