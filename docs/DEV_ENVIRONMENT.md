# Development environment

How to build, run, test and release RandomMac 2.x. For the design, see
[ARCHITECTURE.md](ARCHITECTURE.md). Vietnamese: [i18n/vi/DEV_ENVIRONMENT.md](i18n/vi/DEV_ENVIRONMENT.md).

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Git | 2.x | |
| [rustup](https://rustup.rs/) | any | `rust-toolchain.toml` pins Rust **1.100.0** (+ rustfmt, clippy); rustup installs it on first use. MSRV is 1.90 (Tauri's minimum). |
| [Node.js](https://nodejs.org/) | **26** (`.nvmrc`) | npm 11+. |
| Tauri prerequisites | see below | WebView2/WebKitGTK and native build tools. |

### Windows 10/11

- **Microsoft C++ Build Tools** (or Visual Studio) with *Desktop development with C++* (MSVC v143+
  and a Windows 10/11 SDK).
- **WebView2 runtime** — built into Windows 11; on Windows 10 install the Evergreen runtime.
- NSIS and WiX are downloaded automatically by the Tauri CLI when you build installers.

### Ubuntu 24.04+ / Debian 13+

```bash
sudo apt update
sudo apt install build-essential curl wget file patchelf libssl-dev libxdo-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

To exercise real changes you also need `network-manager` and/or `iproute2`, plus `pkexec`
(`polkitd` + `pkexec` on Ubuntu 24.04, `policykit-1` on older releases).

### Fedora 41+

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install c-development
```

## Getting started

```bash
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC
npm ci
npm run tauri dev          # desktop app with hot reload (Vite on http://localhost:1420)
```

Useful variations:

| Goal | Command / variable |
|---|---|
| UI only, in a normal browser, with demo data | `npm run dev`, open http://localhost:1420 (`src/lib/devmock.ts` fakes the backend) |
| Desktop app with demo adapters (debug builds) | `RANDOMMAC_BACKEND=mock npm run tauri dev` |
| Demo scenarios | `RANDOMMAC_MOCK_SCENARIO=driver_ignores` (rollback path) or `slow` |
| Real changes on Windows (debug builds don't self-elevate) | run the terminal as administrator, or set `RANDOMMAC_ELEVATE=1` |
| Verbose logs | `RANDOMMAC_LOG=debug` (any `tracing` filter, e.g. `randommac_net=trace`) |
| CLI against demo adapters | `cargo run -p randommac-cli -- --mock list --all` |
| CLI with throwaway data | `cargo run -p randommac-cli -- --data-dir ./tmp-data list` |
| Run the GUI as root on Linux (not recommended) | `RANDOMMAC_ALLOW_ROOT=1` |

On Linux, real changes can be tested safely on a dummy interface (this is what CI does):

```bash
sudo ip link add dummy0 type dummy && sudo ip link set dummy0 up
cargo build -p randommac-cli
sudo ./target/debug/randommac-cli --data-dir /tmp/rm --method iproute2 set dummy0 02:11:22:33:44:55
sudo ./target/debug/randommac-cli --data-dir /tmp/rm --method iproute2 restore dummy0
sudo ip link del dummy0
```

DNS changes on Linux need NetworkManager; CI drives them on the dummy interface through the
stand-ins `crates/randommac-net/tests/fake-nmcli.sh` and `fake-busctl.sh` (see the *Linux MAC
change* job in `ci.yml`). The demo backend (`--mock`, `RANDOMMAC_BACKEND=mock`) covers DNS too.

## Checks (same as CI)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run lint            # eslint + prettier --check
npm run check           # svelte-check --tsgo: TypeScript 7 type check (warnings fail)
npm test                # vitest, including the accessibility tests (axe-core)
npm run build           # vite build into dist/ (needed by cargo builds of src-tauri)
node scripts/version.mjs check
cargo deny check        # advisories, licenses, bans (cargo install cargo-deny)
npm run check:licenses  # every npm package's license against scripts/lib/licenses.mjs
```

`npm test` includes the accessibility tests (`src/lib/a11y.test.ts`, helpers in `src/test/axe.ts`):
[axe-core](https://github.com/dequelabs/axe-core) on every page and on an open confirmation dialog,
plus the roles and names keyboard and screen-reader users rely on, against the dev mock in jsdom.
jsdom has no layout, so color contrast is checked by hand (see
[ACCESSIBILITY.md](../ACCESSIBILITY.md)). `npm run check:licenses` is the npm counterpart of
`cargo deny check licenses` (CI runs it in the *Dependency audit* job): allow a new license in
`scripts/lib/licenses.mjs` only after reading its terms.

### Coverage and SonarQube Cloud

CI runs the Rust tests with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) on Windows
and Linux (each report covers its own backend), the instrumented CLI in the two integration jobs
(real MAC and DNS changes on Linux, DNS changes on the Windows runner) and the UI tests with Vitest
coverage, then sends the reports with the analysis to
[SonarQube Cloud](https://sonarcloud.io/project/overview?id=poli0981_randomizerMAC). The
*Sonar way* quality gate fails the CI job when new code has less than 80 % coverage, new issues,
more than 3 % duplication or unreviewed security hotspots. What is deliberately not measured (the
Tauri runtime glue, build scripts, release tooling) is listed with the reason in
`sonar-project.properties`. Pull requests from forks and from Dependabot are not analyzed (no
`SONAR_TOKEN`). Locally:

```bash
rustup component add llvm-tools && cargo install cargo-llvm-cov --locked
cargo llvm-cov --workspace --html   # target/llvm-cov/html/index.html
npm run test:coverage               # coverage/lcov-report/index.html (.ts modules; .svelte files are not analyzed)
```

`npm run check` type-checks with **TypeScript 7**: `@typescript/native` is an npm alias of
`typescript@7`, and svelte-check `--tsgo` runs its `tsc` on the svelte2tsx output in
`.svelte-check/` (gitignored). The `typescript` package itself is an alias of
`@typescript/typescript6`, because svelte-check, svelte2tsx and typescript-eslint still need the
TypeScript 6 API, which TypeScript 7.0 does not ship. Drop `--tsgo` to compare with TypeScript 6:
`npx svelte-check --tsconfig ./tsconfig.json`.

`cargo build -p randommac` needs `dist/` to exist — run `npm run build` once first.

From Windows you can type-check the Linux backend and CLI (they have no GTK dependency):

```bash
rustup target add x86_64-unknown-linux-gnu
cargo clippy -p randommac-net -p randommac-cli --target x86_64-unknown-linux-gnu -- -D warnings
```

### JetBrains Qodana

`.github/workflows/qodana.yml` (workflow *Qodana*) analyzes every push to `master` and every pull
request with two linters, each reporting to its own Qodana Cloud project:

| Linter | Scope | Configuration |
|---|---|---|
| `qodana-js` (Docker, with JetBrains' Svelte plugin) | the UI (`src/`), build configuration, `scripts/`, workflows | `qodana.yaml` |
| `qodana-rust` (Early Access, Docker) | the Cargo workspace (`crates/`, `src-tauri/`) | `qodana-rust.yaml` |

Both configurations hold a quality gate of zero problems, and every exclusion is listed in them with
its reason. The jobs need the projects' tokens — the repository secrets `QODANA_TOKEN_TS_JS` and
`QODANA_TOKEN_RUST` — so pull requests from forks and from Dependabot skip the analysis (`master`
is analyzed after the merge). Both linters need a Qodana Ultimate license; when it lapses, both
jobs fail at the license check. Results: Qodana Cloud, annotations on the run and the
`qodana-report-*` artifacts (SARIF and logs).

## Building installers locally

```bash
node scripts/build-sidecar.mjs      # builds randommac-cli into src-tauri/binaries/
```

Windows:

```bash
npx tauri build --bundles nsis,msi --config '{"bundle":{"externalBin":["binaries/randommac-cli"]}}'
```

Linux:

```bash
npx tauri build --bundles deb,rpm,appimage --config '{"bundle":{"externalBin":["binaries/randommac-cli"]}}'
```

Output: `target/release/bundle/`. Release builds use `src-tauri/tauri.release.conf.json`, which also
creates signed updater artifacts and therefore needs `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Repository layout

```
Cargo.toml                 workspace (version, shared dependencies, release profile)
rust-toolchain.toml        pinned Rust toolchain
deny.toml                  cargo-deny policy
crates/
  randommac-core/          OS-independent logic: MAC type, generator, OUI table, blacklist,
                           history, settings (+ v1 migration), DNS (dns/: model, catalog.json,
                           presets, snapshots, wire format), timefmt.rs, atomic storage, paths, lock
    tests/vectors/         test vectors shared by Rust and TypeScript
  randommac-net/           NetworkBackend trait; windows.rs + windows_dns.rs, linux.rs +
                           linux_dns.rs, mock.rs; plain-data halves testable on any OS
                           (linux_parse.rs, win_classify.rs, win_dns.rs, nm_dns.rs);
                           ops.rs = validate → apply → verify → record, dns_ops.rs = the same
                           for DNS (with snapshots), dns_probe.rs = latency test
    tests/                 fake-nmcli.sh, fake-busctl.sh (stand-ins used by the Linux CI job)
  randommac-cli/           randommac-cli, dns.rs = `dns` subcommands (+ tests/cli.rs)
src-tauri/
  src/                     app entry, commands, tray (+ tray_menu.rs: DNS submenu), autostart,
                           elevation, updater, logging, scheduler, legacy (v1) cleanup,
                           platform/{windows,linux}.rs
  capabilities/, permissions/   what the UI may call
  tauri*.conf.json         base, per-OS and release configuration
  windows/hooks.nsh        NSIS install/uninstall hooks
src/                       Svelte 5 UI: App.svelte, lib/{api,stores,i18n,components,pages};
                           page logic in tested .ts modules (lib/dns.ts, history.ts, nav.ts);
                           accessibility tests in lib/a11y.test.ts (+ test/axe.ts)
scripts/                   version.mjs, gen-oui.mjs, build-sidecar.mjs, release-assets.mjs,
                           lcov-paths.mjs (coverage paths for SonarQube Cloud),
                           check-licenses.mjs (+ lib/licenses.mjs: npm license allow-list)
.github/workflows/         ci.yml (+ SonarQube Cloud job), qodana.yml, release.yml,
                           maintenance-triage.yml, Discord/Discussions notifications
sonar-project.properties   SonarQube Cloud analysis scope and coverage reports
qodana.yaml                Qodana for JS/TS (UI, scripts, workflows)
qodana-rust.yaml           Qodana for Rust (the Cargo workspace)
```

## Conventions

- **Commits**: [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`,
  `docs:`, `chore(deps):`, `ci(deps):`, `feat!:` for breaking changes).
- **Rust**: `rustfmt` defaults (`rustfmt.toml`), clippy clean with `-D warnings`, no `unwrap()` on
  user or OS input, errors carry a stable `code` that the UI translates.
- **UI strings**: add every key to `src/lib/i18n/en.ts` (source of truth) and
  `src/lib/i18n/vi.ts`; the test suite fails when a key is missing. Rust-side strings (tray,
  notifications, dialogs) live in `src-tauri/src/i18n.rs`.
- **New Tauri command**: implement it in `commands.rs`, register it in `lib.rs`, add it to the
  `AppManifest` list in `build.rs` and to `permissions/app-commands.toml`.
- **Versions**: never edit version numbers by hand — use `npm run version:set`.

## Releasing

1. `npm run version:set 2.1.0` — updates `Cargo.toml`, `Cargo.lock`, `package.json`,
   `package-lock.json` (`tauri.conf.json` has no version; it uses `Cargo.toml`).
2. Add `## [2.1.0] - YYYY-MM-DD` to `CHANGELOG.md` and write `docs/RELEASE_v2.1.0.md` (release
   page body and in-app update notes — **use absolute links only**).
3. Open a PR, wait for CI, merge into `master`.
4. Tag the merge commit and push the tag:
   ```bash
   git tag -a v2.1.0 -m "RandomMac 2.1.0"
   git push origin v2.1.0
   ```
5. The **Release** workflow checks the version files against the tag, builds and signs all bundles
   plus the CLI archives, verifies the asset set and every updater signature (against the public
   key in `tauri.conf.json`), writes `latest.json` and `SHA256SUMS.txt`, attests build provenance,
   publishes the release and dispatches the Discord and Discussions announcements.

If a run fails before the release is published, re-run *Actions → Release → Run workflow* with the
tag (`dry_run` builds and verifies without publishing). A re-run always builds the **tagged
sources** with the workflow from the branch you dispatch it on — so workflow or infrastructure
fixes can go to `master` first, but a mistake in the sources needs a new patch version: never move
a tag. Once a release is published, re-runs refuse to touch it. Tags with a suffix
(`v2.1.0-beta.1`) become pre-releases and are not offered to the updater.

Repository secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (updater
signing — **back up the key**, without it no update can ever be delivered), `DISCORD_*` webhooks,
`SONAR_TOKEN` (SonarQube Cloud analysis in CI), `QODANA_TOKEN_TS_JS` and `QODANA_TOKEN_RUST`
(the two Qodana Cloud projects).

## Tested configurations

| Environment | Coverage |
|---|---|
| Windows 11 Pro Insider (developer machine, see [pc_spec.md](pc_spec.md)) | App, tray, read-only mode, CLI, real change attempt on a Wi-Fi adapter with rollback, real DNS changes on the Wi-Fi adapter (plain DNS, DoH, automatic, restore), Narrator and keyboard-only use, signed NSIS/MSI release builds with signature verification |
| GitHub `windows-latest` | Build, tests, real DNS changes on the runner's adapter (plain DNS, DoH, restore, automatic), NSIS + MSI install/uninstall, legacy task cleanup, GUI smoke test |
| GitHub `ubuntu-24.04` | Build, tests, real MAC and DNS changes on a dummy interface (iproute2, and NetworkManager through stand-in `nmcli` and `busctl`), `.deb` install + GUI smoke test (Xvfb) |
| `fedora:43` container | `.rpm` installation and dependency resolution |

## Data locations

See [README → Where your data lives](../README.md#where-your-data-lives). Debug and release builds
share the same folders; use `--data-dir` (CLI) or the mock backend when experimenting.
