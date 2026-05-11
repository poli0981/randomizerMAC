# Developer Toolchain (Personal Reference)

> **Scope.** This file lists language toolchains the developer keeps
> installed for general work — not all are required by RANDOM MAC.
> For **build-and-run instructions for this project**, see
> [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md).

## Language Toolchains

| Tool      | Version                                          | Notes                                                          |
|-----------|--------------------------------------------------|----------------------------------------------------------------|
| .NET SDK  | 11.0.100-preview.3+                              | Required by RANDOM MAC App; pinned via `global.json`           |
| .NET SDK  | 9.0+                                             | Required by `RandomMac.Core` + tests                           |
| Python    | 3.12.x                                           | Latest stable 3.12 line                                        |
| Node.js   | LTS ≥ 22, current ≥ 25.8.1                       | Use `nvm-windows` to switch between LTS and current            |
| Rust      | stable (via `rustup`)                            | `rustup toolchain install stable`                              |
| Git       | 2.x+                                             | Required                                                       |

> The Node.js range above intentionally covers both the LTS kept available
> for most tooling and the newer current that experimental tooling needs.
> Pick the latest at the time of install.

## Editors / IDEs

| Tool                                                              | Notes                                  |
|-------------------------------------------------------------------|----------------------------------------|
| JetBrains Rider 2026.x                                            | Primary IDE for .NET / WinUI 3 work    |
| JetBrains IntelliJ / PyCharm / WebStorm / RustRover 2026.x        | Used per language                      |
| Visual Studio Code (latest)                                       | Lightweight edits, markdown, YAML      |

## Version Control

- **Git**: 2.x or newer.
- **GPG signing**: enabled.
  ```bash
  git config --global commit.gpgsign true
  git config --global user.signingkey <KEY_ID>
  ```
- Verify with `git log --show-signature` on recent commits.

## See Also

- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Project-specific build, run, publish, and WinUI 3 pitfalls.
- [`pc_spec.md`](pc_spec.md) — Developer machine hardware.
- [`i18n/vi/dev_env.md`](i18n/vi/dev_env.md) — Bản tiếng Việt.
