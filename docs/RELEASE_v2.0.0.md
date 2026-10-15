# RandomMac 2.0.0

RandomMac has been rewritten from scratch in **Rust + Tauri 2** and now runs on **Windows 10/11 and
Linux** (Ubuntu, Fedora and other modern distributions). The installers are ~5 MB instead of
~112 MB, updates are signed, and there is a command-line tool for scripts.

## Highlights

- **Linux support** — NetworkManager (persistent, per connection) or iproute2 (temporary); the
  window never runs as root, polkit asks only for the change itself.
- **`randommac-cli`** — `list`, `show`, `random`, `set`, `restore`, `inspect`, `vendors`,
  `history`, with `--json` and stable exit codes. Installed next to the app and available as a
  standalone archive.
- **Generation modes** — random locally administered (default), keep the current vendor, or
  imitate one of 59 vendors from the IEEE registry.
- **Automation** — change at sign-in, every *N* minutes, restore on exit, *Restore all* in the tray.
- **Safer changes** — every change is verified and rolled back automatically if the driver ignores
  it; *Restore* uses the real hardware address and works at any time; one change at a time.
- **Security fixes** — installed per machine in `Program Files`; the 1.x logon task that ran an
  executable from a user-writable folder with administrator rights is removed.
- **More** — History page with search and CSV/JSON export, blacklist by address or vendor prefix
  (per adapter), disabled adapters can be re-enabled, system theme, resizable window, live log
  viewer, diagnostics, an update switch that really turns update checks off.

Full list: [CHANGELOG](https://github.com/poli0981/randomizerMAC/blob/master/CHANGELOG.md#200---2026-10-15).

## Downloads

| System | File |
|---|---|
| Windows 10/11 x64 (recommended) | `RandomMac_2.0.0_x64-setup.exe` |
| Windows 10/11 x64 (MSI) | `RandomMac_2.0.0_x64_en-US.msi` |
| Ubuntu 24.04+ / Debian 13+ | `RandomMac_2.0.0_amd64.deb` — `sudo apt install ./RandomMac_2.0.0_amd64.deb` |
| Fedora 41+ | `RandomMac-2.0.0-1.x86_64.rpm` — `sudo dnf install ./RandomMac-2.0.0-1.x86_64.rpm` |
| Other Linux x86_64 | `RandomMac_2.0.0_amd64.AppImage` |
| CLI only | `randommac-cli-2.0.0-x86_64-pc-windows-msvc.zip`, `randommac-cli-2.0.0-x86_64-unknown-linux-gnu.tar.gz` |

Verify: `sha256sum --ignore-missing -c SHA256SUMS.txt` and
`gh attestation verify <file> --repo poli0981/randomizerMAC`.

## Upgrading from 1.x (Windows)

The 1.x updater cannot install 2.0. Upgrade once by hand:

1. **Install 2.0 first.** On first start it imports your 1.x settings, history and blacklist and
   removes the old startup task.
2. **Then uninstall "RANDOM MAC" 1.x** (Settings → Apps). Uninstalling 1.x deletes
   `%LOCALAPPDATA%\RandomMac`, so do it only after step 1.

Addresses applied by 1.x stay active and can be restored from 2.0.

## Known issues

- The Windows installers are not code-signed yet; SmartScreen may ask you to confirm
  (*More info → Run anyway*).
- Some Wi-Fi drivers ignore address changes, especially when Windows *Random hardware addresses* is
  on for the network. RandomMac detects this and restores the previous state.
- On GNOME without the AppIndicator extension there is no tray icon; closing the window quits.
- Only x86_64 builds are provided.

More help: [README](https://github.com/poli0981/randomizerMAC#readme) ·
[FAQ](https://github.com/poli0981/randomizerMAC#faq) ·
[Report a bug](https://github.com/poli0981/randomizerMAC/issues/new?template=bug_report.yml)
