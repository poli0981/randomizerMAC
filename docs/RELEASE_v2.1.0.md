# RandomMac 2.1.0

The last feature release: RandomMac now sets **DNS servers** per adapter too, and it is easier to
use with a keyboard, a screen reader, zoom and Windows contrast themes. Your settings, history and
blacklist from 2.0.x are kept.

From this release on, RandomMac is in **maintenance mode**: it keeps getting dependency and
security updates and fixes for bugs, accessibility and translations, but no new features. See
[Project status](https://github.com/poli0981/randomizerMAC#project-status).

## What's new

- **DNS page** — choose the DNS servers of each adapter: a built-in provider (Cloudflare, Google,
  Quad9, AdGuard, CleanBrowsing, Control D, DNS4EU, DNS.SB, Cisco OpenDNS) with its filtering
  option, your own servers or presets, or a resolver running on your computer. IPv4 and IPv6.
- **Encrypted DNS** — DNS over HTTPS on Windows 11 and Windows Server 2022 or later; DNS over TLS
  on Linux when NetworkManager uses systemd-resolved. DNSCrypt stamps are ready to copy for
  dnscrypt-proxy.
- **Safe changes** — every change is read back, and when the system does not keep it RandomMac
  puts the previous settings back. **Restore original** returns to the settings from before
  RandomMac's first DNS change. Every change is in History.
- **Latency test** — see how fast each provider answers from your computer, and pick one from the
  results.
- **Tray and command line** — a DNS submenu in the tray, and `randommac-cli dns` (providers, show,
  set, auto, restore, test).
- **Accessibility** — a *Skip to content* link, better focus handling, step-by-step announcements
  for screen readers, higher contrast, Windows contrast themes, reduced motion and window zoom
  (Ctrl+= / Ctrl+− / Ctrl+0). Read the
  [accessibility statement](https://github.com/poli0981/randomizerMAC/blob/master/ACCESSIBILITY.md)
  and tell us about barriers.
- **Shortcuts** — the DNS page is Ctrl+2; History, Settings, Log, Update, About and Social moved
  to Ctrl+3 … Ctrl+8.
- **Fixes** — settings changed quickly one after another are no longer undone; on long pages the
  window no longer scrolls as a whole.
- **Security** — the unmaintained `chrono` crate is replaced by `time`
  ([RUSTSEC-2026-0543](https://rustsec.org/advisories/RUSTSEC-2026-0543)), and all dependencies
  are up to date.

Full list: [CHANGELOG](https://github.com/poli0981/randomizerMAC/blob/master/CHANGELOG.md#210---2026-11-18).

## About DNS and your privacy

- The latency test, and the check before RandomMac switches to unencrypted DNS servers, send a few
  DNS queries to those servers — only when you run the test or apply servers.
- Once you switch, the provider you chose answers your DNS queries, and its privacy policy
  applies.
- Custom DNS servers can break captive portals (hotel or airport Wi-Fi) and company networks. Use
  **Automatic** or **Restore original** to go back.

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
| Windows 10/11 x64 (recommended) | `RandomMac_2.1.0_x64-setup.exe` |
| Windows 10/11 x64 (MSI) | `RandomMac_2.1.0_x64_en-US.msi` |
| Ubuntu 24.04+ / Debian 13+ | `RandomMac_2.1.0_amd64.deb` — `sudo apt install ./RandomMac_2.1.0_amd64.deb` |
| Fedora 41+ | `RandomMac-2.1.0-1.x86_64.rpm` — `sudo dnf install ./RandomMac-2.1.0-1.x86_64.rpm` |
| Other Linux x86_64 | `RandomMac_2.1.0_amd64.AppImage` |
| CLI only | `randommac-cli-2.1.0-x86_64-pc-windows-msvc.zip`, `randommac-cli-2.1.0-x86_64-unknown-linux-gnu.tar.gz` |

Verify: `sha256sum --ignore-missing -c SHA256SUMS.txt` and
`gh attestation verify <file> --repo poli0981/randomizerMAC`.

## Known issues

- The Windows installers are not code-signed yet; SmartScreen may ask you to confirm
  (*More info → Run anyway*).
- Some Wi-Fi drivers ignore MAC address changes, especially when Windows *Random hardware
  addresses* is on for the network. RandomMac detects this and restores the previous state.
- DNS over HTTPS needs Windows 11 or Windows Server 2022; on Windows 10 RandomMac sets unencrypted
  DNS servers. DNS over TLS is Linux only and needs systemd-resolved.
- DNS servers set for a network in Windows Settings take priority over the adapter's; RandomMac
  warns when that is the case.
- On GNOME without the AppIndicator extension there is no tray icon; closing the window quits.
- Only x86_64 builds are provided.

More help: [README](https://github.com/poli0981/randomizerMAC#readme) ·
[FAQ](https://github.com/poli0981/randomizerMAC#faq) ·
[Accessibility](https://github.com/poli0981/randomizerMAC/blob/master/ACCESSIBILITY.md) ·
[Report a bug](https://github.com/poli0981/randomizerMAC/issues/new?template=bug_report.yml)
