# AicDesignDC – lõi C++ (dựng lại chức năng từ video)

Kiến trúc: **Ruby (UI/tool SketchUp) ↔ C ABI ↔ lõi C++17** (`core/`). Lõi không phụ thuộc SketchUp nên test được độc lập;
Ruby chỉ lấy JSON danh sách tấm và dựng hình một lần trong một `start_operation`.

| Chức năng trong video | Nơi cài đặt |
|---|---|
| Khung tủ: Rộng/Sâu/Cao, Dày ván 17.5, Dày hậu 9.5, Len chân, Có hậu, Kiểu hậu (Phủ/Âm) | `Params`, `Builder::run` |
| Resize 1600→1207 không méo (ván giữ độ dày, ngăn kéo/cánh co giãn theo tham số) | tính lại từ tham số, test `test_core` |
| Chia khoang đều / cố định, vách đứng, đợt ngang chia đều | `Column`, `autoW`, `shelves` |
| Cánh Trùm ngoài / Trùm nửa / Lọt lòng, khe hở, hộc trên | `cover()/gapShare()`, `doors()` |
| Bản lề tự tính theo chiều cao (2–6) + lỗ chén Ø35 + lỗ đế | `hingesFor`, holes `hinge_cup/plate` |
| Cam chốt (lỗ Ø15 + ty Ø5) — **đã tắt mặc định**, bật bằng `cam=1`, làm sau | `camSpan` |
| Ngăn kéo (ray 12.7, chiều sâu theo bước 50) | khối drawers |
| Bảng cắt + dán cạnh | `cutlist()` |
| Xuất DXF xếp ván 2440×1220 + lỗ khoan | `exportDxf()` |

Build: `cmake -S . -B build && cmake --build build && ./build/test_core`
Đổi tên thư viện sang `hsx_core.dll` (Windows) / `libhsx_core.dylib` (Mac) và đặt cạnh `bridge.rb`.

Chưa làm (cần thêm thông tin/dữ liệu): thư viện mẫu (lưu/nạp), uốn cong (Cong/Phào/Xà), hậu quá khổ cần ghép.
Giả định cần xác nhận: "Hồi lên trước 19" = cánh lọt lòng thụt vào 19mm; cam cách đầu 34mm, cách mép trước/sau 37mm.

## Lớp SketchUp (Ruby) — `ruby/`
Cài: chép `ruby/aic_design_dc.rb` + thư mục `ruby/aic_design_dc/` vào thư mục Plugins; đặt `hsx_core.dll` (Win) / `libhsx_core.dylib` (Mac) vào `aic_design_dc/`.

- **Vẽ tủ** (`draw_tool.rb`): click điểm 1, click điểm 2 (hoặc gõ `rộng,sâu` + Enter), có xem trước khung bao.
- **Thông số** (`dialog.rb`, `dialog.html`): chỉnh khung/khoang, áp dụng trực tiếp vào tủ đang chọn; bảng cắt; xuất DXF.
- **Kéo giãn như bản gốc** (`observers.rb`): dùng công cụ Scale gốc của SketchUp; khi thả chuột, tủ được đưa về scale 1 và dựng lại từ lõi C++
  (ván giữ độ dày, ngăn kéo giữ chiều cao, khoang cố định co theo tỉ lệ). Hoàn tác một lần (Ctrl+Z) trả về cả bước scale.
- Cam chốt: tắt (`cam=0`), làm sau.

Đã kiểm tra: cú pháp Ruby 2.7+, logic `Params`/co giãn khoang/gọi lõi qua Fiddle. Chưa chạy trong SketchUp thật; toolbar chưa có icon.
