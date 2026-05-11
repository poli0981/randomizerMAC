# Developer Machine Specification

> **Public hardware reference** for the primary workstation behind
> RANDOM MAC. End-user system requirements live in
> [`README.md`](../README.md#system-requirements) — this file documents
> what the developer actually builds and tests on.

## Operating System

| Item     | Detail                          |
|----------|---------------------------------|
| OS       | Windows 11 Pro                  |
| Channel  | Insider Preview — Dev Channel   |
| Release  | 25H2                            |
| Build    | 26300.8376                      |

## CPU / GPU / Memory / Storage

| Component | Specification                       |
|-----------|-------------------------------------|
| CPU       | Intel Core i7-14700KF               |
| GPU       | NVIDIA GeForce RTX 5080 (16 GB VRAM)|
| RAM       | 32 GB DDR5                          |
| Storage   | 1 TB SSD                            |

## IDE / Editors

| Tool                  | Version                |
|-----------------------|------------------------|
| JetBrains Rider       | 2026.x (primary)       |
| Other JetBrains IDEs  | 2026.x (as needed)     |
| Visual Studio Code    | latest stable          |

See [`dev_env.md`](dev_env.md) for the full toolchain (SDKs, runtimes, language versions).

## Notes

- Mobile / iOS devices are intentionally out of scope here — RANDOM MAC is Windows-only.
- This machine is the **primary reference** for "Passed" rows in
  [`DEV_ENVIRONMENT.md` → Tested Configurations](DEV_ENVIRONMENT.md).
- VM configurations (VirtualBox + Windows Sandbox) used for compatibility
  testing are documented in `DEV_ENVIRONMENT.md`.

## See Also

- [`dev_env.md`](dev_env.md) — Developer toolchain (personal reference).
- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Project build & run instructions.
- [`i18n/vi/pc_spec.md`](i18n/vi/pc_spec.md) — Bản tiếng Việt.
