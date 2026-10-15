# End User License Agreement (EULA)

**Effective date:** 2026-10-15
**Application:** RandomMac (formerly "RANDOM MAC")
**Developer:** poli0981

By downloading, installing or using RandomMac ("the Software"), you agree to the following terms.

## 1. License grant

The Software is licensed under the [MIT License](LICENSE). You are granted a non-exclusive,
worldwide, royalty-free license to use, copy, modify and distribute the Software, subject to the
conditions of the MIT License.

## 2. Permitted use

You may use the Software for:

- Personal network administration and troubleshooting
- Privacy protection on networks you do not control
- Security testing on networks you own or are authorized to test
- Educational and research purposes

## 3. Prohibited use

You agree **not** to use the Software for:

- Circumventing network security on systems you do not own or are not authorized to access
- Evading bans, suspensions or restrictions imposed by online services
- Any activity that violates applicable local, state, national or international laws
- Distributing modified versions containing malware or malicious code

## 4. No warranty

The Software is provided "as is" without warranty of any kind. The developer makes no guarantees
regarding functionality, reliability or fitness for any particular purpose. See the
[LICENSE](LICENSE) for the full warranty disclaimer.

## 5. Limitation of liability

In no event shall the developer be liable for any direct, indirect, incidental, special or
consequential damages arising from the use or inability to use the Software, including but not
limited to:

- Loss of network connectivity
- Violation of network policies or terms of service
- Hardware or software damage
- Data loss

## 6. Privacy

- The Software stores all data **locally** on your device.
- No personal data, usage statistics or telemetry is collected or transmitted.
- The only internet feature is the update check against GitHub Releases. It sends no identifying
  information beyond what any HTTPS request to GitHub contains, and it can be turned off.

## 7. Updates

Updates are published on GitHub Releases. The Software verifies the digital signature of every
update before installing it. Automatic checks are optional; installing an update always requires
your action (on Linux, `.deb`/`.rpm` updates are installed with your system's package manager after
you authorize it). The developer may change the Software in future releases.

## 8. Termination

This agreement is effective until terminated. Your rights under this agreement terminate
automatically if you fail to comply with any of its terms. Upon termination, you must stop using the
Software and destroy all copies.

## 9. Third-party components

The Software includes or relies on open-source components, each subject to its own license,
including:

- Tauri and its plugins (MIT / Apache-2.0)
- The Rust standard library and crates such as serde, tracing, clap, chrono, windows-sys, zbus and
  zip (MIT and/or Apache-2.0)
- Svelte (MIT), Lucide icons (ISC), marked (MIT), DOMPurify (Apache-2.0 / MPL-2.0)
- Vendor prefix data from the IEEE Registration Authority public listings
- The system web engine: Microsoft Edge WebView2 (Windows) or WebKitGTK (Linux), which are
  provided by the operating system or installed separately under their own terms

The complete list of Rust and npm dependencies and their licenses is available in `Cargo.lock` and
`package-lock.json` of the source repository.

## 10. AI-assisted development

The Software was developed with AI assistance from Anthropic Claude via Claude Code (Claude Opus 4.6,
Opus 4.7 and Opus 5.5). AI was used for code generation, debugging, testing, documentation and
translations. All AI-generated code was reviewed and approved by the developer. AI-generated
translations are not guaranteed to be accurate.

## 11. Governing law

This agreement shall be governed by and construed in accordance with applicable laws. Any disputes
arising from this agreement shall be resolved through good-faith communication.

## 12. Contact

For questions regarding this EULA, please open an issue on the
[GitHub repository](https://github.com/poli0981/randomizerMAC).
