# Developer Toolchain (Personal Reference)

> **Scope.** This file lists the toolchains the developer keeps installed for general work — not
> all of them are required by RandomMac. For **build-and-run instructions for this project**, see
> [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md).

## Language Toolchains

| Tool      | Version                          | Notes                                                                    |
|-----------|----------------------------------|--------------------------------------------------------------------------|
| Rust      | 1.98.0 (via `rustup`)            | **Required by RandomMac 2.x**; pinned by `rust-toolchain.toml`           |
| Node.js   | 26.x (`.nvmrc`), npm 11          | **Required by RandomMac 2.x** (Svelte UI, Tauri CLI); `nvm-windows` to switch |
| Git       | 2.x+                             | Required                                                                 |
| .NET SDK  | 10.x                             | Not used by RandomMac 2.x (1.x was .NET/WinUI 3)                         |
| Python    | 3.12.x                           | Not used by RandomMac                                                    |

> Pick the latest patch release of each line at the time of install.

## Editors / IDEs

| Tool                                                       | Notes                                   |
|------------------------------------------------------------|-----------------------------------------|
| JetBrains Rider 2026.x                                     | Primary IDE for .NET work               |
| JetBrains IntelliJ / PyCharm / WebStorm / RustRover 2026.x | Used per language                       |
| Visual Studio Code (latest)                                | Lightweight edits, Markdown, YAML       |

Recommended VS Code extensions: *rust-analyzer*, *Svelte for VS Code*, *Tauri*, *ESLint*,
*Prettier*.

## Version Control

- **Git**: 2.x or newer.
- **GPG signing**: enabled.
  ```bash
  git config --global commit.gpgsign true
  git config --global user.signingkey <KEY_ID>
  ```
- Verify with `git log --show-signature` on recent commits.

## See Also

- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Project-specific build, run, test and release.
- [`pc_spec.md`](pc_spec.md) — Developer machine hardware.
- [`i18n/vi/dev_env.md`](i18n/vi/dev_env.md) — Bản tiếng Việt.
