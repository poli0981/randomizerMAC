# RandomMac 2.0.1

A maintenance release. RandomMac works exactly as 2.0.0: changing, restoring and recording
addresses are unchanged, and your settings, history and blacklist are kept.

## What's new

- **Safer update downloads** — update checks and downloads use an updated certificate check
  (rustls-webpki 0.103.16) that rejects certificate chains through the distrusted emSign
  intermediates ([RUSTSEC-2026-0382](https://github.com/rustsec/advisory-db/blob/main/crates/rustls-webpki/RUSTSEC-2026-0382.md)).
- **Newer build tools** — the app is now built with Node.js 26, and its interface code is
  type-checked by the native TypeScript 7 compiler on every change and release.

Full list: [CHANGELOG](https://github.com/poli0981/randomizerMAC/blob/master/CHANGELOG.md#201---2026-10-22).

## Updating

- **From 2.0.0:** RandomMac offers this update on its *Update* page. It is signed and verified
  before it is installed: the Windows installers and the AppImage update in place; `.deb`/`.rpm`
  are installed with `apt`/`dnf` after asking for your password. You can also install a package
  from the table below over 2.0.0.
- **From 1.x (Windows):** the 1.x updater cannot install 2.x — follow
  [Upgrading from 1.x](https://github.com/poli0981/randomizerMAC#upgrading-from-1x) once.

## Downloads

| System | File |
|---|---|
| Windows 10/11 x64 (recommended) | `RandomMac_2.0.1_x64-setup.exe` |
| Windows 10/11 x64 (MSI) | `RandomMac_2.0.1_x64_en-US.msi` |
| Ubuntu 24.04+ / Debian 13+ | `RandomMac_2.0.1_amd64.deb` — `sudo apt install ./RandomMac_2.0.1_amd64.deb` |
| Fedora 41+ | `RandomMac-2.0.1-1.x86_64.rpm` — `sudo dnf install ./RandomMac-2.0.1-1.x86_64.rpm` |
| Other Linux x86_64 | `RandomMac_2.0.1_amd64.AppImage` |
| CLI only | `randommac-cli-2.0.1-x86_64-pc-windows-msvc.zip`, `randommac-cli-2.0.1-x86_64-unknown-linux-gnu.tar.gz` |

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
