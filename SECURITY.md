# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 2.0.x   | Yes |
| 1.x     | No — please upgrade to 2.x (1.x has a known privilege-escalation issue in its startup task, fixed in 2.0.0) |

## Reporting a vulnerability

Please report vulnerabilities **privately** through
[GitHub Security Advisories](https://github.com/poli0981/randomizerMAC/security/advisories/new).
Do **not** open a public issue for anything that could be exploited before a fix is available.
Non-sensitive hardening suggestions can be opened as a regular issue with the `security` label.

Please include:

- a description of the vulnerability and its impact,
- steps to reproduce (OS, RandomMac version, installer type),
- a suggested fix, if you have one.

### Response timeline

| Action | Timeline |
|--------|----------|
| Acknowledgment | within 48 hours |
| Initial assessment | within 5 days |
| Fix release | within 7–14 days, depending on severity |

### In scope

- Code execution or privilege escalation (for example through the elevated Windows process, the
  autostart task, `pkexec` invocations, installers or the updater)
- Bypassing the update signature or delivering a malicious update
- Commands or files reachable from the UI that should not be (IPC permissions, CSP, navigation)
- Unintended data exposure
- Bypass of the blacklist or of the safety checks around address changes

### Out of scope

- An adapter whose driver does not accept address changes (compatibility, not security)
- UI glitches, feature requests, translation errors
- Issues that require an already compromised administrator/root account

## Security design

- **Local only.** Settings, blacklist, history and logs stay on your device. No telemetry. The only
  network request is the optional update check against GitHub Releases.
- **Least privilege.** On Windows the app asks for administrator rights only because changing an
  adapter requires them, and runs read-only if you decline; it is installed in `Program Files`, so
  the elevated executable cannot be replaced by a normal process. On Linux the interface never runs
  as root — only the change itself is authorized through NetworkManager/polkit or `pkexec ip`.
- **No shell.** External programs (`schtasks.exe`, `nmcli`, `ip`, `pkexec`, `apt-get`, `dnf`) are
  started by absolute path with validated arguments and a fixed `PATH`.
- **Locked-down UI.** Strict Content-Security-Policy, frozen prototypes, only the app's own commands
  are callable, navigation restricted to the app, external links limited to an allowlist (opened
  through the non-elevated shell on Windows).
- **Cryptographic randomness.** Addresses are generated from the operating system's CSPRNG.
- **Signed updates.** Update packages and `latest.json` are signed (minisign); the public key is
  built into the app. Every release asset has a SHA-256 checksum and a GitHub build-provenance
  attestation (`gh attestation verify <file> --repo poli0981/randomizerMAC`).
- **Supply chain.** Dependencies are checked with `cargo-deny` (RustSec advisories, licenses,
  sources) and `npm audit` in CI; Dependabot keeps them current; CodeQL scans Rust, TypeScript and
  workflows.
