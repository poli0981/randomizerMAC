# Development Environment

## Prerequisites

| Tool | Version | Required |
|------|---------|----------|
| [.NET 11 SDK preview](https://dotnet.microsoft.com/download/dotnet/11.0) | ≥ `11.0.100-preview.3` | Yes (App project) |
| [.NET 9 SDK](https://dotnet.microsoft.com/download/dotnet/9.0) | 9.0+ | Yes (Core project + tests) |
| [Git](https://git-scm.com/) | 2.x+ | Yes |
| Windows 10 SDK 26100 | Bundled | Auto-installed via `Microsoft.Windows.SDK.BuildTools` package |
| IDE | See below | Yes |

`global.json` at the repo root pins the SDK to `11.0.100-preview.3` with `rollForward: latestFeature`. If multiple SDKs are installed, this avoids accidental version mismatches.

### Recommended IDE

| IDE | Version | Notes |
|-----|---------|-------|
| [JetBrains Rider](https://www.jetbrains.com/rider/) | 2026.1+ | Recommended. Native WinUI 3 / WindowsAppSDK support. |
| [Visual Studio](https://visualstudio.microsoft.com/) | 2022 17.10+ | Install the "Windows App SDK C# Templates" workload. |
| [VS Code](https://code.visualstudio.com/) | Latest | C# Dev Kit; XAML IntelliSense limited. |

## Build & Run

```bash
# Clone
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC

# Restore dependencies
dotnet restore

# Build (csproj sets RuntimeIdentifier=win-x64 by default)
dotnet build

# Run (requires Administrator for full functionality — MAC change needs elevation)
dotnet run --project src/RandomMac.App

# Run unit tests
dotnet test
```

> **Note**: `RandomMac.App.csproj` declares `<RuntimeIdentifier>win-x64</RuntimeIdentifier>` as a default fallback so plain `dotnet build` and IDE "Build Solution" work without flags. Override with `-r <rid>` when needed.

## Publish (Self-Contained)

```bash
# Windows x64 (only architecture supported in v1.1.x)
dotnet publish src/RandomMac.App -c Release -r win-x64 --self-contained true -o publish/
```

The published output is ~225 MB. The WinAppSDK runtime + .NET 11 are bundled (`<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` + `<SelfContained>true</SelfContained>`); no runtime install needed on the target machine.

ARM64 is not currently a supported target — the WinAppSDK packages we use are x64-tested only.

## Project Structure

```
RandomMac.sln                          # Solution
global.json                            # SDK pin (11.0.100-preview.3)
src/
  RandomMac.Core/                      # Business logic, no UI dependency (net9.0)
    Models/                            # MacAddress, AppSettings, MacChangeResult, …
    Services/
      Interfaces/                      # Service contracts (IAdapterCacheService, …)
      Implementations/                 # NetworkAdapter, MacAddress, Blacklist, History, Settings, Update, AdapterCache
    Helpers/                           # MacAddressGenerator, RegistryHelper, OuiLookup, AdminHelper

  RandomMac.App/                       # WinUI 3 desktop app (net11.0-windows10.0.26100.0)
    App.xaml(.cs)                      # Application entry, DI bootstrap, OnLaunched flow
    Program.cs                         # Custom Main (Velopack init before XAML start)
    MainWindow.xaml(.cs)               # Window shell, NavigationView, custom titlebar, tray
    Themes/Theme.xaml                  # ThemeDictionaries (Default/Light) + design tokens
    Views/                             # Page UserControls + PageTemplateSelector
    ViewModels/                        # MVVM ViewModels (CommunityToolkit.Mvvm source-generated)
    Controls/NotificationPopup        # Toast container UserControl
    Converters/                        # IValueConverter implementations
    Services/                          # Theme, Tray, Notification, LogSink
    Helpers/Win32FileDialog            # comdlg32 wrapper (replaces FileSavePicker/FileOpenPicker)
    Localization/                      # Lang.resx + Lang.vi.resx + Loc.cs (singleton + indexer)
    app.manifest                       # requireAdministrator + DPI awareness PerMonitorV2

tests/
  RandomMac.Tests/                     # xUnit (net9.0)

docs/                                  # Documentation
  DEV_ENVIRONMENT.md                   # This file
  RELEASE_v1.0.0.md                    # v1.0.0 release notes
  RELEASE_v1.1.0.md                    # v1.1.0 release notes
.github/                               # Issue templates, PR template, CODEOWNERS
```

## Architecture

- **Pattern**: MVVM with [CommunityToolkit.Mvvm](https://learn.microsoft.com/en-us/dotnet/communitytoolkit/mvvm/) (source generators for `[ObservableProperty]` / `[RelayCommand]`).
- **DI**: `Microsoft.Extensions.DependencyInjection`. Configured in `App.xaml.cs::ConfigureServices`.
- **UI**: WinUI 3 + Microsoft.WindowsAppSDK 1.8 (Fluent Design 2). Unpackaged + `WindowsAppSDKSelfContained=true`.
- **Logging**: Serilog with two sinks — rolling daily file (`%LOCALAPPDATA%\RandomMac\logs\log-YYYY-MM-DD.txt`, 7-day retention) + custom `LogEntrySink` for the in-app log viewer.
- **Updates**: Velopack (when packaged) with a GitHub Releases API fallback for dev builds. Auto-check on launch is throttled by `Settings.LastUpdateCheckedAt` (24h cooldown).
- **MAC Change**: Windows Registry `NetworkAddress` write + WMI `Win32_NetworkAdapter.Disable/Enable` cycle, with post-change verification.
- **Adapter Cache**: `IAdapterCacheService` (Singleton, `SemaphoreSlim` single-flight) — warmed once at `App.OnLaunched`, prevents duplicate WMI scans from concurrent VM constructors.
- **Custom Main**: opt out of XAML-generated `Main` via `DISABLE_XAML_GENERATED_MAIN`; `Program.cs` runs `VelopackApp.Build().Run()` *before* `Application.Start`, then constructs `App`.
- **File dialogs**: Win32 `comdlg32` `GetOpenFileName`/`GetSaveFileName` (in-process) instead of `Windows.Storage.Pickers` (broker-mediated, fails under `requireAdministrator`).

## Key Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| Microsoft.WindowsAppSDK | 1.8.* | WinUI 3 + Fluent Design 2 platform |
| CommunityToolkit.WinUI.UI.Controls.DataGrid | 7.1.2 | DataGrid for WinUI 3 (last published; v8 dropped DataGrid) |
| H.NotifyIcon.WinUI | 2.2.* | System tray icon (replaces `Avalonia.TrayIcon`) |
| CommunityToolkit.Mvvm | 8.4.* | MVVM source generators |
| Microsoft.Extensions.DependencyInjection | 9.0.* | DI container |
| Serilog | 4.2.* | Structured logging |
| Velopack | 0.0.1298 | Auto-update + packaging |
| System.Management | 9.0.* | WMI adapter control (`RandomMac.Core` only) |

## WinUI 3 Pitfalls (Hard-Won Lessons)

These are properties of WinAppSDK 1.8 itself; documented here so future contributors don't rediscover them. See also `memory/feedback_patterns.md` (developer-local).

| Issue | Symptom | Workaround |
|-------|---------|------------|
| Custom `MarkupExtension` returning `Binding` | `XamlCompiler.exe` exits with code 1, **silent**, 0-byte XBF for affected XAML page | Drop the markup extension; bind via `{Binding [Key], Source={StaticResource Loc}}` against an Application resource. |
| `IsEnabled` bound on `Border` (or any non-`Control` `FrameworkElement`) | Same silent XamlCompiler crash; XBF stays at previous build's size | Move the binding to a `Control` descendant inside the `Border`. |
| `<PathGeometry x:Key Figures="M..."/>` | Same silent XamlCompiler crash | Inline `<PathIcon Data="M..."/>` per usage, or use `<FontIcon Glyph="&#xXXXX;" FontFamily="Segoe Fluent Icons"/>`. |
| Inline FontIcon `Glyph="..."` with Private-Use-Area chars | Glyph rendered as empty | Use XML entity references (`&#xE72C;`) in XAML attribute values. |
| `DataGridColumn.Header` `{Binding ...}` | Header text reads `Microsoft.UI.Xaml.Data.Binding` literally | Set headers from code-behind on `Loaded` + subscribe to `Loc.PropertyChanged`. |
| `Application.Resources["X"]` from App constructor | `COMException 0x8000FFFF` (E_UNEXPECTED) at `get_Resources()` | Defer to the start of `OnLaunched`. |
| `Windows.Storage.Pickers.FileSavePicker` / `FileOpenPicker` in elevated unpackaged process | "The parameter is incorrect" / dialog never opens / returns null | Use Win32 `comdlg32` `GetOpenFileName`/`GetSaveFileName` (`Helpers/Win32FileDialog.cs`). |
| Cache events fire on threadpool | UI-bound `ObservableCollection` mutated cross-thread → InvalidOperation | Subscribers marshal via `App.MainDispatcher.TryEnqueue` when `!HasThreadAccess`. |

## Developer Machine (Primary)

| Component | Details |
|-----------|---------|
| **OS** | Windows 11 Pro 25H2 Insider Preview (Dev Channel) |
| **Build** | 26300.8155 |
| **CPU** | Intel Core i7-14700KF |
| **GPU** | NVIDIA GeForce RTX 5080 (16 GB VRAM) |
| **RAM** | 32 GB DDR5 |
| **Storage** | 1 TB SSD |
| **IDE** | JetBrains Rider 2026.1 |

## Tested Configurations

### Physical Machine
- Developer machine (specs above) — **Passed**

### Virtual Machines

| Environment | OS | Specs | Result |
|-------------|-----|-------|--------|
| Oracle VirtualBox 7.2.6 | Windows 10 22H2 (Build 19045.3803) | 4 GB RAM, 2 CPUs, 128 MB VRAM | **Passed** (v1.0.0); v1.1.x falls back to Acrylic backdrop |
| Windows Sandbox | Windows 11 (host version) | Default | **Passed** |

## Data Storage Locations

All data is stored locally under `%LOCALAPPDATA%\RandomMac\`:

| File | Purpose |
|------|---------|
| `settings.json` | Application settings |
| `blacklist.json` | MAC address blacklist (global + per-adapter) |
| `history.json` | MAC change history (atomic save via `.tmp`+`File.Move`; 30-day retention; max 100 entries) |
| `history.json.bak.<timestamp>` | Auto-backup if `history.json` fails to parse on load |
| `logs/log-YYYY-MM-DD.txt` | Rolling daily log files (7-day retention) |

## AI Tooling

This project uses AI-assisted development:

| Phase | Model | Usage |
|-------|-------|-------|
| v1.0.0 | Claude Opus 4.6 (Anthropic) via [Claude Code](https://claude.ai/claude-code) | Code generation, debugging, testing, docs, translations |
| v1.1.0+ | Claude Opus 4.7 (Anthropic) via [Claude Code](https://claude.ai/claude-code) | Migration to WinUI 3, UX polish, bug fixes, docs updates |

All AI output is reviewed and approved by the developer before merge.

## Coding Conventions

- Follow standard [.NET naming conventions](https://learn.microsoft.com/en-us/dotnet/csharp/fundamentals/coding-style/coding-conventions).
- All user-facing strings must be localized via `Loc.Get("Key")` (C#) or `{Binding [Key], Source={StaticResource Loc}}` (XAML).
- New localization keys must be added to both `Lang.resx` (English) and `Lang.vi.resx` (Vietnamese).
- Use `[ObservableProperty]` and `[RelayCommand]` attributes (CommunityToolkit.Mvvm source generators).
- Services are registered in `App.xaml.cs::ConfigureServices`.
- File dialogs go through `Helpers/Win32FileDialog` — never `Windows.Storage.Pickers` directly (see WinUI 3 pitfalls table).
- FontIcon glyphs in XAML use entity references (`&#xE72C;`); glyphs in C# strings use `\uXXXX` escapes.
