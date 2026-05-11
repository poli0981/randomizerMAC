# Toolchain phát triển (tham chiếu cá nhân)

> **Phạm vi.** File này liệt kê các toolchain ngôn ngữ mà nhà phát triển
> duy trì cài đặt cho công việc chung — không phải tất cả đều bắt buộc
> cho RANDOM MAC. Để biết **cách build và chạy chính xác dự án này**,
> xem [`DEV_ENVIRONMENT.md`](../../DEV_ENVIRONMENT.md).

## Toolchain ngôn ngữ

| Công cụ  | Phiên bản                                  | Ghi chú                                                          |
|----------|--------------------------------------------|------------------------------------------------------------------|
| .NET SDK | 11.0.100-preview.3+                        | Bắt buộc cho App của RANDOM MAC; pin qua `global.json`           |
| .NET SDK | 9.0+                                       | Bắt buộc cho `RandomMac.Core` + tests                            |
| Python   | 3.12.x                                     | Dòng 3.12 mới nhất                                               |
| Node.js  | LTS ≥ 22, current ≥ 25.8.1                 | Dùng `nvm-windows` để chuyển đổi giữa LTS và current             |
| Rust     | stable (qua `rustup`)                      | `rustup toolchain install stable`                                |
| Git      | 2.x+                                       | Bắt buộc                                                         |

> Khoảng phiên bản Node.js ở trên cố ý bao gồm cả LTS dùng cho hầu hết
> công cụ và bản current mới hơn cho công cụ thử nghiệm. Khi cài, chọn
> bản mới nhất tại thời điểm đó.

## Trình biên tập / IDE

| Công cụ                                                          | Ghi chú                              |
|------------------------------------------------------------------|--------------------------------------|
| JetBrains Rider 2026.x                                           | IDE chính cho .NET / WinUI 3         |
| JetBrains IntelliJ / PyCharm / WebStorm / RustRover 2026.x       | Sử dụng tùy ngôn ngữ                 |
| Visual Studio Code (mới nhất)                                    | Chỉnh sửa nhẹ, markdown, YAML        |

## Quản lý mã nguồn

- **Git**: 2.x trở lên.
- **GPG signing**: bật.
  ```bash
  git config --global commit.gpgsign true
  git config --global user.signingkey <KEY_ID>
  ```
- Kiểm tra bằng `git log --show-signature` trên các commit gần nhất.

## Xem thêm

- [`../../DEV_ENVIRONMENT.md`](../../DEV_ENVIRONMENT.md) — Build, run, publish dự án + pitfalls WinUI 3.
- [`pc_spec.md`](pc_spec.md) — Phần cứng máy phát triển.
- [`../../dev_env.md`](../../dev_env.md) — English version.
