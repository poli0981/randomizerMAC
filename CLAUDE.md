# CLAUDE.md

Guidance for AI assistants (and humans) working in this repository.

## Project

RandomMac 2.x — change/restore network adapter MAC addresses and per-adapter DNS servers on
**Windows 10/11 and Linux**. Rust workspace + Tauri 2 desktop app + Svelte 5/TypeScript UI +
`randommac-cli`. 1.x was a .NET/WinUI 3 app (still available at tag `v1.1.3`; do not resurrect it).
The maintainer writes in Vietnamese; user-facing docs are English first with Vietnamese mirrors
under `docs/i18n/vi/`.

**Maintenance mode since 2.1.0** (the last feature release): dependency, security, bug, UI/UX,
accessibility and language fixes still land; new features only when the maintainer decides one is
necessary. Outside feature requests and `feat:` PRs are closed with `out-of-scope`
([CONTRIBUTING.md](CONTRIBUTING.md#maintenance-mode)).

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
npm run lint && npm run check && npm test && npm run build   # npm test includes the axe-core tests
node scripts/version.mjs check
cargo deny check
npm run check:licenses                   # npm license allow-list (scripts/lib/licenses.mjs)

# coverage (CI sends it to SonarQube Cloud)
cargo llvm-cov --workspace --html        # needs `rustup component add llvm-tools` + cargo-llvm-cov
npm run test:coverage
```

- `cargo build/clippy -p randommac` needs `dist/` → run `npm run build` first.
- Type-check Linux code from Windows: `cargo clippy -p randommac-net -p randommac-cli --target x86_64-unknown-linux-gnu`.
- Demo backend: `RANDOMMAC_BACKEND=mock` (debug builds), `RANDOMMAC_MOCK_SCENARIO=driver_ignores|slow`.
- Logs filter: `RANDOMMAC_LOG=debug`.

## Layout

- `crates/randommac-core` — pure logic (no OS calls). Add unit tests for everything here.
  `dns/` = DNS model + validation, provider catalog (`catalog.json`; each entry cites its `sources`
  and `verifiedOn` — update them with the data), presets, `dns-state.json` snapshots, latency-test
  wire format. `timefmt.rs` = every timestamp.
- `crates/randommac-net` — `NetworkBackend` (windows.rs / linux.rs / mock.rs; DNS in
  `windows_dns.rs` / `linux_dns.rs`), plain-data halves testable on any OS (`linux_parse.rs`,
  `win_classify.rs`, `win_dns.rs`, `nm_dns.rs`), `ops.rs` (validate → apply → verify → rollback →
  history), `dns_ops.rs` (validate → snapshot → apply → read back → rollback → history),
  `dns_probe.rs` (latency test). Every front end must go through `ops` / `dns_ops`.
- `crates/randommac-cli` — clap CLI (`dns.rs` = the `dns` subcommands); exit codes 0–6 are
  documented in README — keep them stable.
- `src-tauri/src` — `lib.rs` (startup: hardening → args → single instance → elevation → `run_app`),
  `commands.rs`, `ops_runner.rs`, `tray.rs` + `tray_menu.rs` (DNS submenu as tested data),
  `scheduler.rs`, `autostart.rs`, `updater.rs`, `legacy.rs` (1.x import/cleanup),
  `platform/{windows,linux}.rs`.
- `src/` — `lib/api.ts` (only place that calls `invoke`), `lib/stores/app.svelte.ts`,
  `lib/i18n/{en,vi}.ts`, `lib/pages/*` (`Dns.svelte`, …), `lib/components/*`; page logic in tested
  modules: `lib/dns.ts` (mirrors the core DNS validation), `lib/history.ts`, `lib/nav.ts` (page
  order = `Ctrl+1` … `Ctrl+8`); `lib/a11y.test.ts` + `test/axe.ts` (accessibility tests).
- `scripts/` — `version.mjs`, `gen-oui.mjs` (regenerates `oui/vendors.rs` + `index.bin` from the IEEE
  CSV), `build-sidecar.mjs`, `release-assets.mjs` (verify assets, `latest.json`, `SHA256SUMS.txt`),
  `lcov-paths.mjs` (repository-relative LCOV paths so the Windows and Linux reports merge),
  `check-licenses.mjs` (npm license allow-list in `scripts/lib/licenses.mjs`).
- `sonar-project.properties` — SonarQube Cloud scope, coverage reports and the coverage exclusions
  (each with its reason). The analysis is the `sonar` job of `ci.yml`.
- `qodana.yaml` (JS/TS) and `qodana-rust.yaml` (Rust) — Qodana configurations, run by `qodana.yml`.

## Conventions

- Conventional Commits (`feat:`, `fix:`, `docs:`, `chore(deps):`, `ci(deps):`, `feat!:`). Branch from
  `master`; PRs are squash-merged.
- Quality gate (SonarQube Cloud, *Sonar way*): new code needs ≥ 80 % coverage, no new issues,
  ≤ 3 % duplication and reviewed hotspots — the CI `sonar` job fails otherwise. Put testable logic
  where it is measured (`randommac-core`, `randommac-net`, `src/lib/*.ts`), not in the Tauri glue
  that `sonar-project.properties` excludes from coverage.
- New UI string → add to `en.ts` **and** `vi.ts` (tests enforce identical keys and placeholders).
  Rust-side strings (tray, notifications, dialogs) are in `src-tauri/src/i18n.rs`.
- New Tauri command → `commands.rs` + `generate_handler!` in `lib.rs` + the list in `build.rs` +
  `src-tauri/permissions/app-commands.toml` + a wrapper in `src/lib/api.ts` (+ `devmock.ts`).
- Errors crossing to the UI are `ApiError { code, message }`; add new codes to `en.ts`/`vi.ts`
  (`error.<CODE>`) and to the list in `i18n.test.ts`.
- External programs: absolute paths, fixed `PATH`, no shell, validated arguments, always through
  `randommac_net::process::run` (it also decodes UTF-8/OEM-code-page output). On Windows locate
  system programs with `randommac_net::windows::system32()` (re-exported as
  `platform::windows::system32`) — never from `%SystemRoot%`/`%windir%`.
- MAC and DNS changes from the app go through `ops_runner` (own task + op lock + events carrying
  `kind: mac|dns`); never abort a task that is in the middle of a change. UI settings edits go
  through `app.saveSettings` (queued).
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

- Workflow `name: CI`, `name: Qodana` and `name: Release` are referenced **by name** from
  `notify-ci-failure.yml` (CI, Qodana) / `notify-release-pipeline.yml` (`workflow_run`). Renaming
  breaks Discord notifications.
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
- NetworkManager originals are stored per connection UUID (`state.json` and `dns-state.json` →
  `nmProfiles`); NM's own `random`/`stable`/`preserve`/`permanent` values are policies, not
  RandomMac overrides — Restore must not erase them. Windows "random hardware addresses" likewise
  yields `CHANGED_BY_OS`.
- For pull requests, CI `paths-ignore` is evaluated on the whole PR diff: a docs-only push to a PR
  that also changes code re-runs the full CI. Docs-only follow-ups are faster as their own PR.
- SonarQube Cloud counts a source file missing from every LCOV report as 0 % covered. Rust coverage
  is merged from four reports: the Rust job on Windows and on Linux (each covers its own backend),
  the instrumented CLI in the *Linux MAC change* job, which also drives NetworkManager (MAC and
  DNS) through `crates/randommac-net/tests/fake-nmcli.sh` and `fake-busctl.sh` (installed as
  `/usr/bin/nmcli` and `/usr/bin/busctl`), and the instrumented CLI in the *Windows DNS change* job
  (real DNS changes on the runner's adapter). `.svelte` files are not analyzed at all; keep UI
  logic in `.ts` modules.
- The `sonar` job is skipped for Dependabot and fork PRs (no `SONAR_TOKEN`); PR decoration comes
  from the SonarQube Cloud GitHub App ("SonarCloud Code Analysis" check). CI installs npm packages
  with `--ignore-scripts` (Sonar `githubactions:S6505`); no dependency needs install scripts.
- CodeQL default setup: the REST API rejects `rust` in `languages`, but switching the setup off and
  on again without `languages` (`state=not-configured`, then `state=configured`) auto-detects
  actions, JavaScript/TypeScript and Rust.
- Never name a PowerShell function `cli` in workflow steps: `cli` is the built-in alias of
  `Clear-Item`, and aliases win over functions (the Windows DNS job uses `Invoke-RandomMac`).
- Windows DNS: `Get`/`Set`/`FreeInterfaceDnsSettings` are looked up at run time (`windows_dns.rs`);
  a static import would stop the binaries from starting before Windows 10 2004. DoH flags can read
  back as `0x11` (`ENABLE_AUTO | ENABLE_DDR`): snapshots restore them verbatim — never mask them.
  Windows lists an adapter's IPv6 DNS servers before IPv4, so compare as sets (`win_dns::in_use`).
  VPN leak protection may unbind IPv6 from adapters: IPv6 settings are written when Windows accepts
  them, otherwise skipped (`DNS_IPV6_SKIPPED`, reason `FAMILY_DISABLED`).
- The reachability check before a change (CLI `check_servers`, UI `apply` in `Dns.svelte`) runs only
  when the *resolved* request is plain DNS — encrypted DNS uses ports that may be open where 53 is
  not. Resolve first: `--encryption auto` for servers without DoH/DoT endpoints is plain too.
- Timestamps use the `time` crate through `randommac_core::timefmt` (RFC 3339 with `Z`, the text
  chrono wrote, so 2.0.x files load both ways). chrono is gone (RUSTSEC-2026-0543); don't add it
  back.
- Qodana (`qodana.yml`): `qodana-js` in Docker with JetBrains' Svelte plugin (`qodana.yaml`) and the
  Early Access `qodana-rust` with `--config qodana-rust.yaml` (its bootstrap installs Tauri's build
  dependencies as root). Zero-problem gate: fix findings; an exclusion needs its reason written
  next to it in the file. Needs `QODANA_TOKEN_TS_JS`/`QODANA_TOKEN_RUST` (skipped for Dependabot
  and fork PRs) and an Ultimate license: both jobs fail when it lapses.
- `maintenance-triage.yml` runs on `pull_request_target` (write token, also for fork PRs): it must
  never check out or run PR code, and it reads titles from the event payload instead of having them
  interpolated into the script.
- Accessibility tests: axe-core in jsdom (`src/lib/a11y.test.ts`, helpers in `src/test/axe.ts`) on
  every page and an open dialog. `HTMLDialogElement.showModal` is stubbed (jsdom has no modal
  dialogs) and `color-contrast` is off (no layout) — contrast is checked by hand. New pages and
  dialogs must pass.
- Dependency updates wait three days after publication (Dependabot `cooldown`; do the same for
  manual `cargo update`/npm updates) — security updates don't wait.

## Safety when testing on the maintainer's machine

- Never change the address or the DNS servers of the Ethernet adapter that provides connectivity.
- Real changes only with explicit permission, on the Wi-Fi adapter, and always restore afterwards
  (`randommac-cli restore`, then confirm `NetworkAddress` is gone from the driver key;
  `randommac-cli dns restore`, then confirm with `randommac-cli dns show` that the servers and DoH
  settings are back).
- Prefer `--mock`, `RANDOMMAC_BACKEND=mock` or `--data-dir <tmp>` for experiments.
