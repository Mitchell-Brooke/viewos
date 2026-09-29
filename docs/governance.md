# Governance

## Project Structure

ViewOS is a **public community distribution** maintained by the ViewOS Project.

### Maintainer
- **Primary:** ViewOS Project (jmultimate.com)
- **Contact:** help@jmultimate.com

### Decision Making
- Technical decisions: Maintainer + community feedback
- Security issues: Immediate action by maintainer
- Feature requests: GitHub Issues, prioritized by impact/effort

## Policies

### Security Policy
See [SECURITY.md](../SECURITY.md) for vulnerability reporting and disclosure.

### Privacy Policy
See [PRIVACY.md](../PRIVACY.md) for face-tilt data handling.

### Code of Conduct
See [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md) for community standards.

### Trademark Policy
See [TRADEMARKS.md](../TRADEMARKS.md) for name/logo usage.

## Release Management

### Versioning
- **ISO releases:** `viewos-YYYYMMDD` (monthly)
- **Package versions:** Semantic (0.1.0, 0.2.0, 1.0.0)
- **Base:** Tracks Debian Trixie point releases

### Release Cadence
- **Monthly ISO rebuilds** (or on Debian point release)
- **Continuous package updates** via APT repo
- **LTS:** Follows Debian's 3-year + 2-year LTS

### Release Checklist
- [ ] All packages build and pass tests
- [ ] ISO boots in QEMU (UEFI + BIOS)
- [ ] ISO boots on test hardware (NVIDIA + AMD/Intel)
- [ ] Face-tilt calibration works
- [ ] Checksums generated and verified
- [ ] ISO uploaded to Internet Archive
- [ ] GitHub Release created with changelog
- [ ] Documentation updated

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for:
- Build environment setup
- Package contribution process
- Face-tilt development workflow
- Commit conventions
- PR review process

## Transparency

- All source code: GitHub (public)
- Build logs: GitHub Actions (public)
- Security advisories: GitHub Security Advisories
- Package signatures: Verifiable via public key
- Financial: No funding currently; would be disclosed if added

## Upstream Relationships

| Upstream | Relationship |
|---|---|
| Debian | Base distribution; we follow their policies |
| KDE | Desktop environment; we contribute bug fixes upstream |
| OpenCV/YuNet | Face detection; MIT licensed, no changes |
| Linux Kernel | Via Debian; no kernel patches |

## Dispute Resolution

1. Technical disagreements: Maintainer decides, with rationale documented
2. Community conflicts: Code of Conduct enforcement
3. Legal/trademark: Maintainer consults legal counsel

## Succession

In case maintainer becomes unavailable:
1. GitHub repository ownership transfers to designated successor
2. Domain (`jmultimate.com`) renewal instructions in secure location
3. GPG master key recovery procedure documented offline
4. Infrastructure (GitHub, Internet Archive) access shared with successor

---

*This governance document is a living document. Changes require maintainer approval and community notification.*