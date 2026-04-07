# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in RANDOM MAC, please report it responsibly.

### How to Report

1. **GitHub Security Advisories** (preferred): Go to the [Security tab](https://github.com/poli0981/randomizerMAC/security/advisories) and create a new advisory.
2. **GitHub Issue**: If the vulnerability is not sensitive, open an issue with the label `security`.

**Please do NOT** open a public issue for sensitive vulnerabilities (e.g., those that could be exploited before a fix is available).

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

| Action | Timeline |
|--------|----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 5 days |
| Fix release | Within 7-14 days (depending on severity) |

### What Qualifies as a Security Issue

- Code injection or arbitrary code execution
- Privilege escalation beyond intended admin scope
- Data exfiltration or unintended data exposure
- Bypass of blacklist or safety mechanisms
- Malicious update delivery (supply chain)

### What Does NOT Qualify

- MAC address change not working on specific hardware (this is a compatibility issue, not security)
- UI bugs or visual glitches
- Feature requests
- Translation errors

## Security Design

RANDOM MAC is designed with the following security principles:

- **Local-only data storage**: All settings, blacklists, and history are stored locally. No network transmission of user data.
- **No telemetry**: Zero data collection or analytics.
- **Admin by design**: The application requires administrator privileges because MAC address modification inherently requires elevated access.
- **Cryptographic MAC generation**: Uses `System.Security.Cryptography.RandomNumberGenerator` for MAC address generation.
- **Update verification**: Updates are distributed through GitHub Releases with Velopack integrity checks.
