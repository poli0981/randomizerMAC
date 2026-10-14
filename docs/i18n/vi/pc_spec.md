# Thông số máy phát triển

> **Tham chiếu phần cứng công khai** cho máy phát triển chính của RandomMac. Yêu cầu hệ thống cho
> người dùng cuối nằm ở [`README.md`](README.md#yêu-cầu-hệ-thống) — file này ghi lại máy mà nhà phát
> triển thực sự build và test.

## Hệ điều hành

| Mục       | Chi tiết                       |
|-----------|--------------------------------|
| OS        | Windows 11 Pro                 |
| Kênh      | Insider Preview — Dev Channel  |
| Build     | 29648.1000                     |

## CPU / GPU / Bộ nhớ / Ổ cứng

| Linh kiện | Thông số                             |
|-----------|--------------------------------------|
| CPU       | Intel Core i7-14700KF                |
| GPU       | NVIDIA GeForce RTX 5080 (16 GB VRAM) |
| RAM       | 32 GB DDR5                           |
| Ổ cứng    | 1 TB SSD                             |

## Card mạng

| Adapter                            | Vai trò khi kiểm thử                                              |
|------------------------------------|-------------------------------------------------------------------|
| Ethernet (onboard GIGABYTE)        | Kết nối chính — **không bao giờ** dùng để thử đổi MAC              |
| Wi-Fi (Realtek RTL8852CE)          | Thử đổi MAC thật; driver bỏ qua `NetworkAddress` → kiểm tra đường hoàn tác |
| Bluetooth PAN, adapter VirtualBox  | Kiểm tra nhận diện adapter ảo                                     |

## IDE / Trình biên tập

| Công cụ                | Phiên bản                  |
|------------------------|----------------------------|
| JetBrains Rider        | 2026.x (chính)             |
| Các IDE JetBrains khác | 2026.x (khi cần)           |
| Visual Studio Code     | bản ổn định mới nhất       |

Xem [`dev_env.md`](dev_env.md) để biết toolchain đầy đủ (SDK, runtime, phiên bản ngôn ngữ).

## Ghi chú

- RandomMac hỗ trợ Windows và Linux. Gói Linux được build và kiểm thử trên CI (runner Ubuntu 24.04,
  container Fedora 43); xem [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md#cấu-hình-đã-kiểm-thử).
- Thiết bị di động không nằm trong phạm vi.

## Xem thêm

- [`dev_env.md`](dev_env.md) — Toolchain phát triển (tham chiếu cá nhân).
- [`DEV_ENVIRONMENT.md`](DEV_ENVIRONMENT.md) — Hướng dẫn build & chạy dự án.
- [`../../pc_spec.md`](../../pc_spec.md) — English version.
