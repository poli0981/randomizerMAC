# RANDOM MAC v1.0.0

First public release of **RANDOM MAC** - a lightweight Windows utility for randomizing network adapter MAC addresses.

Built with C# 13 / .NET 9 / Avalonia 11.2.

---

## Highlights

- **Randomize MAC** with cryptographically secure generation (locally administered, unicast)
- **Per-adapter blacklist** with automatic failed-MAC tracking
- **Auto-change on startup** - randomize MAC when Windows boots
- **Dark / Light theme** with 6 accent colors
- **Multi-language** - English + Vietnamese (real-time switching)
- **System tray** with minimize-to-tray and background operation
- **Update check** via GitHub Releases + Velopack
- **Self-contained** - no .NET runtime installation required

## Features

| Feature | Description |
|---------|-------------|
| MAC Randomization | Crypto-secure, locally administered, unicast addresses |
| Vendor Lookup | ~70 common OUI prefixes (Intel, Realtek, Broadcom, etc.) |
| Blacklist | Global + per-adapter, 3 reserved MACs, auto-repair |
| One-Click Restore | Revert to factory MAC address |
| Copy to Clipboard | Click any MAC to copy |
| History | Track all changes with timestamps and status |
| Auto-Change | Randomize MAC on OS startup (configurable per adapter) |
| Startup | Task Scheduler integration (no UAC prompt) |
| Theme | Dark/Light + Blue/Red/Green/Purple/Orange/Teal accents |
| Localization | 83 keys, English + Vietnamese, extensible via .resx |
| Notifications | Type-colored toasts (Success/Error/Warning/Info) |
| Logging | Real-time viewer + file export + rolling 7-day retention |
| Updates | Velopack + GitHub API with 12 status codes |
| Settings | Import/Export, all preferences persisted as JSON |

## System Requirements

| | Minimum | Recommended |
|---|---------|-------------|
| **OS** | Windows 10 22H2 | Windows 11 |
| **RAM** | 6 GB | 8 GB+ |
| **Disk** | ~160 MB | ~200 MB |
| **Privileges** | Administrator | Administrator |

## Installation

1. Download the release asset below
2. Extract and run `RandomMac.App.exe`
3. Approve the UAC prompt (admin required for MAC changes)

No .NET runtime needed - the app is **self-contained**.

## Tested On

| Environment | Result |
|-------------|--------|
| Windows 11 Pro 25H2 Insider (physical) | Passed |
| Windows 10 22H2 (VirtualBox VM) | Passed |
| Windows Sandbox | Passed |

## Known Issues

- **Start Minimized** toggle may not work correctly in all scenarios
- **App Icon** is placeholder (no custom icon yet)

## AI Disclosure

This application was developed with assistance from **Anthropic Claude** (Claude Opus 4.6) via Claude Code. All code was reviewed, tested, and approved by the developer. AI-generated translations are not guaranteed for accuracy.

## Links

- [Documentation](https://github.com/poli0981/randomizerMAC/blob/master/README.md)
- [Changelog](https://github.com/poli0981/randomizerMAC/blob/master/CHANGELOG.md)
- [Disclaimer](https://github.com/poli0981/randomizerMAC/blob/master/DISCLAIMER.md)
- [License (MIT)](https://github.com/poli0981/randomizerMAC/blob/master/LICENSE)

---

**Full Changelog**: https://github.com/poli0981/randomizerMAC/commits/v1.0.0
