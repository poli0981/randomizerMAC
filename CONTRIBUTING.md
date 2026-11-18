# Contributing to RandomMac

Thank you for your interest in contributing. Please read this guide before opening an issue or a
pull request.

## Maintenance mode

RandomMac 2.1.0 (November 2026) is the last feature release, and the project is now in
**maintenance mode**: it stays maintained, but it won't grow new features. What still lands:

1. **Dependency updates** on a regular schedule (Dependabot, weekly).
2. **Security fixes and necessary features**: fixes for dependencies with a high or critical CVE
   and for serious security issues in runtimes such as Node.js; a new feature only when it is
   judged genuinely necessary.
3. **Major upgrades** of a language, runtime or framework, for example Node 24 → 26 or
   TypeScript 6 → 7.
4. **Issues and pull requests** that fix a bug, a broken UI/UX or an accessibility barrier.
5. **Language fixes**: wording that doesn't fit a culture or a regional dialect, and spelling
   mistakes that could mislead (see [Localization](#localization)).

Everything else is out of scope.

- The [maintenance triage workflow](.github/workflows/maintenance-triage.yml) closes feature
  issues and `feat:` pull requests from outside contributors automatically, with the
  `out-of-scope` label. The maintainer, members, collaborators and bots are not affected. It also
  closes anything the maintainer labels `out-of-scope`.
- If your issue or pull request was closed by mistake (it is really a bug, accessibility or
  language fix), reply on it and the maintainer will take another look.
- Ideas are welcome in
  [Discussions → Ideas](https://github.com/poli0981/randomizerMAC/discussions/categories/ideas),
  with no commitment.

Where to report:

- **Bugs**: the [Bug Report](.github/ISSUE_TEMPLATE/bug_report.yml) form.
- **Accessibility barriers**: the [Accessibility issue](.github/ISSUE_TEMPLATE/accessibility.yml)
  form, or [accessibility@poli0981.dev](mailto:accessibility@poli0981.dev).
- **Vulnerabilities**: privately through a
  [security advisory](https://github.com/poli0981/randomizerMAC/security/advisories/new), never in
  a public issue; see [SECURITY.md](SECURITY.md).
- **Translations**: the [Language / Translation Fix](.github/ISSUE_TEMPLATE/fix_lang.yml) form.

## Getting started

1. Fork the repository and clone your fork:
   `git clone https://github.com/YOUR_USERNAME/randomizerMAC.git`
2. Create a branch from `master`: `git checkout -b fix/short-description`
3. Make your change and run the checks below.
4. Push and open a pull request.

## Development setup

See [docs/DEV_ENVIRONMENT.md](docs/DEV_ENVIRONMENT.md) for prerequisites on Windows, Ubuntu and
Fedora, and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the pieces fit together.

```bash
npm ci
npm run tauri dev                                   # desktop app
RANDOMMAC_BACKEND=mock npm run tauri dev            # with demo adapters (nothing is changed)
cargo run -p randommac-cli -- --mock list --all     # CLI against demo adapters
```

## Before you open a pull request

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run lint
npm run check
npm test
```

CI runs the same checks on Windows and Ubuntu, plus package installation tests.

## Coding standards

- **Rust** (edition 2024): `rustfmt` formatting, no clippy warnings, no `unwrap()`/`expect()` on
  data that comes from the user or the operating system, errors with a stable `code`. Keep
  OS-independent logic in `randommac-core` with unit tests; parsing of OS tool output belongs in
  `*_parse.rs` so it can be tested on every platform.
- **UI** (Svelte 5 runes + TypeScript strict): call the backend only through `src/lib/api.ts`;
  keep components accessible (labels, keyboard support, visible focus) and do a keyboard-only pass
  of what you changed — see [ACCESSIBILITY.md](ACCESSIBILITY.md#contributor-expectations).
- **Security**: never build shell command lines; start programs by absolute path with validated
  arguments (see `crates/randommac-net/src/process.rs`).
- **Commits**: [Conventional Commits](https://www.conventionalcommits.org/) — `feat:`, `fix:`,
  `docs:`, `refactor:`, `test:`, `chore:`; add `!` for breaking changes.

## Localization

User-facing strings live in `src/lib/i18n/en.ts` (English, the source) and `src/lib/i18n/vi.ts`
(Vietnamese); the few strings produced by Rust (tray, notifications, file dialogs) are in
`src-tauri/src/i18n.rs`. Add every new key to all languages — the tests fail otherwise.

Translations other than English are AI-generated. To report a mistake, use the
**Language / Translation Fix** issue template with the key, the current text, your suggestion and
why. Wording that doesn't fit a culture or a regional dialect, and spelling mistakes that could
mislead, are still fixed in [maintenance mode](#maintenance-mode).

A new language counts as a new feature during [maintenance mode](#maintenance-mode): agree on it
with the maintainer before you start. To add one: create `src/lib/i18n/<code>.ts` typed as
`Dictionary`, register it in `src/lib/i18n/index.svelte.ts`, add the variant to `Language` in
`crates/randommac-core/src/settings.rs` and the strings in `src-tauri/src/i18n.rs`, then open a PR.

## Pull request guidelines

- Use the [PR template](.github/PULL_REQUEST_TEMPLATE.md); keep one topic per PR.
- Title under 70 characters, in Conventional Commits form.
- Include evidence: screenshots for UI changes, test output or new tests for logic changes.
- Do not include unrelated changes, IDE settings or generated files (`dist/`, `target/`,
  `src-tauri/gen/`, `src-tauri/permissions/autogenerated/`).

## Issue guidelines

Use the templates: [Bug Report](.github/ISSUE_TEMPLATE/bug_report.yml),
[Accessibility issue](.github/ISSUE_TEMPLATE/accessibility.yml),
[Language / Translation Fix](.github/ISSUE_TEMPLATE/fix_lang.yml). There is no feature request
template while RandomMac is in [maintenance mode](#maintenance-mode). For bugs, include your OS and
version, the RandomMac version, the adapter, and the diagnostics from **About → Copy diagnostics**.
Search existing issues first. Security problems: see [SECURITY.md](SECURITY.md) — do not open a
public issue.

## Auto-ignore policy

The following submissions are closed without response:

- Offensive, hateful or abusive language toward maintainers or other contributors
- Pull requests containing malicious code or suspected backdoors
- Issues or pull requests unrelated to this project
- Excessively long, vague or unfocused reports
- Duplicates (search before opening)

Repeated violations may result in a permanent ban. See the [Code of Conduct](CODE_OF_CONDUCT.md).

## AI disclosure

This project is developed with assistance from **Anthropic Claude** via Claude Code (Claude Opus
4.6 for 1.0, Opus 4.7 for 1.1.x, Opus 5.5 for the 2.0 rewrite and for 2.1). The 2.1 work (DNS
settings, accessibility, maintenance mode) was AI-assisted the same way as the earlier versions.
AI helps with code, debugging, tests, documentation and translations. Every change — human or AI —
is reviewed by the maintainer.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
