# RANDOM MAC v1.1.1

UX polish + correctness fixes on top of the WinUI 3 framework migration that landed in v1.1.0. Application logic in `RandomMac.Core` is unchanged. Existing `settings.json` / `blacklist.json` / `history.json` files load without migration.

Built with C# 13 / .NET 11 (preview) for the App project, .NET 9 for Core.

---

## Highlights

- **Tray quick action** — right-click the tray icon → "Randomize Active Adapter" applies a fresh MAC to the first connected adapter without opening the window.
- **Hamburger toggle** — collapse the navigation pane to icon-only (48px).
- **Auto-apply Settings** — every toggle / combo applies immediately and persists with a 500ms debounce. The "Save" button is gone.
- **Window 1024×680** — was 880×600. More breathing room for the DataGrid history and the Settings sections.
- **Auto-update check on launch** — silent, throttled to once every 24h. Surfaces a toast if a new version is available.
- **History retention** — entries older than 30 days are pruned automatically; saves are atomic via `.tmp`+`File.Move`; corrupt files are backed up to `.bak.<ts>` before any new write.
- **Bundle export** — Settings → "Export All (zip)" produces a single ZIP with `settings.json` + `blacklist.json` + `history.json` for backup or migrating between machines.
- **History filter** — live search above the Recent History grid by adapter name or MAC string.
- **Keyboard shortcuts** — `Ctrl+1..5` to jump pages, `Ctrl+R` Randomize, `Ctrl+Enter` Apply.
- **Empty state + status InfoBars** — Dashboard shows a "Pick an adapter" hint when nothing is selected; status messages appear in an InfoBar that auto-clears after 5 seconds.
- **Polish** — page transitions (slide+fade), Update view rewrite (state-driven hero card with last-checked), About rewrite (centered hero + compact dependency list), connection icon + text (was a 10px dot), WiFi/Ethernet glyphs in the adapter dropdown, relative timestamps in history ("just now" / "5 min ago" / "yesterday" / `yyyy-MM-dd HH:mm`), Mica + Acrylic backdrop fallback chain, scrollbars hidden but scroll input still works.

## Bug fixes

- **DataGrid headers** in Recent History showed the literal string `Microsoft.UI.Xaml.Data.Binding`. `DataGridColumn` is not in the visual tree, so XAML `{Binding}` on `Header` doesn't resolve. Headers are now set from `DashboardView.xaml.cs` and refreshed on `Loc.PropertyChanged`.
- **Dashboard Refresh** always reported "Error loading adapters". `IAdapterCacheService.RefreshAsync` uses `ConfigureAwait(false)`, so subscribers ran on the threadpool and mutated the bound `ObservableCollection` cross-thread. Subscribers now marshal through `App.MainDispatcher.TryEnqueue`.
- **Recent History timestamps always read "7 h ago"** on UTC+7 machines. `Timestamp` defaults to `DateTime.UtcNow` (correct), but the converter computed `DateTime.Now - when` (`local - utc`). Fixed by normalizing the input via `Kind`-aware `ToLocalTime()`.
- **Recent History wiped when in-memory list happened to be empty.** `LoadAsync` used to swallow JSON parse failures; the next `SaveAsync` then wrote the empty list, overwriting the user's on-disk entries. Hardened with a `_loadCompleted` gate, corrupt-file backup, and atomic `.tmp`+`File.Move` writes.
- **FontIcon glyphs invisible** — raw Private-Use-Area chars in XAML attribute values were silently stripped to empty by the file write pipeline. Replaced all inline glyphs with XML entity references (`&#xE72C;`).
- **Hamburger collapse left "first letter peek"** — custom `MenuItemTemplate` rendered the label TextBlock independently of the auto-collapse logic. Added `IsPaneOpen` two-way binding on `MainWindowViewModel` and a `NavItem.Owner` back-reference so the template can hide the label.
- **Tray right-click menu items inert + double-click no-op + window not focused on Show**. Added `XamlRoot` lazy-init for `MenuFlyout`, `DoubleClickCommand`, and Win32 `ShowWindow(SW_RESTORE)` + `SetForegroundWindow`.
- **LogView "Failed to export log: File extensions must begin with '.' and contain no wildcards"**. WinUI 3 `FileSavePicker.FileTypeChoices` rejects `.*`. Removed the wildcard entry.
- **Settings export / import / bundle export consistently failed in elevated unpackaged WinUI 3.** `Windows.Storage.Pickers.FileSavePicker`/`FileOpenPicker` route through a broker that can't elevate cross-process. All four pickers (Settings export, Settings import, Bundle export, Log export) now use a Win32 `comdlg32` wrapper (`Helpers/Win32FileDialog.cs`) running in-process. Output buffer uses `IntPtr` instead of `StringBuilder` so the `OPENFILENAMEW` struct is blittable for `Marshal.SizeOf`.
- **`Resources["Loc"] = Loc.Instance` crashed at startup** with `COMException 0x8000FFFF`. The COM proxy isn't fully wired in the App constructor even after `InitializeComponent()`. Moved to the start of `OnLaunched`.
- **Plain `dotnet build` failed** with `WindowsAppSDKSelfContained requires a supported Windows architecture`. Added a default `RuntimeIdentifier=win-x64` in the csproj so Rider/VS builds work without `-r` flags.

## Removed

- The 6-color accent picker. Brand accent is hardcoded to `#61AFEF`. The `AccentColor` field is dropped from `AppSettings`; existing `settings.json` files load without migration (System.Text.Json ignores unknown keys).
- The "Save" button from Settings (everything auto-applies + auto-saves now).
- The "All Files (`.*`)" choice from LogView's export dialog (WinUI 3 picker rejects wildcards).
- Dead `accentColor` parameter from `ThemeService.Apply(string mode, string accentColor)` → now `Apply(string mode)`.

## What did NOT change

- Every algorithm in `RandomMac.Core`: MAC generation, registry write, WMI restart, blacklist, history, settings, OUI lookup, update service.
- Velopack 0.0.1298 packaging (unpackaged, self-contained).
- `requireAdministrator` manifest (no UAC at boot via Task Scheduler).
- All 116 localization keys in `Lang.resx` / `Lang.vi.resx`.
- All 37 RandomMac.Tests unit tests still pass.

## Smoke checklist (manual)

1. Launch as Administrator. Window opens at 1024×680, fixed, no maximize.
2. Toggle theme Dark↔Light and language EN↔VI in Settings — UI re-renders live, no Save click needed.
3. Press `Ctrl+1..5` from anywhere — jumps to corresponding nav page.
4. Minimize → tray icon visible. Double-click → window restored to foreground.
5. Right-click tray → menu shows Show / Randomize Active Adapter / Exit. Each one fires.
6. Apply MAC on Dashboard → Recent History row's Time column reads "just now". After ~5 min → "5 min ago".
7. Filter Recent History by adapter name → grid filters live.
8. Settings → Export → save dialog opens with JSON filter; pick a path → file written.
9. Settings → Import → open dialog → pick a file → settings reloaded.
10. Settings → Export All (zip) → save → unzip the result → `settings.json` + `blacklist.json` + `history.json` inside.
11. Log → Export Log → save dialog with `.txt` / `.log` choices → file written.
12. Tail `%LOCALAPPDATA%\RandomMac\logs\log-*.txt` after cold launch — each adapter logs exactly once.
13. Restart app — log shows `Auto update check: status=…` after ~5 seconds (skipped silently if last check < 24h ago).

## Compatibility

- Windows 11 (Mica) / Windows 10 22H2 (Acrylic fallback) / older (solid backdrop).
- x64 only. ARM64 not supported in this release.
- Bundled WinAppSDK runtime + .NET 11 (~225 MB self-contained).
- Building from source needs the .NET 11 preview SDK (`global.json` pinned); see [DEV_ENVIRONMENT.md](DEV_ENVIRONMENT.md).
