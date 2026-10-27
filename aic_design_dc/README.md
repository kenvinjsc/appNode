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
| Uốn cong, đợt bo góc, phào, xà | `core/parts.cpp` |
| Bảng cắt + dán cạnh | `cutlist()` |
| Xuất DXF xếp ván 2440×1220 + lỗ khoan | `exportDxf()` |

Build (Linux/Mac): `cmake -S . -B build && cmake --build build && ./build/test_core`
Build (Windows): `cmake -S . -B build` rồi `cmake --build build --config Release` rồi `build\Release\test_core.exe`;
DLL dùng trong SketchUp là `build\Release\hsx_core.dll` (nên dùng Release, không dùng Debug); đặt cạnh `bridge.rb`
(Mac: `libhsx_core.dylib`, Linux: `libhsx_core.so`).

Chi tiết lẻ (`core/parts.cpp`): uốn cong (`bend`), đợt bo góc (`shelf_round`), phào (`moulding`: cove/round/bevel/step/rect),
xà (`beam`) — biên dạng đa giác đùn theo trục, cung chia theo sai số 0.3 mm; **hậu quá khổ** tự tách theo khổ ván (`sheet_w`).
Hiệu năng: ~0.1 ms/lần giải; C ABI nhớ kết quả lần gọi gần nhất nên Ruby gọi 2 lần chỉ giải 1 lần.

Chưa làm: cam chốt (đã tắt), bo góc/cong cho thân tủ liền khối, nesting tối ưu (hiện xếp dòng), ghép hậu theo khổ 1830.
Giả định cần bạn xác nhận: "Hồi lên trước 19" = cánh lọt lòng thụt vào 19mm; cam cách đầu 34mm, cách mép trước/sau 37mm.

## Lớp SketchUp (Ruby) — `ruby/`
Cài: chép `ruby/aic_design_dc.rb` + thư mục `ruby/aic_design_dc/` vào thư mục Plugins; đặt `hsx_core.dll` (Win) / `libhsx_core.dylib` (Mac) vào `aic_design_dc/`.

- **Vẽ tủ** (`draw_tool.rb`): click điểm 1, click điểm 2 (hoặc gõ `rộng,sâu` + Enter), có xem trước khung bao.
- **Thông số** (`dialog.rb`, `dialog.html`): chỉnh khung/khoang, áp dụng trực tiếp vào tủ đang chọn; bảng cắt; xuất DXF.
- **Kéo giãn như bản gốc** (`observers.rb`): dùng công cụ Scale gốc của SketchUp; khi thả chuột, tủ được đưa về scale 1 và dựng lại từ lõi C++
  (ván giữ độ dày, ngăn kéo giữ chiều cao, khoang cố định co theo tỉ lệ). Hoàn tác một lần (Ctrl+Z) trả về cả bước scale.
- **Thư viện mẫu** (`library.rb`): 7 mẫu tủ + 6 chi tiết có sẵn; lưu/nạp/xóa mẫu riêng tại `~/.aic_design_dc/library.json`.
- **Chi tiết** (`part_tool.rb`): chọn loại trong hộp thoại, click đặt; cạnh cong được làm mịn (soft/smooth) trong SketchUp.
- Icon toolbar dạng SVG (SketchUp 2016+).
- Cam chốt: tắt (`cam=0`), làm sau.

Đã kiểm tra: cú pháp Ruby 2.7+, logic `Params`/thư viện mẫu/co giãn khoang/gọi lõi qua Fiddle. Chưa chạy trong SketchUp thật.
