# RANDOM MAC v1.1.2

A small but important bugfix release on top of v1.1.1. Two correctness fixes around app instancing and the system-tray menu, plus tray-menu localization + restored Fluent icons.

Application logic in `RandomMac.Core` is unchanged. Existing `settings.json` / `blacklist.json` / `history.json` files load without migration.

Built with C# 13 / .NET 11 (preview) for the App project, .NET 9 for Core.

---

## Bug fixes

- **Process duplication on every launch.** Every time you opened RANDOM MAC while the previous instance was still resident in the tray (default `MinimizeToTray=true` keeps the process alive when you close the window), a new `RandomMac.App.exe` process started. Over hours of use, Task Manager filled with parallel processes — none of them visibly active.
  Fixed via a single-instance gate in `Program.cs` using `Microsoft.Windows.AppLifecycle.AppInstance.FindOrRegisterForKey`. Secondary launches now redirect activation to the primary instance and exit; the primary's `Activated` handler marshals to the UI dispatcher and brings the existing window back from the tray. Velopack's `--veloapp-*` install/update lifecycle still runs first and is unaffected.
- **Tray right-click menu items didn't fire.** The right-click menu showed all 3 items (Show / Randomize Active Adapter / Exit) with correct text and icons, but clicking any of them did nothing.
  Root cause: H.NotifyIcon's default `ContextMenuMode = PopupMenu` builds a Win32 popup from `MenuFlyout.Items` and dispatches selections via each item's **`ICommand`** — `MenuFlyoutItem.Click` events do **not** fire in this mode (documented in the H.NotifyIcon README, easy to miss because the WinUI MenuFlyout API exposes both). All three items now use `Command = new RelayCommand(...)` instead of `item.Click += ...`.
  As defense-in-depth, the `--minimized` startup path also now primes the `MenuFlyout`'s `XamlRoot` via an off-screen activate-then-hide, so a future switch to `ContextMenuMode.SecondWindow` would work without further changes.

## Changed

- **Tray menu localized.** Items used to render hardcoded English regardless of the chosen UI language. New keys `Tray_Show`, `Tray_RandomizeActive`, `Tray_Exit` in `Lang.resx` (en) and `Lang.vi.resx` (vi: "Hiện cửa sổ" / "Đổi MAC adapter đang hoạt động" / "Thoát"), pulled via `Loc.Get(...)` at menu-build time.
- **Tray FontIcon glyphs** rewritten as `\uXXXX` C# escape sequences (`&#xE737;` Window, `&#xE8B1;` Shuffle, `&#xE8BB;` ChromeClose) instead of raw Private-Use-Area chars, so they survive any future tooling write that strips PUA codepoints.

## What did NOT change

- Every algorithm in `RandomMac.Core`: MAC generation, registry write, WMI restart, blacklist, history, settings, OUI lookup, update service.
- Velopack 0.0.1298 packaging (unpackaged, self-contained).
- `requireAdministrator` manifest (no UAC at boot via Task Scheduler).
- All 37 RandomMac.Tests unit tests still pass.

## Smoke checklist (manual)

1. Launch as Administrator. Window opens, tray icon appears.
2. Close the window with `MinimizeToTray=true` (default). Process stays in the tray.
3. Double-click the desktop shortcut (or run the exe again) — the existing window comes back to the foreground; **Task Manager shows exactly one `RandomMac.App.exe`**, not two.
4. Repeat step 3 several times — process count stays at 1.
5. Right-click the tray icon → menu shows three localized items with Fluent icons.
6. Click "Hiện cửa sổ" / "Show" → window appears centered on screen.
7. Close → returns to tray. Right-click → "Đổi MAC adapter đang hoạt động" / "Randomize Active Adapter" → notification confirms MAC change.
8. Right-click → "Thoát" / "Exit" → process exits cleanly.
9. Settings → enable Run at Startup + Start Minimized. Sign out and back in (or run the scheduled task manually). Window does **not** flash on screen, tray icon appears, right-click menu works on first try.

## Compatibility

- Windows 11 (Mica) / Windows 10 22H2 (Acrylic fallback) / older (solid backdrop).
- x64 only. ARM64 not supported in this release.
- Bundled WinAppSDK runtime + .NET 11 (~225 MB self-contained).
- Building from source needs the .NET 11 preview SDK (`global.json` pinned); see [DEV_ENVIRONMENT.md](DEV_ENVIRONMENT.md).
