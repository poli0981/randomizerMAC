# Developer Machine Specification

> **Public hardware reference** for the primary workstation behind RandomMac. End-user system
> requirements live in [`README.md`](../README.md#system-requirements) — this file documents what
> the developer actually builds and tests on.

## Operating System

| Item     | Detail                          |
|----------|---------------------------------|
| OS       | Windows 11 Pro                  |
| Channel  | Insider Preview — Dev Channel   |
| Build    | 29648.1000                      |

## CPU / GPU / Memory / Storage

| Component | Specification                        |
|-----------|--------------------------------------|
| CPU       | Intel Core i7-14700KF                |
| GPU       | NVIDIA GeForce RTX 5080 (16 GB VRAM) |
| RAM       | 32 GB DDR5                           |
| Storage   | 1 TB SSD                             |

## Network Adapters

| Adapter                          | Use in testing                                                   |
|----------------------------------|------------------------------------------------------------------|
| Ethernet (GIGABYTE onboard)      | Primary connection — **never** used for MAC change tests          |
| Wi-Fi (Realtek RTL8852CE)        | Real change tests; its driver ignores `NetworkAddress` → exercises the rollback path |
| Bluetooth PAN, VirtualBox adapters | Virtual-adapter detection                                       |

## IDE / Editors

| Tool                  | Version                |
|-----------------------|------------------------|
| JetBrains Rider       | 2026.x (primary)       |
| Other JetBrains IDEs  | 2026.x (as needed)     |
| Visual Studio Code    | latest stable          |

See [`dev_env.md`](dev_env.md) for the full toolchain (SDKs, runtimes, language versions).

## Notes

- RandomMac targets Windows and Linux. Linux packages are built and tested in CI (Ubuntu 24.04
  runner, Fedora 43 container); see
  [`DEV_ENVIRONMENT.md` → Tested configurations](DEV_ENVIRONMENT.md#tested-configurations).
- Mobile devices are out of scope.

## See Also

- [`dev_env.md`](dev_env.md) — Developer toolchain (personal reference).
- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Project build & run instructions.
- [`i18n/vi/pc_spec.md`](i18n/vi/pc_spec.md) — Bản tiếng Việt.
