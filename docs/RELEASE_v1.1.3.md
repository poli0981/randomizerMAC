# RANDOM MAC v1.1.3

A hotfix release focused on developer ergonomics: GitHub Actions hygiene, repo metadata, and a new in-app **Social Dev** page surfacing donate / report-bug / social links. No changes to MAC randomization, registry, WMI, or any `RandomMac.Core` logic.

Existing `settings.json` / `blacklist.json` / `history.json` files load unchanged.

Built with C# 13 / .NET 11 (preview) for the App project, .NET 9 for Core.

---

## Added

- **Social Dev nav page (Ctrl+6).** New 6th navigation entry hosting:
  - **Donate** button — a `MenuFlyout` exposing Ko-fi, Buy Me a Coffee, Patreon, and PayPal.
  - **Report a Bug** button — opens GitHub Issues with the `bug_report.yml` template pre-selected.
  - **Networks** grid — 11 social profiles (X / YouTube / Discord ×2 / Patreon / Ko-fi / Steam / Bluesky / Mastodon / Telegram ×2) rendered with `ItemsRepeater` + `UniformGridLayout`.
  - **Contact** section — direct `mailto:` link.
  - All localized via `Lang.resx` / `Lang.vi.resx` (7 new keys); URL launching dispatched from code-behind to sidestep WinUI 3 flyout `DataContext` fragility.
- **`.github/FUNDING.yml`.** Activates the "Sponsor" button on the repo with five channels: GitHub Sponsors (poli0981), Ko-fi, Buy Me a Coffee, Patreon, PayPal (custom URL).
- **Auto-create GitHub Discussion on each release.** New `.github/workflows/discussion-on-release.yml` runs on `release: published` (parallel to the existing Discord announcer) and opens a thread in the **Announcements** category via the GraphQL `createDiscussion` mutation. Manual `workflow_dispatch` with a `tag` input supports back-filling discussions for prior releases.
- **Developer-facing docs.** `docs/pc_spec.md` (hardware reference for the dev workstation) and `docs/dev_env.md` (personal toolchain — .NET, Python, Node, Rust, Git/GPG). Vietnamese mirrors at `docs/i18n/vi/`. `docs/DEV_ENVIRONMENT.md` (project build instructions) is intentionally unchanged.

## Changed

- **CI no longer runs on documentation-only commits.** `ci.yml` now declares `paths-ignore` for `docs/**`, `**/*.md`, `.github/FUNDING.yml`, `.github/ISSUE_TEMPLATE/**`, and the root metadata files (`CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `DISCLAIMER.md`, `EULA.md`, `SECURITY.md`, `LICENSE`). Workflow file edits (`.github/workflows/**`) still trigger CI.
- **Reusable workflow callers tightened.**
  - `notify-ci-failure.yml` used `workflows: ["*"]` which fired on every workflow_run completion. Now restricted to `["CI", "Release", "Announce Release to Discord"]` with an explicit `if: github.event.workflow_run.conclusion == 'failure'` gate at the job level.
  - `notify-release-pipeline.yml` watched for `"Release Desktop App"` while the actual workflow `name:` is just `"Release"`, so the notifier never fired. Names now match.
- **Dependabot config.** Schedule day shifted from `monday` → `tuesday` to avoid the Monday-morning queue. Added `assignees`/`reviewers: [poli0981]` and `commit-message` prefixes (`chore(deps)` for NuGet, `chore(ci)` for github-actions).
- **Keyboard shortcut hint in README** updated from `Ctrl+1..5` to `Ctrl+1..6`.

## What did NOT change

- Every algorithm in `RandomMac.Core`: MAC generation, registry write, WMI restart, blacklist, history, settings, OUI lookup, update service.
- The 5 existing nav pages (Dashboard, Settings, Log, Update, About) — their Ctrl+1..5 shortcuts still resolve to the same destinations.
- Velopack 0.0.1298 packaging (unpackaged, self-contained).
- `requireAdministrator` manifest (no UAC at boot via Task Scheduler).
- All 37 RandomMac.Tests unit tests still pass.

## Out of scope

- **`webapp/TAURI.md`** referenced in the developer's personal spec is intentionally deferred — the repo has no `webapp/` directory and no Tauri 2 tooling. A separate companion is being evaluated as future work, not part of v1.1.x.

## Smoke checklist (manual)

1. Build + launch. Six nav items appear (Dashboard, Settings, Log, Update, About, Social).
2. `Ctrl+1` … `Ctrl+6` each jump to the corresponding page. `Ctrl+6` opens **Social Dev**.
3. Settings → switch language to Tiếng Việt. All Social Dev labels translate (header, "Ủng hộ", "Báo lỗi", section titles); nav label flips to "Mạng xã hội". Switch back to English.
4. Click **Donate** → flyout shows 4 channels → click each → browser opens the correct destination.
5. Click **Report a Bug** → GitHub Issues page loads with the **Bug Report** template pre-selected.
6. Click each social card (X / YouTube / Discord / Patreon / Ko-fi / Steam / Bluesky / Mastodon / Telegram entries) → browser opens the correct URL.
7. Click the contact email link → default mail handler launches with `lopop05905@proton.me`.
8. Tray icon, `--minimized` startup, MinimizeToTray behavior: unchanged from v1.1.2.

## Workflow / repo verification

1. Push a docs-only commit → `Actions` tab shows no CI run for that push.
2. Push a code commit → CI runs as before.
3. Cut a draft pre-release tag (e.g. `v0.0.0-test1`) → confirm `Release`, `Announce Release to Discord`, **and** `Create Discussion on Release` all run; the Discussion appears under **Announcements** with release notes embedded in a `<details>` block.
4. `notify-release-pipeline` (previously broken) now triggers after `Release` completes.
5. Repository homepage shows the "Sponsor" button; clicking it lists GitHub Sponsors, Ko-fi, Buy Me a Coffee, Patreon, and the PayPal custom URL.

## Prerequisites (one-time, repo Settings)

- **Discussions** must be enabled (Settings → Features → Discussions). The first run of `discussion-on-release.yml` will fail with a clear error message if it isn't.
- An **Announcements** category must exist (the default category — already present on most repos).

## Compatibility

- Windows 11 (Mica) / Windows 10 22H2 (Acrylic fallback) / older (solid backdrop).
- x64 only. ARM64 not supported in this release.
- Bundled WinAppSDK runtime + .NET 11 (~225 MB self-contained).
- Building from source needs the .NET 11 preview SDK (`global.json` pinned); see [DEV_ENVIRONMENT.md](DEV_ENVIRONMENT.md).
