# Toolchain phát triển (tham chiếu cá nhân)

> **Phạm vi.** File này liệt kê các toolchain mà nhà phát triển duy trì cho công việc chung — không
> phải tất cả đều bắt buộc cho RandomMac. Để biết **cách build và chạy chính xác dự án này**, xem
> [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md).

## Toolchain ngôn ngữ

| Công cụ  | Phiên bản                        | Ghi chú                                                                   |
|----------|----------------------------------|---------------------------------------------------------------------------|
| Rust     | 1.98.0 (qua `rustup`)            | **Bắt buộc cho RandomMac 2.x**; cố định bởi `rust-toolchain.toml`         |
| Node.js  | 26.x (`.nvmrc`), npm 11          | **Bắt buộc cho RandomMac 2.x** (giao diện Svelte, Tauri CLI); dùng `nvm-windows` để chuyển phiên bản |
| Git      | 2.x+                             | Bắt buộc                                                                  |
| .NET SDK | 10.x                             | RandomMac 2.x không dùng (bản 1.x là .NET/WinUI 3)                        |
| Python   | 3.12.x                           | RandomMac không dùng                                                      |

> Khi cài, chọn bản vá mới nhất của từng dòng tại thời điểm đó.

## Trình biên tập / IDE

| Công cụ                                                    | Ghi chú                              |
|------------------------------------------------------------|--------------------------------------|
| JetBrains Rider 2026.x                                     | IDE chính cho .NET                   |
| JetBrains IntelliJ / PyCharm / WebStorm / RustRover 2026.x | Sử dụng tùy ngôn ngữ                 |
| Visual Studio Code (mới nhất)                              | Chỉnh sửa nhẹ, Markdown, YAML        |

Extension VS Code nên dùng: *rust-analyzer*, *Svelte for VS Code*, *Tauri*, *ESLint*, *Prettier*.

## Quản lý mã nguồn

- **Git**: 2.x trở lên.
- **GPG signing**: bật.
  ```bash
  git config --global commit.gpgsign true
  git config --global user.signingkey <KEY_ID>
  ```
- Kiểm tra bằng `git log --show-signature` trên các commit gần nhất.

## Xem thêm

- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Build, chạy, kiểm thử và phát hành dự án.
- [`pc_spec.md`](pc_spec.md) — Phần cứng máy phát triển.
- [`../../dev_env.md`](../../dev_env.md) — English version.
