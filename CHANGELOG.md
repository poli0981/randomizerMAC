# Changelog

All notable changes to RandomMac (named "RANDOM MAC" in 1.x) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-10-15

A complete rewrite. RandomMac now runs on **Windows 10/11 and Linux (Ubuntu, Fedora and other
glibc 2.39+ distributions)**, is built with Rust + Tauri 2 + Svelte 5 instead of .NET/WinUI 3, and
ships a command-line tool. Installers shrink from ~112 MB to ~5 MB.

### Breaking

- **New installers and update channel.** NSIS (`*-setup.exe`) and MSI on Windows; `.deb`, `.rpm` and
  AppImage on Linux. The 1.x Velopack updater cannot install 2.0 — install 2.0 manually, then
  uninstall 1.x (see the README). 1.x data (`settings.json`, `history.json`, `blacklist.json` in
  `%LOCALAPPDATA%\RandomMac`) is imported automatically on first start.
- **Installed per machine in `C:\Program Files\RandomMac`** (1.x installed into the user-writable
  `%LOCALAPPDATA%` while running with administrator rights).
- **New data location and format**: `%LOCALAPPDATA%\io.github.poli0981.randommac\` on Windows,
  `~/.config/…` and `~/.local/share/io.github.poli0981.randommac/` on Linux; camelCase JSON with a
  `schemaVersion`.
- The application id is `io.github.poli0981.randommac`; the executable is `randommac(.exe)`.

### Added

- **Linux support**: adapters from sysfs/udev, hardware address via the `ETHTOOL_GPERMADDR` ioctl
  (no root needed), changes through **NetworkManager** (`cloned-mac-address` on the active
  connection — persistent; the original value of every connection used is remembered, so Restore
  also covers connections the adapter roamed away from) or **iproute2** via `pkexec` (temporary),
  selectable as Auto / NetworkManager / iproute2. The GUI never runs as root; polkit asks only for
  the change itself. Tray support is detected over D-Bus (StatusNotifierWatcher), also when the
  panel starts after RandomMac; without a tray, closing quits. XDG autostart entry; NVIDIA
  blank-window workaround (`WEBKIT_DISABLE_DMABUF_RENDERER`).
- **`randommac-cli`** (installed next to the app and published as standalone archives): `list`,
  `show`, `random`, `set`, `restore`, `inspect`, `vendors`, `history`; `--json`, `--dry-run`,
  `--data-dir`, `--method`, `--mock`; documented exit codes 0–6.
- **Generation modes**: Random (locally administered, default), Keep current vendor, Imitate vendor
  (59 vendors, 11,247 prefixes generated from the IEEE registry). Warnings for addresses that are
  multicast, blacklisted, identical to the current one or not locally administered (Wi-Fi on
  Windows needs `x2/x6/xA/xE`).
- **Automation**: change selected adapters at sign-in, periodically every 5 minutes – 7 days while
  running, and optionally restore every change on exit. **Restore all** in the app and the tray.
- **Disabled adapters** are listed and can be re-enabled; a *Changed* badge marks spoofed adapters;
  vendor lookup for every address.
- **History page** with search, CSV (formula-safe) and JSON export; configurable size (default 500
  entries) and retention (default 90 days).
- **Blacklist** of exact addresses or 1–5 byte prefixes, global or per adapter, with import/export.
- **Settings import/export**, **Export all (zip)**, **re-import from 1.x**, reset to defaults.
- **Diagnostics** (version, OS, privileges, available methods, package type) copied in one click.
- **Signed updates** with `tauri-plugin-updater`: release notes, download progress, install and
  restart; `.deb`/`.rpm` are installed with `pkexec apt-get/dnf`. A real *Check for updates
  automatically* switch (1.x always checked).
- **System theme** option, resizable window that remembers its size and position, **History** as its
  own page (`Ctrl+2`), `F5` refreshes adapters, OS notifications when the window is hidden.
- **App and tray icon** (1.x had none).
- **Security hardening**: strict CSP, frozen prototypes, only the app's own commands are exposed to
  the UI, navigation limited to the app origin, links opened only from an exact host/path
  allowlist (on Windows through the non-elevated shell), DLL search-path and WebView2 environment
  hardening, system programs located through the Windows API instead of `%SystemRoot%`, data
  folder refused when it is a junction/symlink.
- **CI/CD**: Windows + Ubuntu builds, clippy/rustfmt/eslint/prettier/svelte-check, Rust and Vitest
  tests, a real address change on a Linux dummy interface, install/uninstall tests of every package
  (NSIS, MSI, deb, rpm in a Fedora container) with a GUI smoke test, `cargo-deny` + `npm audit`.
  Releases are built from the tag, every updater signature is checked against the app's public
  key, assets are checksummed (`SHA256SUMS.txt`), attested (build provenance) and published with
  a signed `latest.json`; a re-run can never overwrite a published release.
- Documentation: rewritten README (+ Vietnamese), `docs/ARCHITECTURE.md`, `docs/DEV_ENVIRONMENT.md`
  (+ Vietnamese), `CLAUDE.md`.

### Changed

- **Windows MAC changes** use SetupAPI instead of WMI: the adapter is matched by its device instance
  id (not by description), the driver's `NetworkAddress` value is written through the device's own
  driver key, and the device is restarted with `DIF_PROPERTYCHANGE` under a watchdog. Disabled
  adapters are enumerated too.
- **Every change is verified** by polling the live address for up to 20 seconds; if the driver
  ignores it, the previous registry value is restored automatically (1.x slept 2 s and left
  garbage behind).
- **"Original MAC" is the real hardware address** from `GetIfEntry2` (Windows) / ethtool (Linux);
  1.x used the address seen when the app started, so after a change and a restart "original" was
  the spoofed one. **Restore** is available whenever the current address differs; privacy
  features of the operating system (Windows *random hardware addresses*, NetworkManager
  `random`/`stable`) are reported, not overridden.
- **Elevation**: the app asks for administrator rights itself (manifest `asInvoker` + UAC relaunch),
  so a second launch only shows the existing window without a UAC prompt; declining opens a
  read-only mode with *Restart as administrator*.
- **Autostart** uses a per-user Task Scheduler task with highest privileges, no battery
  restrictions and no 72-hour limit, created from XML with the system's own `schtasks.exe`.
  Auto-change runs only when started by the system (`--autostart`), as documented.
- **Storage**: atomic writes with fsync; a damaged file is renamed `*.corrupt-<timestamp>` (3 kept)
  and the app continues with defaults instead of disabling saves or crashing.
- **One change at a time** across the UI, tray, automation and CLI (operation lock). Every change
  runs to completion; quitting waits for a running change before restoring on exit.
- **Logs**: rotating files (5 × 2 MB) plus a live in-app viewer with level filter, search,
  auto-scroll, copy and export.
- **MAC parser** is strict: six hex pairs with one consistent separator (`:`/`-`/none) or Cisco
  `xxxx.xxxx.xxxx` (1.x accepted input like `GG-AA-…`).
- **Vendor table** generated from the IEEE registry (fixes wrong entries such as `52:54:00` being
  listed as Realtek); adapter type comes from the interface type, not from its name.
- Dependabot covers `cargo`, `npm` and `github-actions` with grouped updates and correct commit
  prefixes; workflows pin `ubuntu-24.04`; reusable Discord workflows receive only the secrets they
  need.

### Fixed

- Elevated logon task pointed to an executable in a user-writable folder — any process could replace
  it and gain administrator rights at the next sign-in (**security**). 2.0 removes that task.
- The wrong adapter could be modified when two adapters shared a description or stale registry keys
  existed.
- An adapter could stay disabled (and vanish from the list) if the WMI disable/enable cycle failed.
- `AutoChangeAdapterIds: null` in `settings.json` crashed the app at every start.
- History, settings and blacklist could be lost or truncated on write errors; corrupt history
  disabled saving permanently and created unbounded `.bak` files.
- Concurrent changes (startup + tray + dashboard) interfered with each other's verification.
- The update page reported "downloaded" before anything was downloaded; the last-check time was
  stored in local time.
- "Show notifications" had no effect; ~45 strings were not translated; the tray did not follow the
  language; the fixed 1024×680 window did not fit 1366×768 screens at 125 %.
- Failed addresses were added to the blacklist automatically (removed: the blacklist is user-owned).
- `Announce Release to Discord` and `Create Discussion on Release` never ran for releases published
  by the workflow (`GITHUB_TOKEN` events do not trigger workflows) — they are now dispatched
  explicitly. `SHA256SUMS.txt` could include files from older builds.

### Removed

- The .NET/WinUI 3 solution (`RandomMac.sln`, `src/RandomMac.*`, `tests/RandomMac.Tests`) and
  `global.json` (which pinned an unavailable .NET 11 preview SDK). The 1.x code remains available
  at the `v1.1.3` tag.
- Velopack packaging and the NuGet Dependabot ecosystem.

### Dependencies (merged after 1.1.3, for the 1.x code base)

- `actions/checkout` 4 → 7, `actions/setup-dotnet` 4 → 6, `actions/upload-artifact` 4 → 7,
  `actions/attest-build-provenance` 2 → 4, `actions/github-script` 7 → 9,
  `softprops/action-gh-release` 2 → 3, Microsoft packages, `Microsoft.NET.Test.Sdk` 18.5.1,
  `xunit.runner.visualstudio` 3.1.5 (#3, #5, #6, #7, #9, #12–#17). Dependabot PR #19 (xunit) was
  closed because the C# test project no longer exists.

## [1.1.3] - 2026-05-11

Developer-ergonomics hotfix. No changes to MAC randomization, registry, WMI, or any `RandomMac.Core` logic. Existing `settings.json` / `blacklist.json` / `history.json` files load unchanged.

### Added

- **Social Dev nav page (Ctrl+6).** New 6th navigation entry hosting a **Donate** button (MenuFlyout with Ko-fi / Buy Me a Coffee / Patreon / PayPal), a **Report a Bug** button (deep-links to `issues/new?template=bug_report.yml`), a **Networks** grid (`ItemsRepeater` + `UniformGridLayout`) listing 11 social profiles, and a **Contact** `mailto:` link. URL launching dispatched from code-behind to sidestep WinUI 3 flyout `DataContext` fragility. 7 new localization keys (EN + VI).
- **`.github/FUNDING.yml`.** Activates the repository's "Sponsor" button with five channels: GitHub Sponsors (poli0981), Ko-fi, Buy Me a Coffee, Patreon, PayPal (custom URL).
- **Auto-create GitHub Discussion on each release.** New `.github/workflows/discussion-on-release.yml` runs on `release: published` (parallel to the Discord announcer) and opens a thread in the **Announcements** category via the GraphQL `createDiscussion` mutation. `workflow_dispatch` with a `tag` input supports back-filling discussions for prior releases. Fails with a clear message if Discussions are not enabled or the category is missing.
- **Developer docs.** `docs/pc_spec.md` (developer workstation hardware reference) and `docs/dev_env.md` (personal toolchain — .NET, Python, Node, Rust, Git/GPG). Vietnamese mirrors at `docs/i18n/vi/`. The existing `docs/DEV_ENVIRONMENT.md` (project build instructions) is intentionally unchanged.

### Changed

- **CI no longer runs on documentation-only commits.** `.github/workflows/ci.yml` now declares `paths-ignore` for `docs/**`, `**/*.md`, `.github/FUNDING.yml`, `.github/ISSUE_TEMPLATE/**`, and the root metadata files (`CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `DISCLAIMER.md`, `EULA.md`, `SECURITY.md`, `LICENSE`). Edits under `.github/workflows/**` still trigger CI.
- **`notify-ci-failure.yml` no longer spams on every workflow_run.** Previously watched `workflows: ["*"]` (every completion of any workflow). Now restricted to `["CI", "Release", "Announce Release to Discord"]` with an explicit `if: github.event.workflow_run.conclusion == 'failure'` gate at the job level.
- **Dependabot.** Schedule day shifted `monday` → `tuesday` to avoid the Monday-morning queue. Added `assignees`/`reviewers: [poli0981]` and `commit-message` prefixes (`chore(deps)` for NuGet, `chore(ci)` for github-actions).
- **README** keyboard-shortcut hint `Ctrl+1..5` → `Ctrl+1..6` to reflect the new Social Dev page.

### Fixed

- **`notify-release-pipeline.yml` never fired.** The caller watched `workflows: ["Release Desktop App", "Announce Release to Discord"]` but the actual `name:` field in `release.yml` is just `"Release"` — `workflow_run` is a strict string match. Names corrected to `["Release", "Announce Release to Discord"]`. The reusable downstream notification (Discord) will now actually run after each release pipeline completes.

## [1.1.2] - 2026-05-08

Two correctness fixes around app instancing and the system-tray menu, plus tray-menu polish (Vietnamese localization + restored Fluent icons).

### Fixed

- **Process duplication on every launch.** Each time the user opened `RandomMac.App.exe` while the previous instance was still resident in the tray (default `MinimizeToTray=true` keeps it alive on close), a new process started. Over time, Task Manager filled up with parallel `RandomMac.App.exe` entries — none of them visibly active. Added a single-instance gate in `Program.cs` using `Microsoft.Windows.AppLifecycle.AppInstance.FindOrRegisterForKey("RandomMac.SingleInstance")`. Secondary launches call `RedirectActivationToAsync` and exit; the primary's `Activated` handler marshals to the UI dispatcher and calls `TrayIconService.ShowMainWindow()` to bring the existing window back from the tray. `VelopackApp.Build().Run()` still runs first so installer/update CLI args (`--veloapp-*`) bypass the gate as before.
- **Tray right-click menu items did nothing when clicked.** Initially diagnosed as a `XamlRoot` issue (the `--minimized` startup path never activates the window, and `MenuFlyout.ContextFlyout` needs a XamlRoot). The real cause was the H.NotifyIcon dispatch model: in default `ContextMenuMode = PopupMenu`, the library builds a Win32 popup menu from `MenuFlyout.Items` and dispatches selections via the items' **`ICommand`** — `MenuFlyoutItem.Click` events do not fire in this mode (documented in the H.NotifyIcon README, easy to miss). All three menu items now use `Command = new RelayCommand(...)` instead of `item.Click += ...`. `XamlRoot` priming was kept as defense-in-depth: `App.OnLaunched` for `--minimized` startups now performs an off-screen activate (`AppWindow.Move(-32000,-32000)` → `Activate()` → first-Activated handler hides + restores centered position) so a future switch to `ContextMenuMode.SecondWindow` would also work.

### Changed

- **Tray menu localized.** The three items previously rendered hardcoded English ("Show" / "Randomize Active Adapter" / "Exit") regardless of the chosen UI language. New keys `Tray_Show`, `Tray_RandomizeActive`, `Tray_Exit` in `Lang.resx` (en) and `Lang.vi.resx` (vi: "Hiện cửa sổ" / "Đổi MAC adapter đang hoạt động" / "Thoát"), pulled via `Loc.Get(...)` at menu-build time.
- **Tray FontIcon glyphs** rewritten as `\uXXXX` C# escape sequences (`&#xE737;` Window, `&#xE8B1;` Shuffle, `&#xE8BB;` ChromeClose) instead of raw Private-Use-Area chars. Survives any future tooling write that strips PUA codepoints.

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
- **JSON encoding** — `SettingsService` / `HistoryService` / `BlacklistService` now use `JavaScriptEncoder.UnsafeRelaxedJsonEscaping`, so `&` in PnpDeviceId is written as-is instead of being escaped to `\u0026`.

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

[2.0.0]: https://github.com/poli0981/randomizerMAC/releases/tag/v2.0.0
[1.1.3]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.1.3
[1.1.2]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.1.2
[1.1.1]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.1.1
[1.1.0]: https://github.com/poli0981/randomizerMAC/blob/v1.1.1/docs/RELEASE_v1.1.0.md
[1.0.0]: https://github.com/poli0981/randomizerMAC/releases/tag/v1.0.0
