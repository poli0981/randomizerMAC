# Môi trường phát triển

Cách build, chạy, kiểm thử và phát hành RandomMac 2.x. Thiết kế tổng thể: xem
[ARCHITECTURE.md](../../ARCHITECTURE.md). English: [DEV_ENVIRONMENT.md](../../DEV_ENVIRONMENT.md)
(bản gốc — nếu hai bản khác nhau, bản tiếng Anh đúng).

## Yêu cầu

| Công cụ | Phiên bản | Ghi chú |
|---|---|---|
| Git | 2.x | |
| [rustup](https://rustup.rs/) | bất kỳ | `rust-toolchain.toml` cố định Rust **1.98.0** (+ rustfmt, clippy); rustup tự cài ở lần dùng đầu. MSRV là 1.89. |
| [Node.js](https://nodejs.org/) | **26** (`.nvmrc`) | npm 11+. |
| Thành phần cho Tauri | xem bên dưới | WebView2/WebKitGTK và công cụ build native. |

### Windows 10/11

- **Microsoft C++ Build Tools** (hoặc Visual Studio) với *Desktop development with C++* (MSVC v143+ và
  Windows 10/11 SDK).
- **WebView2 runtime** — có sẵn trên Windows 11; trên Windows 10 cài bản Evergreen.
- NSIS và WiX được Tauri CLI tự tải khi build bộ cài.

### Ubuntu 24.04+ / Debian 13+

```bash
sudo apt update
sudo apt install build-essential curl wget file patchelf libssl-dev libxdo-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

Để thử đổi MAC thật cần thêm `network-manager` và/hoặc `iproute2`, cùng `pkexec` (`polkitd` + `pkexec`
trên Ubuntu 24.04, `policykit-1` trên bản cũ hơn).

### Fedora 41+

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install c-development
```

## Bắt đầu

```bash
git clone https://github.com/poli0981/randomizerMAC.git
cd randomizerMAC
npm ci
npm run tauri dev          # app desktop có hot reload (Vite ở http://localhost:1420)
```

Các biến thể hữu ích:

| Mục đích | Lệnh / biến môi trường |
|---|---|
| Chỉ giao diện, trong trình duyệt thường, dữ liệu giả lập | `npm run dev`, mở http://localhost:1420 (`src/lib/devmock.ts` giả lập backend) |
| App desktop với adapter giả lập (bản debug) | `RANDOMMAC_BACKEND=mock npm run tauri dev` |
| Kịch bản giả lập | `RANDOMMAC_MOCK_SCENARIO=driver_ignores` (đường hoàn tác) hoặc `slow` |
| Đổi MAC thật trên Windows (bản debug không tự xin quyền Admin) | mở terminal bằng quyền Administrator, hoặc đặt `RANDOMMAC_ELEVATE=1` |
| Log chi tiết | `RANDOMMAC_LOG=debug` (bộ lọc `tracing` bất kỳ, ví dụ `randommac_net=trace`) |
| CLI với adapter giả lập | `cargo run -p randommac-cli -- --mock list --all` |
| CLI với dữ liệu tạm | `cargo run -p randommac-cli -- --data-dir ./tmp-data list` |
| Chạy GUI bằng root trên Linux (không khuyến khích) | `RANDOMMAC_ALLOW_ROOT=1` |

Trên Linux có thể thử đổi MAC thật an toàn trên một interface giả (CI cũng làm vậy):

```bash
sudo ip link add dummy0 type dummy && sudo ip link set dummy0 up
cargo build -p randommac-cli
sudo ./target/debug/randommac-cli --data-dir /tmp/rm --method iproute2 set dummy0 02:11:22:33:44:55
sudo ./target/debug/randommac-cli --data-dir /tmp/rm --method iproute2 restore dummy0
sudo ip link del dummy0
```

## Kiểm tra (giống CI)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run lint            # eslint + prettier --check
npm run check           # svelte-check --tsgo: kiểm tra kiểu bằng TypeScript 7 (cảnh báo cũng tính là lỗi)
npm test                # vitest
npm run build           # vite build ra dist/ (cần cho cargo build src-tauri)
node scripts/version.mjs check
cargo deny check        # advisories, giấy phép, bans (cargo install cargo-deny)
```

### Coverage và SonarQube Cloud

CI chạy test Rust bằng [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) trên Windows và
Linux (mỗi báo cáo phủ backend của hệ điều hành đó) và test UI bằng Vitest có coverage, rồi gửi các
báo cáo cùng lần phân tích lên
[SonarQube Cloud](https://sonarcloud.io/project/overview?id=poli0981_randomizerMAC). Quality gate
*Sonar way* làm job CI thất bại khi code mới có coverage dưới 80 %, có issue mới, trùng lặp quá 3 %
hoặc còn security hotspot chưa review. Những phần cố ý không đo (glue của Tauri runtime, build
script, công cụ phát hành) được liệt kê kèm lý do trong `sonar-project.properties`. Pull request từ
fork và từ Dependabot không được phân tích (không có `SONAR_TOKEN`). Chạy trên máy:

```bash
rustup component add llvm-tools && cargo install cargo-llvm-cov --locked
cargo llvm-cov --workspace --html   # target/llvm-cov/html/index.html
npm run test:coverage               # coverage/lcov-report/index.html (module .ts; file .svelte không được phân tích)
```

`npm run check` kiểm tra kiểu bằng **TypeScript 7**: `@typescript/native` là npm alias của
`typescript@7`, và svelte-check `--tsgo` chạy `tsc` của nó trên kết quả svelte2tsx trong
`.svelte-check/` (đã gitignore). Bản thân gói `typescript` là alias của `@typescript/typescript6`,
vì svelte-check, svelte2tsx và typescript-eslint vẫn cần API của TypeScript 6 mà TypeScript 7.0
không có. Bỏ `--tsgo` để so sánh với TypeScript 6: `npx svelte-check --tsconfig ./tsconfig.json`.

`cargo build -p randommac` cần thư mục `dist/` — chạy `npm run build` một lần trước.

Từ Windows có thể kiểm tra kiểu backend Linux và CLI (không phụ thuộc GTK):

```bash
rustup target add x86_64-unknown-linux-gnu
cargo clippy -p randommac-net -p randommac-cli --target x86_64-unknown-linux-gnu -- -D warnings
```

## Build bộ cài trên máy

```bash
node scripts/build-sidecar.mjs      # build randommac-cli vào src-tauri/binaries/
```

Windows:

```bash
npx tauri build --bundles nsis,msi --config '{"bundle":{"externalBin":["binaries/randommac-cli"]}}'
```

Linux:

```bash
npx tauri build --bundles deb,rpm,appimage --config '{"bundle":{"externalBin":["binaries/randommac-cli"]}}'
```

Kết quả nằm ở `target/release/bundle/`. Bản phát hành dùng thêm `src-tauri/tauri.release.conf.json`
(tạo artifact updater có chữ ký) nên cần `TAURI_SIGNING_PRIVATE_KEY` và
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Cấu trúc repo

Xem [bản tiếng Anh](../../DEV_ENVIRONMENT.md#repository-layout).

## Quy ước

- **Commit**: [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`,
  `chore(deps):`, `ci(deps):`, `feat!:` cho thay đổi phá vỡ tương thích).
- **Rust**: định dạng bằng `rustfmt`, clippy sạch với `-D warnings`, không `unwrap()` trên dữ liệu từ
  người dùng hoặc hệ điều hành, lỗi có `code` cố định để UI dịch.
- **Chuỗi giao diện**: thêm mọi key vào `src/lib/i18n/en.ts` (nguồn chuẩn) và `src/lib/i18n/vi.ts`;
  test sẽ lỗi nếu thiếu key. Chuỗi phía Rust (khay, thông báo, hộp thoại) nằm ở
  `src-tauri/src/i18n.rs`.
- **Thêm Tauri command**: viết trong `commands.rs`, đăng ký trong `lib.rs`, thêm vào danh sách
  `AppManifest` trong `build.rs` và `permissions/app-commands.toml`.
- **Phiên bản**: không sửa tay — dùng `npm run version:set`.

## Phát hành

1. `npm run version:set 2.1.0` — cập nhật `Cargo.toml`, `Cargo.lock`, `package.json`,
   `package-lock.json` (`tauri.conf.json` không chứa version, lấy từ `Cargo.toml`).
2. Thêm `## [2.1.0] - YYYY-MM-DD` vào `CHANGELOG.md` và viết `docs/RELEASE_v2.1.0.md` (nội dung trang
   Release và ghi chú cập nhật trong app — **chỉ dùng link tuyệt đối**).
3. Mở PR, chờ CI, merge vào `master`.
4. Gắn tag cho commit merge và push tag:
   ```bash
   git tag -a v2.1.0 -m "RandomMac 2.1.0"
   git push origin v2.1.0
   ```
5. Workflow **Release** kiểm tra version khớp tag, build và ký mọi gói cùng tệp nén CLI, kiểm tra đủ
   bộ asset và từng chữ ký updater (với khoá công khai trong `tauri.conf.json`), tạo `latest.json` và
   `SHA256SUMS.txt`, chứng thực nguồn gốc bản build, publish release và kích hoạt thông báo Discord +
   Discussions.

Nếu lần chạy lỗi trước khi release được publish: chạy lại *Actions → Release → Run workflow* với tag đó
(`dry_run` chỉ build và kiểm tra, không publish). Lần chạy lại luôn build **mã nguồn của tag** bằng
workflow của nhánh bạn chọn khi chạy — nên lỗi ở workflow/hạ tầng có thể sửa trên `master` trước, còn
lỗi trong mã nguồn thì phải phát hành bản vá mới: không bao giờ di chuyển tag. Khi release đã publish,
chạy lại sẽ từ chối động vào nó. Tag có hậu tố (`v2.1.0-beta.1`) thành bản pre-release và không được
đưa vào updater.

Secret của repo: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (ký bản cập nhật —
**hãy sao lưu khoá**, mất khoá thì không thể phát hành bản cập nhật tự động nữa), các webhook
`DISCORD_*`, `SONAR_TOKEN` (phân tích SonarQube Cloud trong CI).

## Cấu hình đã kiểm thử

| Môi trường | Phạm vi |
|---|---|
| Windows 11 Pro Insider (máy phát triển, xem [pc_spec.md](pc_spec.md)) | App, khay, chế độ chỉ xem, CLI, thử đổi MAC thật trên adapter Wi-Fi có hoàn tác, build bản cài NSIS/MSI có ký và kiểm tra chữ ký |
| GitHub `windows-latest` | Build, test, cài/gỡ NSIS + MSI, dọn tác vụ cũ, smoke test GUI |
| GitHub `ubuntu-24.04` | Build, test, đổi MAC thật trên interface giả (iproute2, và NetworkManager qua một `nmcli` giả lập), cài `.deb` + smoke test GUI (Xvfb) |
| Container `fedora:43` | Cài `.rpm` và giải quyết phụ thuộc |
