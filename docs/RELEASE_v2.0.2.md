# RandomMac 2.0.2

A maintenance release with two bug fixes and up-to-date dependencies. Changing, restoring and
recording addresses work as in 2.0.1, and your settings, history and blacklist are kept.

## What's new

- **Leaner, safer updater** — the update component no longer contains the old zip 4 library,
  which has a critical advisory ([GHSA-m665-v7pm-7grj](https://github.com/advisories/GHSA-m665-v7pm-7grj)).
  2.0.1 was not affected in practice (it only installs signed installers, never zip archives), but
  that code is no longer shipped.
- **Windows: autostart in folders with `&`** — when RandomMac is installed in a folder whose name
  contains `&`, it no longer re-creates its autostart task at every start.
- **1.x import count** — importing RandomMac 1.x data reports only the history entries it keeps;
  entries older than the retention period are no longer counted.
- **Updated components** — Tauri plugins, the Rust crates and the interface libraries are on their
  latest releases.

Full list: [CHANGELOG](https://github.com/poli0981/randomizerMAC/blob/master/CHANGELOG.md#202---2026-11-04).

## Updating

- **From 2.0.x:** RandomMac offers this update on its *Update* page. It is signed and verified
  before it is installed: the Windows installers and the AppImage update in place; `.deb`/`.rpm`
  are installed with `apt`/`dnf` after asking for your password. You can also install a package
  from the table below over 2.0.x.
- **From 1.x (Windows):** the 1.x updater cannot install 2.x — follow
  [Upgrading from 1.x](https://github.com/poli0981/randomizerMAC#upgrading-from-1x) once.

## Downloads

| System | File |
|---|---|
| Windows 10/11 x64 (recommended) | `RandomMac_2.0.2_x64-setup.exe` |
| Windows 10/11 x64 (MSI) | `RandomMac_2.0.2_x64_en-US.msi` |
| Ubuntu 24.04+ / Debian 13+ | `RandomMac_2.0.2_amd64.deb` — `sudo apt install ./RandomMac_2.0.2_amd64.deb` |
| Fedora 41+ | `RandomMac-2.0.2-1.x86_64.rpm` — `sudo dnf install ./RandomMac-2.0.2-1.x86_64.rpm` |
| Other Linux x86_64 | `RandomMac_2.0.2_amd64.AppImage` |
| CLI only | `randommac-cli-2.0.2-x86_64-pc-windows-msvc.zip`, `randommac-cli-2.0.2-x86_64-unknown-linux-gnu.tar.gz` |

Verify: `sha256sum --ignore-missing -c SHA256SUMS.txt` and
`gh attestation verify <file> --repo poli0981/randomizerMAC`.

## Known issues

- The Windows installers are not code-signed yet; SmartScreen may ask you to confirm
  (*More info → Run anyway*).
- Some Wi-Fi drivers ignore address changes, especially when Windows *Random hardware addresses* is
  on for the network. RandomMac detects this and restores the previous state.
- On GNOME without the AppIndicator extension there is no tray icon; closing the window quits.
- Only x86_64 builds are provided.
- Error details in tray notifications are shown in English.

More help: [README](https://github.com/poli0981/randomizerMAC#readme) ·
[FAQ](https://github.com/poli0981/randomizerMAC#faq) ·
[Report a bug](https://github.com/poli0981/randomizerMAC/issues/new?template=bug_report.yml)
