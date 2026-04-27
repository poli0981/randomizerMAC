# Changelog

All notable changes to RANDOM MAC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.1] - 2026-04-27

UX polish + correctness fixes on top of the WinUI 3 migration.

### Added

- **Tray quick action** "Randomize Active Adapter" — one-click MAC change on the first connected adapter, no UI needed.
- **Tray double-click** restores the window (was single-click only). Right-click menu items (Show / Randomize / Exit) now reliably fire.
- **Hamburger toggle** on the navigation pane — collapses to 48px icon-only.
- **Empty-state InfoBar** on Dashboard when no adapter is selected.
- **Status InfoBar** with auto-clear after 5s (replaces the static status TextBlock).
- **Keyboard shortcuts**: `Ctrl+1..5` jump between pages, `Ctrl+R` Randomize, `Ctrl+Enter` Apply.
- **Page transitions** — `NavigationThemeTransition` slide+fade between pages.
- **Auto-update check on launch** — throttled by `LastUpdateCheckedAt` (24h cooldown). Surfaces `Notif_UpdateAvailable` toast if a new version is found.
- **History filter** — TextBox above the Recent History grid, live-filters by adapter name or MAC string.
- **History retention** — entries older than 30 days are auto-pruned on Load and on every Add. Hard cap stays at 100 entries.
- **Atomic history save** — writes to `history.json.tmp` then `File.Move(..., overwrite:true)`; a process kill mid-write no longer corrupts the file.
- **Corrupt-history backup** — if `history.json` fails to parse on load, it's copied to `history.json.bak.<timestamp>` and SaveAsync refuses to write until the next clean parse, preventing silent data loss.
- **Bundle export** — Settings → "Export All (zip)" packages `settings.json` + `blacklist.json` + `history.json` into a single ZIP for backup or migration.
- **Connection icon** with text — replaces the 10px colored dot on the Dashboard adapter status row.
- **Adapter dropdown icon** — WiFi / Ethernet glyph next to each adapter name.
- **Relative timestamps** in Recent History — "just now", "5 min ago", "yesterday HH:mm", "yyyy-MM-dd HH:mm".

### Changed

- **Window size** 880×600 → **1024×680**. Fixed, non-resizable, no maximize button.
- **Auto-apply Settings** — every toggle/combo applies immediately (theme, language, startup task) and persists asynchronously after a 500ms debounce. The "Save" button is removed.
- **Accent color picker removed** — UI is now hardcoded to `#61AFEF`. The `AccentColor` field is dropped from `AppSettings`; existing `settings.json` files load without migration (System.Text.Json ignores unknown keys).
- **Update view** — state-driven hero card (✓ green up-to-date, ⬆ accent update-available, ⚠ red error, ⓘ idle), "Last checked: 5 min ago" line, scrollable release notes, indeterminate ProgressBar during download.
- **About view** — centered hero (icon + name + version), compact 4-column third-party rows, dependency list updated for v1.1.x (drops Avalonia / FluentAvalonia, adds Microsoft.WindowsAppSDK / CommunityToolkit.WinUI.UI.Controls.DataGrid / H.NotifyIcon.WinUI).
- **Scrollbars** set to Hidden everywhere — scroll input still works (wheel / touch / keyboard) but the visible track is suppressed.
- **JSON encoding** — `SettingsService` / `HistoryService` / `BlacklistService` now use `JavaScriptEncoder.UnsafeRelaxedJsonEscaping`, so `&` in PnpDeviceId no longer escapes to `&`.

### Fixed

- **Recent History DataGrid headers** showed `Microsoft.UI.Xaml.Data.Binding` instead of localized strings. Cause: `DataGridColumn` is not in the visual tree, so XAML `{Binding}` on `Header` doesn't resolve. Fixed by setting headers from `DashboardView.xaml.cs` code-behind, refreshed on `Loc.PropertyChanged`.
- **Dashboard Refresh always reported "Error loading adapters"**. `IAdapterCacheService.RefreshAsync` uses `ConfigureAwait(false)`, so `AdaptersRefreshed` fired on the threadpool — VM subscribers then mutated bound `ObservableCollection` from a non-UI thread. Fixed by marshaling event handlers through `App.MainDispatcher.TryEnqueue`.
- **Recent History timestamps always read "7 h ago"** on UTC+7 machines. `Timestamp` defaults to `DateTime.UtcNow` (correct), but `RelativeTimeConverter` computed delta as `DateTime.Now - when` (`local - utc`). Fixed by normalizing the input via `Kind`-aware `ToLocalTime()` before subtracting.
- **Recent History wiped when in-memory list happened to be empty.** `LoadAsync` used to swallow JSON parse failures; the next `SaveAsync` then wrote the empty list, overwriting the user's on-disk entries. Hardened with a `_loadCompleted` gate, corrupt-file backup, and atomic `.tmp`+`File.Move` writes.
- **FontIcon glyphs invisible** in `DashboardView` / `UpdateView` / `AboutView` (Refresh, Copy, Shuffle, etc.). Cause: raw Private-Use-Area chars in XAML attribute values were silently stripped to empty by the file write pipeline. Replaced all inline glyphs with XML entity references (`&#xE72C;`).
- **Hamburger collapse left "first letter peek"** — the custom `MenuItemTemplate` rendered the label TextBlock independently of the auto-collapse logic. Added `IsPaneOpen` two-way binding on `MainWindowViewModel` and a back-reference (`NavItem.Owner`) so the template can hide the label when the pane is compact.
- **Tray right-click menu items inert + double-click no-op + window not focused on Show**. Added `XamlRoot` lazy-init for the `MenuFlyout`, `DoubleClickCommand`, and Win32 `ShowWindow(SW_RESTORE)` + `SetForegroundWindow` after `AppWindow.Show`.
- **Window 880×600 felt cramped**. Resized to 1024×680.
- **LogView "Failed to export log: File extensions must begin with '.' and contain no wildcards"**. WinUI 3 `FileSavePicker.FileTypeChoices` rejected `.*`. Removed the wildcard entry and switched to concrete `.txt` / `.log` choices.
- **Settings export / import / bundle export consistently failed in elevated unpackaged WinUI 3.** `Windows.Storage.Pickers.FileSavePicker` and `FileOpenPicker` route through a broker that can't elevate cross-process. Replaced all four pickers (Settings export, Settings import, Bundle export, Log export) with a Win32 `comdlg32` wrapper (`Helpers/Win32FileDialog.cs`) that runs in-process. Output buffer uses an `IntPtr` instead of `StringBuilder` so the `OPENFILENAMEW` struct is blittable for `Marshal.SizeOf`.
- **Resources["Loc"] = Loc.Instance crashed at startup with `COMException 0x8000FFFF`**. The COM proxy isn't fully wired in the App constructor even after `InitializeComponent()`. Moved the assignment to the start of `OnLaunched` where `Application.Resources` is reachable.
- **Plain `dotnet build` failed** with `WindowsAppSDKSelfContained requires a supported Windows architecture`. Added `<RuntimeIdentifier Condition="'$(RuntimeIdentifier)' == ''">win-x64</RuntimeIdentifier>` to the csproj so Rider/VS builds pick a default RID without requiring an explicit `-r` flag.

### Removed

- `AppSettings.AccentColor` (dead since v1.1.0 Phase 6 removed the picker).
- `ThemeService.Apply(string mode, string accentColor)` second parameter dropped.
- The "All Files (`.*`)" choice from LogView's export dialog.

## [1.1.0] - 2026-04-27

### Changed

- **UI/UX migrated from Avalonia 11.2 to WinUI 3 + Microsoft.WindowsAppSDK 1.8 (Fluent Design 2).**
  Fixed window size 880x600 with resize and maximize disabled (and the maximize
  button hidden). Mica backdrop on Windows 11; Desktop Acrylic fallback on
  Windows 10; solid fallback otherwise. Custom titlebar via
  `ExtendsContentIntoTitleBar` + `SetTitleBar(AppTitleBar)`. Pane-permanently-open
  `NavigationView` with Segoe Fluent Icons glyphs. Logic in `RandomMac.Core`
  is unchanged.
- **Target framework moved to `net11.0-windows10.0.26100.0`** for the App
  project (Core stays on `net9.0`); `global.json` pins the SDK to
  `11.0.100-preview.3`. Self-contained Windows App SDK payload, unpackaged
  (compatible with Velopack and the existing `requireAdministrator` manifest).
- Localization: replaced the Avalonia-specific `{loc:L Key}` `MarkupExtension`
  with `Loc.Instance` registered as an Application resource — XAML now binds
  via `{Binding [Key], Source={StaticResource Loc}}`. Live language switching
  preserved via `INotifyPropertyChanged.Item[]`.
- Tray icon backed by `H.NotifyIcon.WinUI` (replacing `Avalonia.TrayIcon`).
- Notifications routed through `App.MainDispatcher` (replacing
  `Avalonia.Threading.Dispatcher.UIThread`).
- File pickers (Settings export/import, Log export) use
  `Windows.Storage.Pickers.FileSavePicker`/`FileOpenPicker` with
  `WinRT.Interop.InitializeWithWindow.Initialize` (unpackaged HWND init).
- Clipboard copy uses `Windows.ApplicationModel.DataTransfer.Clipboard`.

### Fixed

- **Duplicate "Detected adapter:" / "Excluded adapter:" log lines at cold
  start.** Two ViewModels (`DashboardViewModel` and `SettingsViewModel`) used
  to fire-and-forget `INetworkAdapterService.GetPhysicalAdaptersAsync()` from
  their constructors. The two scans ran concurrently and each emitted its own
  log block. Fixed by introducing `IAdapterCacheService` (single-flight load
  guarded by `SemaphoreSlim`); `App.OnLaunched` warms the cache once before
  the ViewModels resolve, and ViewModels read the cached list synchronously
  in their constructors.

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

[1.1.1]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.1.1
[1.1.0]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.1.0
[1.0.0]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.0.0
