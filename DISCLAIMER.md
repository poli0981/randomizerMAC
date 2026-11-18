# Disclaimer

**Last updated:** 2026-11-18

## General

RandomMac is provided "as is", without warranty of any kind, express or implied. By using this
software, you acknowledge and agree to the following terms.

## Legal risks

Changing (spoofing) a MAC address may be **illegal or violate terms of service** in certain
jurisdictions, networks or contexts, including but not limited to:

- Corporate or institutional network policies
- Internet Service Provider (ISP) terms of service
- Online gaming platforms with anti-cheat systems (e.g. EAC, BattlEye, Vanguard)
- Public Wi-Fi terms of use
- Government or regulatory requirements

**You are solely responsible** for ensuring that your use of this software complies with all
applicable laws, regulations and policies in your jurisdiction.

## Hardware and driver compatibility

RandomMac does **not** work with every network adapter.

- **Windows**: the change is made through the driver's `NetworkAddress` setting. Some drivers —
  and Windows itself when *Random hardware addresses* is enabled for a Wi-Fi network — ignore it.
  Wi-Fi drivers accept only locally administered addresses.
- **Linux**: changes are made through NetworkManager or iproute2. Some drivers refuse address
  changes, and other network tools may overwrite them.

RandomMac verifies every change and rolls it back if it was not applied, but **no guarantee** is
made that a specific adapter will work. You can check the active address yourself with
`getmac /v` or `ipconfig /all` (Windows) or `ip link` (Linux).

A change briefly interrupts the connection while the adapter restarts or the connection is
re-activated.

## DNS settings

- The DNS providers listed in RandomMac are independent third parties. Their inclusion is not an
  endorsement, and **no guarantee** is made about their availability, filtering or handling of
  your queries. Their addresses were checked against each provider's documentation on the date
  shown in the app and may change.
- Custom DNS servers can stop captive portals (the sign-in pages of hotel, airport or other public
  Wi-Fi) and company or school networks that rely on their own DNS servers from working. Choose
  *Automatic (from the network)* or *Restore original* to go back.

## Elevated privileges

Changing a MAC address or DNS settings requires elevated privileges: administrator rights on
Windows (the app runs elevated after a UAC prompt, or read-only if you decline) and
polkit/`pkexec` authorization on Linux. Running software with elevated privileges carries inherent
risks. Only run software you trust and download it from the
[official releases](https://github.com/poli0981/randomizerMAC/releases).

## AI-assisted development

- This application was built and tested with AI assistance from **Anthropic Claude** via Claude
  Code (Claude Opus 4.6 for 1.0, Opus 4.7 for 1.1.x, Opus 5.5 for the 2.0 rewrite).
- AI was used for code generation, debugging, test creation, documentation and translations.
- All code is reviewed, tested and approved by the developer before release.
- Translations to languages other than English are **AI-generated** and are **not guaranteed** to
  be accurate, natural or contextually appropriate. Please report mistakes with the language fix
  issue template.

## Data privacy

- All data (settings and DNS presets, blacklist, history, original addresses and DNS settings,
  logs) is stored **locally** on your device.
- No data about you is collected by the developer or shared with any third party.
- RandomMac itself uses the network only for:
  - **the update check**, which downloads a small manifest from GitHub Releases at most once a
    day. It can be turned off in Settings; manual checks happen only when you click *Check for
    updates*;
  - **the DNS latency test**, which sends a few DNS queries (for the root zone; they contain no
    names you visit and no personal data) to each DNS server being tested, only when you run it;
  - **the reachability check**: before plain (unencrypted) DNS servers are applied, the same kind
    of query is sent to them to check that they answer.

  As with any connection, the servers contacted can see your IP address.
- **Third-party DNS providers**: once you switch an adapter to other DNS servers, your device sends
  its DNS queries — which reveal the names of the websites and services it looks up — to that
  provider instead of your network's servers. The provider's own privacy policy and terms apply;
  the developer has no control over third-party DNS services and accepts no responsibility for
  them. Encrypted DNS (DNS over HTTPS or TLS) protects the queries in transit, not from the
  provider itself.

## No warranty

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. IN NO EVENT SHALL THE AUTHORS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THIS SOFTWARE. See the
[LICENSE](LICENSE) file for the full terms.

## Acknowledgment

By downloading, installing or using RandomMac, you acknowledge that you have read and understood
this disclaimer and agree to its terms.
