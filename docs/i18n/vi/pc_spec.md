# Thông số máy phát triển

> **Tham chiếu phần cứng công khai** cho máy phát triển chính đằng sau
> RANDOM MAC. Yêu cầu hệ thống cho người dùng cuối nằm ở
> [`README.md`](../../../README.md#system-requirements) — file này ghi
> lại máy mà nhà phát triển thực sự build và test.

## Hệ điều hành

| Mục       | Chi tiết                       |
|-----------|--------------------------------|
| OS        | Windows 11 Pro                 |
| Kênh      | Insider Preview — Dev Channel  |
| Phiên bản | 25H2                           |
| Build     | 26300.8376                     |

## CPU / GPU / Bộ nhớ / Ổ cứng

| Linh kiện | Thông số                             |
|-----------|--------------------------------------|
| CPU       | Intel Core i7-14700KF                |
| GPU       | NVIDIA GeForce RTX 5080 (16 GB VRAM) |
| RAM       | 32 GB DDR5                           |
| Ổ cứng    | 1 TB SSD                             |

## IDE / Trình biên tập

| Công cụ                | Phiên bản                  |
|------------------------|----------------------------|
| JetBrains Rider        | 2026.x (chính)             |
| Các IDE JetBrains khác | 2026.x (khi cần)           |
| Visual Studio Code     | bản ổn định mới nhất       |

Xem [`dev_env.md`](dev_env.md) để biết toolchain đầy đủ (SDK, runtime, phiên bản ngôn ngữ).

## Ghi chú

- Thiết bị di động / iOS không nằm trong phạm vi — RANDOM MAC chỉ hỗ trợ Windows.
- Máy này là **tham chiếu chính** cho các dòng "Passed" trong
  [`DEV_ENVIRONMENT.md`](../../DEV_ENVIRONMENT.md).
- Cấu hình VM (VirtualBox + Windows Sandbox) dùng để kiểm thử khả năng
  tương thích được liệt kê trong `DEV_ENVIRONMENT.md`.

## Xem thêm

- [`dev_env.md`](dev_env.md) — Toolchain phát triển (tham chiếu cá nhân).
- [`../../DEV_ENVIRONMENT.md`](../../DEV_ENVIRONMENT.md) — Hướng dẫn build & chạy dự án.
- [`../../pc_spec.md`](../../pc_spec.md) — English version.
