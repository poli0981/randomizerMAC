# Disclaimer

**Last updated:** 2026

## General

RANDOM MAC is provided "as-is" without warranty of any kind, express or implied. By using this software, you acknowledge and agree to the following terms.

## Legal Risks

Changing (spoofing) a MAC address may be **illegal or violate terms of service** in certain jurisdictions, networks, or contexts, including but not limited to:

- Corporate or institutional network policies
- Internet Service Provider (ISP) terms of service
- Online gaming platforms with anti-cheat systems (e.g., EAC, BattlEye, Vanguard)
- Public Wi-Fi terms of use
- Government or regulatory requirements

**You are solely responsible** for ensuring that your use of this software complies with all applicable laws, regulations, and policies in your jurisdiction.

## Vendor Compatibility

This application does **NOT** support MAC address changes for all network adapters. Some hardware vendors and drivers silently ignore the `NetworkAddress` registry value. The application will attempt to verify the change and notify you if it was not applied, but **no guarantee** is made that any specific adapter will work.

Always verify the MAC change was applied using system tools such as `ipconfig /all` or `getmac`.

## AI-Assisted Development

- This application was built and tested with AI assistance from **Anthropic Claude** (Claude Opus 4.6) via Claude Code.
- AI was used for: code generation, debugging, test creation, documentation, and language translations.
- All code was reviewed, tested, and approved by the developer before inclusion.
- All translations to languages other than English are **AI-generated** and are **NOT guaranteed** to be accurate, natural, or contextually appropriate. If you find a translation error, please submit an issue using the language fix template.

## Administrator Privileges

This application requires **administrator (elevated) privileges** to function. It modifies Windows registry values and restarts network adapters via WMI. Running software with elevated privileges carries inherent risks. Only run software you trust.

## Data Privacy

- All data is stored **locally** on your machine.
- No data is collected, transmitted, or shared with any third party.
- The only network activity is checking for updates via the GitHub Releases API (optional, user-initiated).

## No Warranty

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY ARISING FROM THE USE OF THIS SOFTWARE. See the [LICENSE](LICENSE) file for full terms.

## Acknowledgment

By downloading, installing, or using RANDOM MAC, you acknowledge that you have read and understood this disclaimer and agree to its terms.
