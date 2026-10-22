# CLAUDE.md

Guidance for AI assistants (and humans) working in this repository.

## Project

RandomMac 2.x — change/restore network adapter MAC addresses on **Windows 10/11 and Linux**.
Rust workspace + Tauri 2 desktop app + Svelte 5/TypeScript UI + `randommac-cli`.
1.x was a .NET/WinUI 3 app (still available at tag `v1.1.3`; do not resurrect it).
The maintainer writes in Vietnamese; user-facing docs are English first with Vietnamese mirrors
under `docs/i18n/vi/`.

Read first: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DEV_ENVIRONMENT.md](docs/DEV_ENVIRONMENT.md).

## Commands

```bash
npm ci                                   # once
npm run tauri dev                        # run the app (debug builds do not self-elevate)
npm run dev                              # UI only in a browser, backend faked by src/lib/devmock.ts
cargo run -p randommac-cli -- --mock list --all

# everything CI checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run lint && npm run check && npm test && npm run build
node scripts/version.mjs check
cargo deny check
```

- `cargo build/clippy -p randommac` needs `dist/` → run `npm run build` first.
- Type-check Linux code from Windows: `cargo clippy -p randommac-net -p randommac-cli --target x86_64-unknown-linux-gnu`.
- Demo backend: `RANDOMMAC_BACKEND=mock` (debug builds), `RANDOMMAC_MOCK_SCENARIO=driver_ignores|slow`.
- Logs filter: `RANDOMMAC_LOG=debug`.

## Layout

- `crates/randommac-core` — pure logic (no OS calls). Add unit tests for everything here.
- `crates/randommac-net` — `NetworkBackend` (windows.rs / linux.rs / mock.rs), parsers
  (`linux_parse.rs`, `win_classify.rs` — testable on any OS), `ops.rs` (validate → apply → verify →
  rollback → history). Every front end must go through `ops`.
- `crates/randommac-cli` — clap CLI; exit codes 0–6 are documented in README — keep them stable.
- `src-tauri/src` — `lib.rs` (startup: hardening → args → single instance → elevation → `run_app`),
  `commands.rs`, `ops_runner.rs`, `tray.rs`, `scheduler.rs`, `autostart.rs`, `updater.rs`,
  `legacy.rs` (1.x import/cleanup), `platform/{windows,linux}.rs`.
- `src/` — `lib/api.ts` (only place that calls `invoke`), `lib/stores/app.svelte.ts`,
  `lib/i18n/{en,vi}.ts`, `lib/pages/*`, `lib/components/*`.
- `scripts/` — `version.mjs`, `gen-oui.mjs` (regenerates `oui/vendors.rs` + `index.bin` from the IEEE
  CSV), `build-sidecar.mjs`, `release-assets.mjs` (verify assets, `latest.json`, `SHA256SUMS.txt`).

## Conventions

- Conventional Commits (`feat:`, `fix:`, `docs:`, `chore(deps):`, `ci(deps):`, `feat!:`). Branch from
  `master`; PRs are squash-merged.
- New UI string → add to `en.ts` **and** `vi.ts` (tests enforce identical keys and placeholders).
  Rust-side strings (tray, notifications, dialogs) are in `src-tauri/src/i18n.rs`.
- New Tauri command → `commands.rs` + `generate_handler!` in `lib.rs` + the list in `build.rs` +
  `src-tauri/permissions/app-commands.toml` + a wrapper in `src/lib/api.ts` (+ `devmock.ts`).
- Errors crossing to the UI are `ApiError { code, message }`; add new codes to `en.ts`/`vi.ts`
  (`error.<CODE>`) and to the list in `i18n.test.ts`.
- External programs: absolute paths, fixed `PATH`, no shell, validated arguments, always through
  `randommac_net::process::run` (it also decodes UTF-8/OEM-code-page output). On Windows locate
  system programs with `platform::windows::system32()` — never from `%SystemRoot%`/`%windir%`.
- Changes from the app go through `ops_runner` (own task + op lock + events); never abort a task
  that is in the middle of a change. UI settings edits go through `app.saveSettings` (queued).
- A new external link in the UI must also be added to `src-tauri/src/links.rs` (exact host/path).
- Never bump versions by hand: `npm run version:set X.Y.Z` (Cargo.toml, Cargo.lock, package.json,
  package-lock.json; `tauri.conf.json` deliberately has no `version`).

## Release

1. `npm run version:set X.Y.Z`; add `## [X.Y.Z] - date` to `CHANGELOG.md`; write
   `docs/RELEASE_vX.Y.Z.md` (**absolute links only** — it becomes the release body and the in-app
   update notes).
2. PR → CI green → squash-merge.
3. `git tag -a vX.Y.Z -m "RandomMac X.Y.Z" && git push origin vX.Y.Z` on the merge commit.
4. `release.yml` builds/signs (Windows: nsis+msi; Ubuntu 24.04: deb+rpm+appimage + CLI archives),
   verifies the assets and every `.sig` against the pubkey in `tauri.conf.json`, generates
   `latest.json` + `SHA256SUMS.txt`, attests, publishes, dispatches `announce-release.yml` and
   `discussion-on-release.yml`. A failed run is re-run with `workflow_dispatch` (input `tag`,
   optional `dry_run`): it builds the tagged sources with the workflow of the dispatch branch and
   refuses to touch an already published release. Never move a tag — ship a patch version.

## Gotchas (learned the hard way)

- Workflow `name: CI` and `name: Release` are referenced **by name** from
  `notify-ci-failure.yml` / `notify-release-pipeline.yml` (`workflow_run`). Renaming breaks Discord
  notifications.
- A release published with `GITHUB_TOKEN` does not trigger `release: published` workflows → the
  release job dispatches them with `gh workflow run`.
- Runners are pinned to `ubuntu-24.04` (`ubuntu-latest` becomes 26.04 on 2026-10-19). Linux builds
  therefore need glibc ≥ 2.39 (Ubuntu 24.04+, Debian 13+, Fedora 41+).
- TypeScript is installed twice, as npm aliases. `@typescript/native` = `typescript@7`: its `tsc`
  does the type checking (`npm run check` is `svelte-check --tsgo`, which writes svelte2tsx output
  to the gitignored `.svelte-check/`). `typescript` = `@typescript/typescript6` (bin `tsc6`): the
  TS 6 API that svelte-check, svelte2tsx and typescript-eslint still load, because TS 7.0 ships no
  JS API. Never install a plain `typescript@7` — `npm ci` (peer ranges), lint and check break.
  Dependabot skips aliases, so update both by hand with the same syntax
  (`npm i -D "@typescript/native@npm:typescript@~7.1.0"`); drop the TS 6 alias once svelte-check
  and typescript-eslint support TS 7's API (expected with 7.1).
- `.nvmrc` (26) is the only Node pin: every `actions/setup-node` step reads it. `engines.node` and
  the `@types/node` major follow it (Dependabot ignores `@types/node` majors).
- Tauri derives Linux package names from `productName`; the deb/rpm *package* is `random-mac`,
  binaries are `randommac` and `randommac-cli`.
- `src-tauri/permissions/autogenerated/` and `src-tauri/gen/` are generated — don't commit them.
- The GUI ignores unknown flags and starts normally (on Windows release builds that means a UAC
  prompt). Only `--version`/`--help` exit immediately.
- Windows Wi-Fi accepts only locally administered addresses (`x2/x6/xA/xE`); Windows *Random
  hardware addresses* or the driver may ignore `NetworkAddress` — the backend detects it and rolls
  back (`DRIVER_REJECTED`). The maintainer's Realtek 8852CE Wi-Fi behaves this way.
- WebKitGTK + proprietary NVIDIA → blank window; `platform/linux.rs` sets
  `WEBKIT_DISABLE_DMABUF_RENDERER=1`.
- The GUI refuses to run as root on Linux (`RANDOMMAC_ALLOW_ROOT=1` overrides, for debugging only).
- The updater signing key lives outside the repository (`*.key` is gitignored). Losing it means
  existing installs can never auto-update again. tauri-cli only *warns* when the key does not
  match the pubkey in `tauri.conf.json`; `release-assets.mjs verify` is what fails the release.
- NetworkManager originals are stored per connection UUID (`state.json` → `nmProfiles`); NM's own
  `random`/`stable`/`preserve`/`permanent` values are policies, not RandomMac overrides — Restore
  must not erase them. Windows "random hardware addresses" likewise yields `CHANGED_BY_OS`.
- For pull requests, CI `paths-ignore` is evaluated on the whole PR diff: a docs-only push to a PR
  that also changes code re-runs the full CI. Docs-only follow-ups are faster as their own PR.

## Safety when testing on the maintainer's machine

- Never change the address of the Ethernet adapter that provides connectivity.
- Real changes only with explicit permission, on the Wi-Fi adapter, and always restore afterwards
  (`randommac-cli restore`, then confirm `NetworkAddress` is gone from the driver key).
- Prefer `--mock`, `RANDOMMAC_BACKEND=mock` or `--data-dir <tmp>` for experiments.
