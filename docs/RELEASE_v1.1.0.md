# RANDOM MAC v1.1.0

UI/UX rewrite from Avalonia 11.2 to **WinUI 3 + Microsoft.WindowsAppSDK 1.8 (Fluent Design 2)**, plus a startup-log fix. Logic in `RandomMac.Core` is unchanged from v1.0.0.

Built with C# 13 / .NET 11 (preview) for the App project, .NET 9 for Core.

---

## Highlights

- **Native WinUI 3 shell** with NavigationView (pane permanently open) and Mica backdrop on Windows 11 (Acrylic fallback on Windows 10).
- **Fixed window 880×600** — resize disabled, maximize hidden. Single, predictable layout on every monitor / DPI.
- **Custom titlebar** with the app's accent color, drawn via `ExtendsContentIntoTitleBar`.
- **Self-contained payload** (~225 MB) — no .NET runtime install required, works offline.
- **Single-flight adapter scan** at cold start — the duplicate "Detected adapter:" / "Excluded adapter:" log lines from v1.0.0 are gone.

## What changed under the hood

| Area | v1.0.0 | v1.1.0 |
|------|--------|--------|
| UI framework | Avalonia 11.2 + FluentAvaloniaUI 2.2 | Microsoft.WindowsAppSDK 1.8 (Fluent 2) |
| App TFM | net9.0 | net11.0-windows10.0.26100.0 |
| Core TFM | net9.0 | net9.0 (unchanged) |
| Tray | Avalonia.TrayIcon | H.NotifyIcon.WinUI 2.2 |
| DataGrid | Avalonia.Controls.DataGrid | CommunityToolkit.WinUI.UI.Controls.DataGrid 7.1.2 |
| Localization | `{loc:L Key}` MarkupExtension | `{Binding [Key], Source={StaticResource Loc}}` |
| File pickers | Avalonia.Platform.Storage | Windows.Storage.Pickers + InitializeWithWindow |
| Adapter scan | 2× concurrent at startup (bug) | Single-flight via `IAdapterCacheService` |

## What did NOT change

- Every algorithm in `RandomMac.Core`: MAC generation, registry write, WMI restart, blacklist, history, settings, OUI lookup, update service.
- Velopack 0.0.1298 packaging (unpackaged, self-contained).
- `requireAdministrator` manifest (no UAC at boot via Task Scheduler).
- All 116 localization keys in `Lang.resx` / `Lang.vi.resx`.
- All 37 RandomMac.Tests unit tests still pass.

## Known soft warnings

- **40× MVVMTK0045** during build — informational note that fields with `[ObservableProperty]` are not AOT-compatible in WinRT scenarios. Non-blocking; does not affect non-AOT builds. Conversion to `partial` properties (C# 13) is a future polish item.
- **NETSDK1057** — preview SDK warning. Expected because v1.1.0 targets .NET 11 preview.

## Smoke checklist (manual)

1. Launch `RandomMac.App.exe` (admin). Window should open exactly 880×600, no maximize button, fixed size.
2. Switch theme (Dark↔Light) and language (English↔Vietnamese) in Settings — UI updates live.
3. Minimize → tray icon visible, click → window restored. Tray menu Exit closes the process.
4. Tail `%LOCALAPPDATA%\RandomMac\logs\log-*.txt` after a cold start: each adapter logs exactly once. Click Refresh on Dashboard → exactly one more block.
5. Apply MAC: success notification appears; history grid gains a row.
6. Settings → Auto-change on startup → tick adapter → restart app. Log shows one scan + the auto-change attempt.

## Compatibility

- Windows 11 (Mica), Windows 10 22H2 (Acrylic fallback).
- x64 only (no ARM64 build in this release).
- Requires .NET 11 preview SDK to build from source; runtime is bundled in the self-contained output.
