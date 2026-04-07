# RANDOM MAC

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![.NET 9](https://img.shields.io/badge/.NET-9.0-purple.svg)](https://dotnet.microsoft.com/)
[![Avalonia](https://img.shields.io/badge/UI-Avalonia%2011.2-blueviolet.svg)](https://avaloniaui.net/)
[![Windows](https://img.shields.io/badge/Platform-Windows-0078D6.svg)]()

A lightweight Windows desktop utility for randomizing network adapter MAC addresses.

Built with C# 13, .NET 9, and Avalonia UI.

---

## Features

- **MAC Randomization** - Generate cryptographically secure, locally administered unicast MAC addresses
- **Per-Adapter Blacklist** - Global and adapter-specific blacklists with reserved MAC protection
- **Auto-Change on Startup** - Automatically randomize MAC when the app starts with Windows
- **MAC Vendor Lookup** - Identify the manufacturer of current MAC addresses (OUI database)
- **MAC History** - Track all MAC changes with timestamps and status
- **Copy to Clipboard** - Click any MAC address to copy it
- **Restore Original** - Revert to factory MAC with one click
- **Dark / Light Mode** - Full theme support with 6 accent colors
- **Multi-Language** - English and Vietnamese (extensible via .resx files)
- **System Tray** - Minimize to tray, background operation
- **Auto-Update** - Check for updates via GitHub Releases + Velopack
- **Real-time Log Viewer** - Live application logs with filter and export
- **Settings Import/Export** - Backup and restore configuration

## Screenshots

> *Coming soon*

## Installation

### Download

Download the latest release from [GitHub Releases](https://github.com/poli0981/randomizerMAC/releases).

The application is **self-contained** - no .NET runtime installation required.

### System Requirements

| | Minimum | Recommended |
|---|---------|-------------|
| **OS** | Windows 10 22H2 | Windows 11 |
| **RAM** | 6 GB | 8 GB+ |
| **Disk** | ~160 MB | ~200 MB |
| **Privileges** | Administrator | Administrator |

### Notes

- The application requires **administrator privileges** to modify MAC addresses (registry + WMI).
- Windows will show a UAC prompt on launch. When using "Run at Startup" via Task Scheduler, the prompt is bypassed.

## Usage

1. Launch the application (approve UAC if prompted)
2. Select a network adapter from the dropdown
3. Click **Randomize** to generate a preview MAC
4. Click **Apply** to change the MAC address
5. The adapter will briefly disconnect and reconnect

To restore the original MAC: click **Revert to Factory**.

## Build from Source

### Prerequisites

- [.NET 9 SDK](https://dotnet.microsoft.com/download/dotnet/9.0)
- Git
- IDE: [JetBrains Rider](https://www.jetbrains.com/rider/) (recommended) or Visual Studio 2022+

### Build

```bash
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC
dotnet restore
dotnet build
```

### Run

```bash
dotnet run --project src/RandomMac.App
```

> Note: Run as Administrator for full functionality.

### Test

```bash
dotnet test
```

### Publish (self-contained)

```bash
dotnet publish src/RandomMac.App -c Release -r win-x64 --self-contained true -o publish/
```

## Known Issues

- **Start Minimized**: The "Start Minimized" toggle may not function correctly in all scenarios. The core startup-with-OS feature works via Task Scheduler.
- **App Icon**: Placeholder only - a proper icon has not been designed yet.

## Project Structure

```
RandomMac.sln
src/
  RandomMac.Core/          # Business logic (models, services, helpers)
  RandomMac.App/           # Avalonia UI (views, viewmodels, styles)
tests/
  RandomMac.Tests/         # Unit tests (xUnit)
docs/                      # Documentation
.github/                   # Issue/PR templates
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | C# 13 / .NET 9 |
| UI Framework | Avalonia 11.2 |
| Design System | FluentAvalonia |
| MVVM | CommunityToolkit.Mvvm |
| Logging | Serilog |
| Updates | Velopack + GitHub Releases API |
| DI | Microsoft.Extensions.DependencyInjection |

## Credits

- **Developer**: [poli0981](https://github.com/poli0981)
- **Third-party**: See the About page in the application for a full list of open-source dependencies.

## AI Acknowledgment

This application was built and tested with AI assistance using **Anthropic Claude** (Claude Opus 4.6).

| Role | Contributor |
|------|-------------|
| **Developer** | poli0981 (coding, prompting, testing, review, final decisions) |
| **AI Assistant** | Claude Opus 4.6 by [Anthropic](https://www.anthropic.com/) via [Claude Code](https://claude.ai/claude-code) |

### AI was used for:
- Code generation (architecture, services, UI, models)
- Debugging and bug analysis
- Unit test creation
- Documentation and legal files
- Language translations (English to Vietnamese)
- GitHub templates and community files

### AI was NOT used for:
- Final review and approval of all code (done by developer)
- Manual testing on physical and virtual machines (done by developer)
- Design decisions and feature prioritization (decided by developer)

> All AI-generated translations are **not guaranteed** for accuracy. See [DISCLAIMER.md](DISCLAIMER.md).

## Legal

- [License (MIT)](LICENSE)
- [Disclaimer](DISCLAIMER.md)
- [EULA](EULA.md)
- [Security Policy](SECURITY.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)

> **Important**: Changing your MAC address may violate network policies or terms of service. Use responsibly and in compliance with applicable laws. See [DISCLAIMER.md](DISCLAIMER.md) for full details.
