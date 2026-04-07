# Development Environment

## Prerequisites

| Tool | Version | Required |
|------|---------|----------|
| [.NET SDK](https://dotnet.microsoft.com/download/dotnet/9.0) | 9.0+ | Yes |
| [Git](https://git-scm.com/) | 2.x+ | Yes |
| IDE | See below | Yes |

### Recommended IDE

| IDE | Version | Notes |
|-----|---------|-------|
| [JetBrains Rider](https://www.jetbrains.com/rider/) | 2026.1+ | Recommended. Best Avalonia support. |
| [Visual Studio](https://visualstudio.microsoft.com/) | 2022+ | Install "Desktop development with .NET" workload |
| [VS Code](https://code.visualstudio.com/) | Latest | With C# Dev Kit + Avalonia extensions |

## Build & Run

```bash
# Clone
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC

# Restore dependencies
dotnet restore

# Build
dotnet build

# Run (requires Administrator for full functionality)
dotnet run --project src/RandomMac.App

# Run tests
dotnet test
```

## Publish (Self-Contained)

```bash
# Windows x64
dotnet publish src/RandomMac.App -c Release -r win-x64 --self-contained true -o publish/

# Windows ARM64
dotnet publish src/RandomMac.App -c Release -r win-arm64 --self-contained true -o publish/
```

Output will be in the `publish/` directory. The published app does not require .NET runtime on the target machine.

## Project Structure

```
RandomMac.sln                    # Solution file
src/
  RandomMac.Core/                # Business logic (no UI dependency)
    Models/                      # Data models (MacAddress, AppSettings, etc.)
    Services/
      Interfaces/                # Service contracts
      Implementations/           # WMI, Registry, Blacklist, Update services
    Helpers/                     # MacAddressGenerator, RegistryHelper, OuiLookup
    Localization/                # Core localization resources

  RandomMac.App/                 # Avalonia desktop application
    Views/                       # AXAML views (MainWindow, Dashboard, Settings, etc.)
    ViewModels/                  # MVVM ViewModels
    Controls/                    # Custom controls (NotificationPopup)
    Converters/                  # XAML value converters
    Services/                    # App services (Theme, Tray, Notification, LogSink)
    Localization/                # UI strings (Lang.resx, Lang.vi.resx, Loc.cs)
    Styles/                      # Theme resources and icon geometries
    Assets/                      # App icon, fonts

tests/
  RandomMac.Tests/               # xUnit unit tests
```

## Architecture

- **Pattern**: MVVM with CommunityToolkit.Mvvm (source generators)
- **DI**: Microsoft.Extensions.DependencyInjection
- **UI**: Avalonia 11.2 with FluentAvalonia
- **Logging**: Serilog (file sink + observable sink for real-time UI)
- **Updates**: Velopack + GitHub Releases API fallback
- **MAC Change**: Windows Registry (`NetworkAddress`) + WMI adapter restart

## Key Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| Avalonia | 11.2.x | UI framework |
| FluentAvaloniaUI | 2.2.x | Fluent Design controls |
| CommunityToolkit.Mvvm | 8.4.x | MVVM source generators |
| Serilog | 4.2.x | Structured logging |
| Velopack | 0.0.x | Auto-update packaging |
| System.Management | 9.0.x | WMI adapter control |

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
- Developer machine (specs above) - **Passed**

### Virtual Machines

| Environment | OS | Specs | Result |
|-------------|-----|-------|--------|
| Oracle VirtualBox 7.2.6 | Windows 10 22H2 (Build 19045.3803) | 4 GB RAM, 2 CPUs, 128 MB VRAM | **Passed** |
| Windows Sandbox | Windows 11 (host version) | Default | **Passed** |

## Data Storage Locations

All data is stored locally under `%LOCALAPPDATA%\RandomMac\`:

| File | Purpose |
|------|---------|
| `settings.json` | Application settings |
| `blacklist.json` | MAC address blacklist (global + per-adapter) |
| `history.json` | MAC change history |
| `logs/log-YYYY-MM-DD.txt` | Rolling daily log files (7-day retention) |

## AI Tooling

This project uses AI-assisted development:

| Tool | Model | Usage |
|------|-------|-------|
| [Claude Code](https://claude.ai/claude-code) | Claude Opus 4.6 (Anthropic) | Code generation, debugging, testing, docs, translations |

All AI output is reviewed and approved by the developer before merge.

## Coding Conventions

- Follow standard [.NET naming conventions](https://learn.microsoft.com/en-us/dotnet/csharp/fundamentals/coding-style/coding-conventions)
- All user-facing strings must be localized via `Loc.Get("Key")` (C#) or `{loc:L Key}` (AXAML)
- New localization keys must be added to both `Lang.resx` (English) and `Lang.vi.resx` (Vietnamese)
- Use `[ObservableProperty]` and `[RelayCommand]` attributes (CommunityToolkit.Mvvm source generators)
- Services should be registered in `App.axaml.cs` ConfigureServices method
