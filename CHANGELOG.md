# Changelog

All notable changes to RANDOM MAC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-07

### Initial Release

First public release of RANDOM MAC, a lightweight Windows utility for randomizing network adapter MAC addresses.

### Added

#### Core
- Cryptographically secure MAC address generation (locally administered, unicast) using `RandomNumberGenerator`
- MAC address change via Windows Registry (`NetworkAddress`) + WMI adapter restart (Disable/Enable)
- Post-change verification with automatic rollback logging
- Physical adapter detection via WMI with smart filtering (excludes VPN, Hyper-V, virtual, Bluetooth adapters)
- Original (factory) MAC backup and one-click restore
- MAC vendor lookup with ~70 common OUI prefixes (Intel, Realtek, Broadcom, Qualcomm, Apple, Samsung, etc.)

#### Blacklist
- Global MAC blacklist with 3 reserved addresses (`00:00:00:00:00:00`, `FF:FF:FF:FF:FF:FF`, `AA:AA:AA:AA:AA:AA`)
- Per-adapter blacklist (keyed by PnpDeviceId) for adapter-specific exclusions
- Failed MAC addresses automatically added to adapter blacklist
- Reserved MACs cannot be removed; survive Clear operations
- Auto-repair: blacklist file recreated with defaults if deleted or corrupted
- Migration: automatic upgrade from flat format to nested JSON format

#### Dashboard
- Adapter selector with type labels (Ethernet, WiFi)
- Three MAC info cards: Current MAC, Original MAC, Preview MAC
- Vendor name display under each MAC address
- Click-to-copy on any MAC address (with notification)
- Action buttons: Randomize, Apply, Revert to Factory, Refresh
- Connection status indicator (green/red dot)
- Randomize disabled when adapter has no connection
- Auto-refresh adapter status 3 seconds after MAC change
- MAC change history table with Time, Adapter, Previous, New, Status columns
- Clear History button (clears in-memory only, preserves JSON file)

#### Settings
- Language selection (English, Vietnamese) with real-time UI switching
- Theme mode: Dark / Light with instant switching
- 6 accent colors: Blue, Red, Green, Purple, Orange, Teal
- Run at Startup via Windows Task Scheduler (`/RL HIGHEST` - no UAC prompt)
- Start Minimized (to system tray)
- Minimize to Tray on close
- Show Notifications toggle
- Auto-Change MAC on Startup with per-adapter selection
- Settings Import / Export (JSON)
- Open Settings File / Open Blacklist File in Explorer

#### System Tray
- Minimize to tray on window close (configurable)
- Tray icon context menu: Show / Exit
- Click tray icon to restore window
- Proper exit via tray menu (bypasses minimize-to-tray handler)

#### Update System
- Update check via Velopack + GitHub Releases API (with fallback for dev/debug)
- 12 status codes with human-readable messages: Idle, Checking, UpdateAvailable, UpToDate, Downloading, ReadyToInstall, ConnectionError, ReleaseNotFound (404), RateLimitExceeded (403), ServerError (5xx), ParseError, InstallError, UnknownError
- Color-coded status badge (green/blue/yellow/red)
- Release notes viewer with scrollable content
- Open release page in browser

#### Localization
- 83 translation keys covering all UI elements
- English (default) and Vietnamese
- Custom `{loc:L Key}` MarkupExtension for XAML bindings
- `Loc.SetLanguage()` with internal culture storage (thread-safe, no drift)
- Reactive navigation labels (update on language change without restart)
- Localized in-app notifications (success, error, warning, info)
- Extensible via `.resx` files

#### Logging
- Serilog with dual sinks: rolling file (7-day retention) + real-time UI
- Real-time log viewer with filter and entry count
- Export logs to `.txt` file
- Clear log (UI only)
- Structured log output: `[timestamp] [LEVEL] message`

#### UI/UX
- Custom window chrome (no native title bar)
- Fixed window size (880x600)
- Left sidebar navigation with 5 pages: Dashboard, Settings, Log, Update, About
- Material Design SVG icons on all buttons (12 icon geometries)
- In-app toast notifications with type-colored backgrounds and left stripe
- Theme-aware colors for Dark and Light modes
- Semi-transparent card backgrounds
- Hidden scrollbars (scroll via mouse wheel)
- About page with developer info, third-party library list (clickable GitHub links), and legal disclaimer

#### Security & Privacy
- Administrator privileges required (app manifest `requireAdministrator`)
- All data stored locally (`%LOCALAPPDATA%\RandomMac\`)
- Zero telemetry, zero data collection
- Only network activity: optional update check via GitHub API
- Admin check on startup with log warning if not elevated

#### Build & Distribution
- .NET 9 / C# 13
- Self-contained publishing (no runtime required)
- Velopack integration for auto-update packaging
- Assembly metadata: Version, Authors, Company, Product, Description, Copyright

#### Documentation
- README with badges, features, install guide, build instructions, system requirements
- MIT License
- Disclaimer (MAC spoofing risks, vendor compatibility, AI translations, privacy)
- EULA (permitted/prohibited use, liability, privacy, AI disclosure)
- Security Policy (reporting process, response timeline, scope)
- Code of Conduct (Contributor Covenant + project-specific rules)
- Contributing Guide (workflow, coding standards, auto-ignore policy)
- Dev Environment Guide (prerequisites, machine specs, tested configs, project structure)
- GitHub issue templates: Bug Report, Feature Request, Language Fix (YAML forms)
- Pull Request template
- CODEOWNERS
- Changelog

### Known Issues
- **Start Minimized** toggle may not function correctly in all scenarios
- **App Icon** is placeholder only (no custom icon designed yet)

[1.0.0]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.0.0
