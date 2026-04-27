# RANDOM MAC

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![.NET 11](https://img.shields.io/badge/.NET-11.0--preview-purple.svg)](https://dotnet.microsoft.com/)
[![WinUI 3](https://img.shields.io/badge/UI-WinUI%203%20%2B%20WinAppSDK%201.8-0078D4.svg)](https://learn.microsoft.com/en-us/windows/apps/winui/winui3/)
[![Windows](https://img.shields.io/badge/Platform-Windows-0078D6.svg)]()

A lightweight Windows desktop utility for randomizing network adapter MAC addresses.

Built with C# 13, .NET 11, and WinUI 3 (Microsoft.WindowsAppSDK 1.8) — Fluent Design 2.

> **v1.1.0 — Framework migration.** The UI was rewritten from Avalonia 11.2 to WinUI 3 / WindowsAppSDK 1.8. Application logic in `RandomMac.Core` is unchanged. Existing `settings.json` / `blacklist.json` / `history.json` files load without migration. See [Migration notes](#migration-notes-v110) below.

---

## Features

- **MAC Randomization** — Cryptographically secure, locally administered, unicast MAC addresses.
- **Per-Adapter Blacklist** — Global and adapter-specific blacklists with reserved MAC protection. Failed MACs auto-blacklisted.
- **Auto-Change on Startup** — Automatically randomize MAC when Windows boots (per-adapter).
- **Auto-Update Check** — Throttled 24h check on launch via Velopack + GitHub Releases.
- **MAC Vendor Lookup** — Identify the manufacturer of any MAC address (~70 OUI prefixes).
- **MAC History** — Track all changes with relative timestamps ("5 min ago"). Auto-prune after 30 days, atomic save.
- **History Filter** — Live search by adapter name or MAC string.
- **Copy to Clipboard** — Click any MAC card to copy it.
- **Restore Original** — Revert to factory MAC with one click.
- **Dark / Light Mode** — Theme switches live; brand accent color is fixed (`#61AFEF` blue).
- **Multi-Language** — English and Vietnamese, real-time switching (extensible via `.resx`).
- **System Tray** — Minimize to tray, double-click to restore, right-click menu (Show / Randomize Active Adapter / Exit).
- **Auto-Apply Settings** — Each toggle applies immediately; persist is debounced (500ms) to keep disk I/O minimal — no Save button.
- **Bundle Export** — Single ZIP with `settings.json` + `blacklist.json` + `history.json` for backup or migration.
- **Real-time Log Viewer** — Live application logs with filter and export to `.txt` / `.log`.
- **Keyboard Shortcuts** — `Ctrl+1..5` jump between pages, `Ctrl+R` Randomize, `Ctrl+Enter` Apply.
- **Hamburger Toggle** — Collapse the navigation pane to icon-only (48px).

## Screenshots

> *Coming soon*

## Installation

### Download

Download the latest release from [GitHub Releases](https://github.com/poli0981/randomizerMAC/releases).

The application is **self-contained** — the WinAppSDK runtime + .NET 11 are bundled, no separate install needed.

### System Requirements

| | Minimum | Recommended |
|---|---------|-------------|
| **OS** | Windows 10 22H2 (build 19041+) | Windows 11 22H2+ |
| **Architecture** | x64 | x64 |
| **RAM** | 4 GB | 8 GB+ |
| **Disk** | ~250 MB | ~300 MB |
| **Privileges** | Administrator | Administrator |

### Notes

- Requires **administrator privileges** to modify MAC addresses (registry + WMI).
- Windows shows a UAC prompt on launch. When using "Run at Startup" via Task Scheduler, the prompt is bypassed.
- **Mica backdrop** on Windows 11; **Acrylic** fallback on Windows 10; solid color otherwise.

## Usage

1. Launch the application (approve UAC if prompted).
2. Select a network adapter from the dropdown (icons indicate WiFi / Ethernet).
3. Click **Randomize** (or press `Ctrl+R`) to generate a preview MAC.
4. Click **Apply** (or press `Ctrl+Enter`) to commit the change.
5. The adapter briefly disconnects and reconnects.

To restore the original MAC, click **Revert to Factory**.

For one-click MAC change without opening the UI, right-click the tray icon → **Randomize Active Adapter**.

## Build from Source

### Prerequisites

- [.NET 11 SDK preview](https://dotnet.microsoft.com/download/dotnet/11.0) (≥ `11.0.100-preview.3`). Pinned via `global.json`.
- Git
- IDE: [JetBrains Rider 2026.1+](https://www.jetbrains.com/rider/) (recommended) or [Visual Studio 2022 17.10+](https://visualstudio.microsoft.com/) with the "Windows App SDK C# Templates" workload.
- Windows 10 SDK 26100 (installed automatically by `Microsoft.Windows.SDK.BuildTools` package).

### Build

```bash
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC
dotnet restore
dotnet build
```

The csproj sets `RuntimeIdentifier=win-x64` by default — Rider/VS "Build" works without flags. CLI users can override with `-r <rid>` if needed.

### Run

```bash
dotnet run --project src/RandomMac.App
```

> **Run as Administrator** for full functionality (MAC change requires elevation).

### Test

```bash
dotnet test
```

37 unit tests cover the `RandomMac.Core` business logic.

### Publish (self-contained)

```bash
dotnet publish src/RandomMac.App -c Release -r win-x64 --self-contained true -o publish/
```

The output is ~225 MB and ships the WinAppSDK runtime + .NET 11 framework — no external dependencies on the target machine.

## Migration Notes (v1.1.0)

- **UI framework**: Avalonia 11.2 → WinUI 3 (Microsoft.WindowsAppSDK 1.8).
- **TFM**: `RandomMac.App` → `net11.0-windows10.0.26100.0`. `RandomMac.Core` stays on `net9.0`.
- **Window**: was 880×600, now **1024×680** fixed; resize disabled, maximize button hidden.
- **Theme**: removed the 6-color accent picker — accent is now hardcoded `#61AFEF`.
- **Settings UX**: removed the "Save" button. Each toggle auto-applies immediately and persists with a 500ms debounce.
- **Tray**: ported to `H.NotifyIcon.WinUI` (replacing `Avalonia.TrayIcon`). Now supports double-click restore and includes a "Randomize Active Adapter" quick action.
- **File pickers**: switched to Win32 `comdlg32` `GetOpenFileName`/`GetSaveFileName` because `Windows.Storage.Pickers` are unreliable in elevated unpackaged WinUI 3 processes.
- **Settings/blacklist/history JSON**: format unchanged. The dropped `AccentColor` field is silently ignored on load. `&` in PnpDeviceId is no longer JSON-escaped to `&`.

## Known Issues

- **App Icon** — placeholder only; a custom icon is not yet designed.
- **MVVMTK0045 build warnings** — 40 informational notices about `[ObservableProperty]` AOT compatibility with WinRT. Non-blocking; deferred for a future polish pass.

## Project Structure

```
RandomMac.sln
src/
  RandomMac.Core/                    # Business logic, no UI dependency (net9.0)
  RandomMac.App/                     # WinUI 3 desktop app (net11.0-windows10.0.26100.0)
    App.xaml(.cs)                    # Application + DI bootstrap
    MainWindow.xaml(.cs)             # Window shell + NavigationView + tray
    Themes/Theme.xaml                # ThemeDictionaries (Default/Light)
    Views/                           # Page UserControls + PageTemplateSelector
    ViewModels/                      # MVVM ViewModels
    Controls/                        # NotificationPopup
    Converters/                      # IValueConverter implementations
    Services/                        # Theme, Tray, Notification, LogSink
    Helpers/                         # Win32FileDialog (comdlg32 wrapper)
    Localization/                    # Lang.resx + Lang.vi.resx + Loc.cs
tests/
  RandomMac.Tests/                   # xUnit (net9.0)
docs/                                # Documentation
.github/                              # Issue / PR templates
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | C# 13 |
| Runtime (App) | .NET 11 (preview) |
| Runtime (Core) | .NET 9 |
| UI Framework | WinUI 3 + Microsoft.WindowsAppSDK 1.8 (Fluent Design 2) |
| DataGrid | CommunityToolkit.WinUI.UI.Controls.DataGrid 7.1.2 |
| System Tray | H.NotifyIcon.WinUI 2.2 |
| MVVM | CommunityToolkit.Mvvm 8.4 |
| Logging | Serilog 4.2 |
| Updates | Velopack + GitHub Releases API |
| DI | Microsoft.Extensions.DependencyInjection 9.0 |
| WMI | System.Management 9.0 |

## Credits

- **Developer**: [poli0981](https://github.com/poli0981)
- **Third-party**: see the About page in the application for the full dependency list.

## AI Acknowledgment

This application was built and tested with AI assistance using **Anthropic Claude**.

| Role | Contributor |
|------|-------------|
| **Developer** | poli0981 (coding, prompting, testing, review, final decisions) |
| **AI Assistant (v1.0.0)** | Claude Opus 4.6 by [Anthropic](https://www.anthropic.com/) via [Claude Code](https://claude.ai/claude-code) |
| **AI Assistant (v1.1.0+)** | Claude Opus 4.7 by [Anthropic](https://www.anthropic.com/) via [Claude Code](https://claude.ai/claude-code) |

### AI was used for
- Code generation (architecture, services, UI, models)
- Debugging and bug analysis
- Unit test creation
- Documentation and legal files
- Language translations (English to Vietnamese)
- GitHub templates and community files

### AI was NOT used for
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
