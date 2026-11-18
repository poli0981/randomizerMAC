# Trợ năng

**Rà soát lần cuối**: 2026-11-18 · **Áp dụng cho**: RandomMac 2.1.x (ứng dụng Windows và Linux, CLI)

RandomMac cần dùng được với tất cả mọi người muốn đổi hoặc khôi phục địa chỉ MAC hay cài đặt DNS,
bất kể bạn dùng thiết bị nhập liệu, trình đọc màn hình, thiết lập hiển thị hay ngôn ngữ nào. Tài
liệu này nói về trợ năng (accessibility) của RandomMac: mục tiêu, những gì đã hoạt động, cách kiểm
thử, những gì còn chưa ổn, và cách báo cáo khi bạn gặp rào cản.

RandomMac là dự án do một người làm (xem [DISCLAIMER.md](../../../DISCLAIMER.md)). Những gì viết ở
đây là nỗ lực trung thực trong khả năng có thể, không phải kết quả đã được chứng nhận.

## Mục tiêu

- **Chuẩn hướng tới: [WCAG 2.2](https://www.w3.org/TR/WCAG22/) mức AA.** Ứng dụng Windows và Linux
  dùng chung một giao diện nên mục tiêu này áp dụng cho cả hai. Đây là mục tiêu, không phải tuyên bố
  đã đạt chuẩn: chưa có đợt đánh giá chính thức nào.
- **Thứ tự ưu tiên:**
  1. Mọi thao tác đều làm được chỉ với bàn phím: đổi và khôi phục địa chỉ MAC hay cài đặt DNS, xem
     lịch sử, chỉnh cài đặt và danh sách chặn.
  2. Mọi điều khiển đều có tên, vai trò và trạng thái để trình đọc màn hình đọc lên, bằng ngôn ngữ
     của giao diện.
  3. Nội dung vẫn dễ đọc khi phóng to, ở giao diện sáng hay tối, khi bật giảm chuyển động hoặc dùng
     chủ đề tương phản của Windows.
- **Trợ năng kém đi so với trước là lỗi.** Những lỗi này được phân loại và sửa như mọi lỗi khác (xem
  [Báo cáo vấn đề trợ năng](#báo-cáo-vấn-đề-trợ-năng)).

## Những gì đã hoạt động

### Bàn phím

- Nút, liên kết và điều khiển biểu mẫu đều là phần tử HTML gốc. Không phần tử nào có `tabindex`
  dương.
- **Danh sách adapter** ở trang **Bảng điều khiển** là một listbox: ↑ ↓ Home End để di chuyển giữa
  các adapter, Enter hoặc Space để chọn.
- **Bỏ qua, tới nội dung** là điểm dừng đầu tiên khi nhấn Tab. Khi bạn chuyển sang trang khác, tiêu
  điểm (focus) chuyển đến tiêu đề của trang đó.
- Điều khiển nào đang có tiêu điểm cũng hiện chỉ báo tiêu điểm nhìn thấy được.
- **Hộp thoại xác nhận** là hộp thoại gốc (native dialog):
  - tiêu điểm đặt sẵn ở nút **Huỷ**, lựa chọn an toàn;
  - tiêu điểm không ra ngoài hộp thoại khi hộp thoại đang mở;
  - Escape đóng hộp thoại;
  - sau đó tiêu điểm quay về nút đã mở hộp thoại.
- **Khi điều khiển đang có tiêu điểm biến mất**, tiêu điểm chuyển đến một chỗ hợp lý gần đó. Điều
  này áp dụng khi đóng banner hoặc thông báo, xoá một quy tắc trong danh sách chặn và xoá lịch sử.
- **Trong lúc một thay đổi đang chạy**, các nút **Ngẫu nhiên**, **Áp dụng** và **Khôi phục gốc** vẫn
  nhận tiêu điểm. Chúng được đánh dấu là tạm không dùng được (`aria-disabled`) chứ không bị vô hiệu
  hoá, nên tiêu điểm không bị mất.
- **Các vùng cuộn** (bảng lịch sử, danh sách log, bảng đo độ trễ, phần **Có gì mới** của bản cập
  nhật) nhận tiêu điểm và cuộn được bằng phím mũi tên. Đây là các vùng (region) có nhãn.
- Phím tắt được khai báo bằng `aria-keyshortcuts`. Gợi ý phím tắt trên nút không bị đọc lẫn vào tên
  nút.

### Trình đọc màn hình

- **Cấu trúc trang:**
  - các vùng mốc (landmark): điều hướng, nội dung chính, và một vùng có nhãn dành cho thông báo;
  - mỗi trang có đúng một `h1`.
- `<html lang>` theo ngôn ngữ giao diện ngay từ lúc cửa sổ mở. Tên mỗi ngôn ngữ trong ô chọn ngôn
  ngữ có `lang` riêng.
- **Khi đang thay đổi:**
  - Tiến trình đổi MAC hoặc DNS được đọc lên từng bước trong một live region duy nhất (vùng mà trình
    đọc màn hình tự đọc khi nội dung thay đổi), ví dụ "Bước 2/4: …".
  - Bước hiện tại được đánh dấu bằng `aria-current="step"`.
  - Kết quả của thay đổi được đọc lên một lần.
- **Thông báo trong cửa sổ:**
  - không có live region lồng nhau;
  - thông báo lỗi giữ nguyên cho đến khi bạn đóng;
  - các thông báo khác tự đóng, và tạm dừng khi bạn rê chuột lên hoặc đưa tiêu điểm vào;
  - mỗi nút đóng được đặt tên theo thông báo của nó.
- **Cập nhật:** banner cập nhật là một thông điệp trạng thái (status), chấm báo cập nhật trên thanh
  bên có chữ dành cho trình đọc màn hình, và thanh tiến trình trên trang **Cập nhật** có nhãn.
- Trong danh sách adapter, adapter đang bận mà không phải adapter đang chọn có chữ báo điều đó.
- Công tắc dùng `role="switch"`; phần mô tả và gợi ý của công tắc được liên kết bằng
  `aria-describedby`.
- Bảng ở trang **Lịch sử** có chú thích (caption). Chi tiết lỗi hiện thành chữ, không chỉ nằm trong
  tooltip.
- **Trang DNS:**
  - Các tuỳ chọn mã hoá không dùng được bị vô hiệu hoá, và lý do nằm trong phần mô tả của tuỳ chọn
    đó.
  - Lỗi của từng ô nhập được liên kết với ô đó (`aria-invalid`, `aria-describedby`).
  - Phần **Đo độ trễ** đọc lên tiến trình và kết quả đo trong một live region.
  - Mỗi nút sao chép nêu rõ điểm truy cập (endpoint) mà nó sao chép.
- Biểu tượng trang trí được ẩn khỏi công nghệ hỗ trợ; nút chỉ có biểu tượng đều có tên.
- Thông báo của hệ thống, ví dụ sau một thao tác từ khay, đã được dịch và có kèm mã lỗi khi thao tác
  thất bại.

### Hiển thị

- **Giao diện:** Sáng, Tối và Theo hệ thống.
  - Màu chữ, kể cả chữ mờ, chữ màu nhấn và chữ gợi ý (placeholder), được kiểm tra đạt độ tương phản
    từ 4.5:1 trở lên ở cả hai giao diện.
  - Viền ô nhập, viền ô chọn và rãnh của công tắc được kiểm tra đạt từ 3:1 trở lên.
- **Chủ đề tương phản của Windows** (`forced-colors`): công tắc, adapter đang chọn, trang hiện tại
  trên thanh bên, chip đang chọn, chấm báo cập nhật, thanh tiến trình và nút chính đều dùng màu hệ
  thống.
- **Giảm chuyển động** (`prefers-reduced-motion`): hiệu ứng chuyển động dừng lại. Biểu tượng đang
  tải và thanh tiến trình không xác định đứng yên.
- **Không truyền đạt ý nghĩa chỉ bằng màu:** chấm kết nối của adapter có kèm chữ ghi trạng thái, và
  kết quả luôn được ghi bằng chữ.

### Thu phóng

- Ctrl+= phóng to cửa sổ, Ctrl+− thu nhỏ, Ctrl+0 đưa về mức ban đầu.
- Khi phóng to, thanh bên cuộn được.
- Bố cục đã được kiểm tra ở mức 200 %.
- Cửa sổ nhỏ nhất là 880 × 600.

### Ngôn ngữ

Giao diện có hai ngôn ngữ, English và Tiếng Việt, kể cả văn bản dành cho trình đọc màn hình, menu
khay và thông báo. Bản tiếng Việt do AI dịch; nếu thấy chỗ nào chưa chuẩn, rất
mong bạn góp ý qua mẫu issue
[Language / Translation Fix](https://github.com/poli0981/randomizerMAC/issues/new?template=fix_lang.yml).

## Bảng phím tắt

| Phím | Ở đâu | Tác dụng |
| --- | --- | --- |
| Tab / Shift+Tab | Mọi nơi | Điều khiển tiếp theo / trước đó |
| Enter trên **Bỏ qua, tới nội dung** | Điểm dừng Tab đầu tiên | Nhảy tới nội dung trang |
| Ctrl+1 … Ctrl+8 | Mọi nơi | Mở các trang theo thứ tự trên thanh bên: Bảng điều khiển, DNS, Lịch sử, Cài đặt, Nhật ký, Cập nhật, Giới thiệu, Mạng xã hội |
| F5 | Mọi nơi | Làm mới danh sách adapter |
| Ctrl+R | Bảng điều khiển | **Ngẫu nhiên**: xem trước một địa chỉ mới |
| Ctrl+Enter | Bảng điều khiển | **Áp dụng** địa chỉ mới |
| ↑ ↓ Home End | Danh sách adapter | Di chuyển giữa các adapter |
| Enter / Space | Danh sách adapter | Chọn adapter |
| Escape | Hộp thoại xác nhận | Đóng hộp thoại; tiêu điểm quay về nút đã mở nó |
| Ctrl+= / Ctrl+− / Ctrl+0 | Mọi nơi | Phóng to / thu nhỏ / về mức ban đầu |

## Cách chúng tôi kiểm thử

### Đã kiểm thử với

| Thời gian | Phiên bản | Nền tảng | Cách kiểm thử | Kết quả |
| --- | --- | --- | --- | --- |
| 2026-11 | 2.0.2 | Windows 11, ứng dụng Windows (WebView2) | Narrator; chỉ dùng bàn phím, đã tắt chuột (người bảo trì) | Không có lỗi chặn thao tác |
| Mỗi pull request | — | jsdom trong CI | [axe-core](https://github.com/dequelabs/axe-core) trên mọi trang (kể cả DNS) và trên một hộp thoại xác nhận đang mở; kiểm tra role và tên bằng Testing Library (`src/lib/a11y.test.ts`) | Bắt buộc đạt |

Bản quét tự động chạy trong jsdom, vốn không có bố cục (layout) thật, nên không đo được độ tương
phản màu. Độ tương phản được kiểm tra thủ công.

**Chưa kiểm thử** (rất mong nhận được báo cáo):

- **Trình đọc màn hình:** NVDA và JAWS (Windows); Orca với ứng dụng Linux (WebKitGTK).
- **Điều khiển bằng giọng nói:** Voice Access, Dragon.
- **Hiển thị:** chủ đề tương phản của Windows trên máy thật; tỉ lệ hiển thị (high-DPI) trên 200 %.

### Kiểm tra thủ công trước khi phát hành

1. **Chỉ dùng bàn phím.** Đi hết các luồng chính:
   - Bảng điều khiển: chọn adapter, tạo địa chỉ ngẫu nhiên, áp dụng, khôi phục;
   - DNS: chọn nhà cung cấp, áp dụng, khôi phục, chạy phần đo độ trễ;
   - Lịch sử: lọc và xuất;
   - Cài đặt, kể cả danh sách chặn.

   Kiểm tra rằng tiêu điểm luôn nhìn thấy được và không bao giờ bị kẹt.
2. **Trình đọc màn hình.** Dùng Narrator, NVDA hoặc Orca cho cùng các luồng đó. Chú ý nghe tên,
   trạng thái, lỗi và tiến trình của thay đổi.
3. **Thu phóng.** Kiểm tra ở mức 200 %.
4. **Thiết lập hiển thị.** Kiểm tra với một chủ đề tương phản của Windows, khi bật giảm chuyển động,
   và ở cả giao diện sáng lẫn tối.
5. **Quét tự động.** Chạy `npm test`, lệnh này gồm cả bản quét axe-core.

## Môi trường được hỗ trợ

| Thành phần | Engine | Công nghệ hỗ trợ đã kiểm tra |
| --- | --- | --- |
| Ứng dụng Windows (Windows 10/11) | Microsoft Edge WebView2 | Narrator |
| Ứng dụng Linux (Ubuntu 24.04+, Debian 13+, Fedora 41+) | WebKitGTK | Chưa kiểm tra |
| Công cụ dòng lệnh (`randommac-cli`) | Mọi terminal | Kết quả là văn bản thuần; chưa kiểm thử riêng |

## Hạn chế đã biết

Đây là những rào cản chúng tôi đã biết, kèm cách khắc phục tạm nếu có.

1. **Menu khay là menu gốc của hệ điều hành.**
   - Windows: nhấn Win+B để tới vùng thông báo (notification area).
   - Linux: biểu tượng khay cần desktop hỗ trợ StatusNotifier. GNOME cần extension AppIndicator.

   *Cách khắc phục tạm:* cửa sổ chính làm được những việc của menu khay, trừ hai mục:
   - **Khôi phục tất cả adapter**: khôi phục lần lượt từng adapter ở trang **Bảng điều khiển**.
   - **Thoát**: tắt **Tiếp tục chạy ở khay khi đóng cửa sổ** trong **Cài đặt**; khi đó đóng cửa sổ
     là thoát RandomMac.
2. **Những phần thuộc về hệ điều hành.** Hộp thoại UAC khi RandomMac chạy lại với quyền quản trị,
   hộp thoại chọn tệp khi xuất và nhập, và thông báo hệ thống đều do hệ điều hành vẽ, nên trợ năng
   của chúng phụ thuộc vào hệ điều hành.
3. **Nội dung rộng trong cửa sổ hẹp.** Bảng đo độ trễ và danh sách máy chủ DNS dài cuộn ngang bên
   trong thẻ chứa chúng. *Cách khắc phục tạm:* mở rộng cửa sổ, hoặc thu gọn thanh bên.
4. **Phóng to trên 200 % trên màn hình nhỏ.** Bố cục chưa được thiết kế cho trường hợp này. *Cách
   khắc phục tạm:* thu gọn thanh bên.
5. **Orca.** Ứng dụng Linux chưa được kiểm thử với Orca.
6. **Bản dịch.** Bản tiếng Việt, kể cả văn bản dành cho trình đọc màn hình, do AI dịch; rất mong
   bạn góp ý nếu thấy chỗ nào chưa chuẩn.

## Yêu cầu đối với người đóng góp

Người đóng góp kiểm tra các điểm sau cho mọi thay đổi về giao diện (xem thêm
[CONTRIBUTING.md](../../../CONTRIBUTING.md)):

- **Phần tử.** Dùng nút, liên kết và điều khiển biểu mẫu gốc. Widget tự làm, như danh sách adapter,
  cần role ARIA tương ứng và hỗ trợ bàn phím đầy đủ. Không dùng `tabindex` dương.
- **Tên.** Mọi điều khiển đều có tên truy cập (accessible name) lấy từ `src/lib/i18n/en.ts` và
  `vi.ts`, đủ cả hai ngôn ngữ. Nút chỉ có biểu tượng cần `aria-label`, biểu tượng trang trí cần
  `aria-hidden`.
- **Tiêu điểm.** Vùng cuộn nhận tiêu điểm cần `role="region"` và một nhãn.
- **Chuyển động và màu.** Hiệu ứng chuyển động chỉ chạy khi `prefers-reduced-motion: no-preference`,
  hoặc phải được tắt khi `reduce`. Không truyền đạt ý nghĩa chỉ bằng màu.
- **Nội dung đọc lên.** Dùng các live region sẵn có, và giữ nội dung đọc lên ngắn gọn.
- **Kiểm thử.**
  - `npm test` phải đạt, kể cả bản quét axe-core trong `src/lib/a11y.test.ts`. Hãy mở rộng bản quét
    cho trang và hộp thoại mới.
  - Tự kiểm tra phần mình sửa chỉ bằng bàn phím và ghi rõ điều đó trong pull request.
- **Tài liệu này.** Cập nhật khi hành vi hoặc một hạn chế đã biết thay đổi.

## Báo cáo vấn đề trợ năng

Mở issue bằng biểu mẫu
**[Accessibility issue](https://github.com/poli0981/randomizerMAC/issues/new?template=accessibility.yml)**.
Hãy cho biết:

- bạn đang định làm gì và điều gì đã xảy ra thay vào đó;
- công nghệ hỗ trợ bạn dùng và phiên bản của nó;
- phiên bản RandomMac (ở trang **Giới thiệu**, hoặc `randommac-cli --version`) và hệ điều hành của
  bạn.

Bạn không cần biết nguyên nhân.

Nếu bạn không thể hoặc không muốn dùng GitHub, hãy gửi email tới **accessibility@poli0981.dev**,
tiêu đề có chữ "Accessibility". Vấn đề bảo mật thì báo qua [SECURITY.md](../../../SECURITY.md).

### Mức độ nghiêm trọng

| Mức độ | Ý nghĩa | Ví dụ |
| --- | --- | --- |
| Nghiêm trọng (Critical) | Chặn một thao tác chính và không có cách khắc phục tạm | Bạn không thể đổi hoặc khôi phục địa chỉ MAC hay cài đặt DNS bằng bàn phím hoặc trình đọc màn hình |
| Cao (High) | Rất khó khăn, nhưng có cách khắc phục tạm | Lỗi không được đọc lên, nhưng có hiển thị trên màn hình |
| Trung bình (Medium) | Gây khó hiểu, nhưng vẫn làm được | Nhãn sai hoặc thiếu; tiêu điểm nhảy tới chỗ lạ |
| Thấp (Low) | Phiền toái nhỏ | Đọc lên quá dài dòng; một gợi ý chưa được dịch |

### Cách chúng tôi phản hồi

- Chúng tôi cảm ơn bạn và hỏi thêm nếu có điều gì chưa rõ. Chúng tôi không bao giờ yêu cầu bạn giải
  thích vì sao bạn cần một tính năng trợ năng.
- Chúng tôi gắn nhãn `accessibility`, đặt mức độ nghiêm trọng, và xử lý lỗi chặn thao tác trước lỗi
  về hình thức.
- Chúng tôi chia sẻ cách khắc phục tạm ngay khi tìm ra.
- Khi có bản sửa, nếu bạn sẵn lòng, chúng tôi sẽ nhờ bạn kiểm tra lại. Các bản sửa được ghi trong
  [CHANGELOG.md](../../../CHANGELOG.md).

### Thời gian phản hồi

Đây là mức cố gắng tốt nhất của một dự án do một người làm:

| Mức độ | Phản hồi đầu tiên | Phát hành bản sửa |
| --- | --- | --- |
| Nghiêm trọng | trong vòng 7 ngày | bản vá (patch) kế tiếp |
| Cao | trong vòng 14 ngày | bản phát hành kế tiếp |
| Trung bình / Thấp | trong vòng 30 ngày | khi có thể, thường là bản minor kế tiếp |

## Phụ trách và duy trì

Người bảo trì duy nhất, [@poli0981](https://github.com/poli0981), phụ trách trợ năng, gồm các việc:

- phân loại các báo cáo;
- theo dõi chúng bằng nhãn `accessibility`;
- chạy [bước kiểm tra thủ công](#kiểm-tra-thủ-công-trước-khi-phát-hành) trước mỗi bản phát hành;
- giữ tài liệu này luôn cập nhật, kể cả ngày rà soát.

## Góp ý

Mọi góp ý cho tài liệu này đều được hoan nghênh, qua issue hoặc pull request.

Đây là bản dịch của [bản tiếng Anh](../../../ACCESSIBILITY.md).

Tài liệu này theo cấu trúc được khuyến nghị trong Open Source Guides
([Accessibility best practices for your project](https://opensource.guide/accessibility-best-practices-for-your-project/#start-with-an-accessibility-statement))
và [Open Source Accessibility toolkit](https://github.com/open-source-accessibility/accessibility-toolkit/blob/main/ACCESSIBILITY.md).

Nếu bản dịch này khác bản tiếng Anh, bản tiếng Anh được ưu tiên.
