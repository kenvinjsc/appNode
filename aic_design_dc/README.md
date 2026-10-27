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
| Cam chốt: tự dò mối nối chữ T giữa các tấm thân tủ → cam Ø15, lỗ ty Ø8, ty vít Ø5, chốt gỗ Ø8 | `core/joinery.cpp` (`addJoinery`) |
| Dán cạnh theo độ phơi lộ (cạnh bị tấm khác che/chạm sàn thì không dán); mặt cánh/hộc kéo dán 4 cạnh; kích thước cắt = thành phẩm − chỉ | `applyBanding`, `cutlist()` |
| Gom chi tiết nhiều tủ + xếp ván MaxRects (kerf, lề, xoay tuỳ chọn), chạy **luồng nền** | `core/nest.cpp`, `ruby/.../nest_session.rb` |
| Tấm ván gia công: bo/vát góc (lồi/lõm/vát), mộng ngón/đuôi én/chữ T (dương/âm), khử dao, đục khung + hèm kính, phay rãnh; ván uốn kerf; cánh khung/kính | `core/board.cpp`, `core/geom.hpp` |
| Ngăn kéo (ray 12.7, chiều sâu theo bước 50) | khối drawers |
| Bảng cắt + dán cạnh | `cutlist()` |
| Xuất DXF xếp ván 2440×1220 + lỗ khoan | `exportDxf()` |

Build (Linux/Mac): `cmake -S . -B build && cmake --build build && ./build/test_core`
Build (Windows): `cmake -S . -B build` rồi `cmake --build build --config Release` rồi `build\Release\test_core.exe`;
DLL dùng trong SketchUp là `build\Release\hsx_core.dll` (nên dùng Release, không dùng Debug).
Đặt thư viện (`hsx_core.dll` Windows / `libhsx_core.dylib` Mac / `libhsx_core.so` Linux) cạnh `bridge.rb`.

Bổ sung: **chi tiết lẻ** (`core/parts.cpp`): uốn cong (`bend`), đợt bo góc (`shelf_round`), phào (`moulding`: cove/round/bevel/step/rect),
xà (`beam`) — biên dạng đa giác đùn theo trục, cung chia theo sai số 0.3 mm; **hậu quá khổ** tự tách theo khổ ván (`sheet_w`).
Hiệu năng: ~0.1 ms/lần giải; C ABI nhớ kết quả lần gọi gần nhất nên Ruby gọi 2 lần chỉ giải 1 lần.

Chưa làm: bo góc/cong cho thân tủ liền khối, khấu ván khi các tấm giao cắt (cần cắt boolean trên hình học có sẵn), ghép hậu theo khổ 1830.
Giả định cần bạn xác nhận: "Hồi lên trước 19" = cánh lọt lòng thụt vào 19mm; cam cách đầu tấm 34mm, cách đầu mối nối 37mm, bước tối đa 320mm; chốt gỗ Ø8 nằm giữa hai cam.

## Quy trình dựng tủ (như video): khung rỗng → thêm từng thứ
1. **Vẽ tủ mới** (Tab đổi mẫu khung rỗng ngay lúc vẽ, không cần mở hộp thoại) / nạp mẫu "Khung tủ … (rỗng)" ở Thư viện: chỉ có khung (hông, đáy, nóc, hậu).
2. **Đợt / Vách**: bấm vào khoang để chia tại con trỏ (xem trước ván); **phím 2..9** = chia ngay thành N khoang đều (không cần Enter); gõ `200` + Enter = cách mép 200 mm; **Alt/Tab** đổi đợt ↔ vách.
3. **Ngăn kéo**: bấm vào khoang (phím 1..9 = số ngăn, Tab = đặt ↔ xóa), đặt từ đáy khoang lên (số ngăn, cao mỗi ngăn trên thanh tuỳ chọn).
4. **Cánh** (phím 1..4 = số cánh, Tab = đổi kiểu cánh): Trùm ngoài / Trùm nửa / Lọt trong, số bản lề, số cánh (tự theo rộng tối đa); **Shift** mở cài đặt dày cánh/khe/rộng tối đa.
Bố cục là cây khoang (`V(...)`/`H(...)`/`L[...]`, `core/layout.cpp`); mỗi thao tác sửa cây rồi lõi C++ dựng lại (một Undo). Khoang lấy từ `openings` của lõi.

### Tủ bếp (loại tủ, cụm)
- `cab_type`: `base` (không nóc, thanh giằng `rail_w`/`rail_count`), `wall` (treo cao `hang_z`), `tall`. Chân: `side_on_plinth`, `plinth_front`, `plinth_setback`. Ray âm: `drawer_slide=under`, `slide_len`. Tấm ốp hông: `end_panel`.
- Khóa khoang thêm: `f=` tấm fix dưới khoang, `x=` ống hút mùi, `r=` bản lề cánh đơn (1 trái, 2 phải).
- **Cụm** (`ruby/.../cluster.rb`): nhiều khối xếp hàng (tủ dưới/treo/cao), kéo giãn chung bằng Scale gốc (khối `fixed` giữ rộng, khối còn lại chia lại), phào/thanh che chạy suốt hàng, bảng cắt gộp. Mẫu "Cụm bếp mẫu" trong Thư viện. Test: `ruby -Ku tests/cluster_test.rb`.

## Lớp SketchUp (Ruby) — `ruby/`
Cài: chép `ruby/aic_design_dc.rb` + thư mục `ruby/aic_design_dc/` vào thư mục Plugins; đặt `hsx_core.dll` (Win) / `libhsx_core.dylib` (Mac) vào `aic_design_dc/`.

- **Vẽ tủ** (`draw_tool.rb`): click điểm 1, click điểm 2 (hoặc gõ `rộng,sâu` + Enter), có xem trước khung bao.
- **Thông số** (`dialog.rb`, `dialog.html`): chỉnh khung/khoang, áp dụng trực tiếp vào tủ đang chọn; bảng cắt; xuất DXF.
- **Kéo giãn như bản gốc** (`observers.rb`): dùng công cụ Scale gốc của SketchUp; khi thả chuột, tủ được đưa về scale 1 và dựng lại từ lõi C++
  (ván giữ độ dày, ngăn kéo giữ chiều cao, khoang cố định co theo tỉ lệ). Hoàn tác một lần (Ctrl+Z) trả về cả bước scale.
- **Thư viện mẫu** (`library.rb`): 7 mẫu tủ + 6 chi tiết có sẵn; lưu/nạp/xóa mẫu riêng tại `~/.aic_design_dc/library.json`.
- **Chi tiết** (`part_tool.rb`): chọn loại trong hộp thoại, click đặt; cạnh cong được làm mịn (soft/smooth) trong SketchUp.
- Icon toolbar dạng SVG (SketchUp 2016+).
- **Cam chốt & dán cạnh** (`cam=1`, `band=1` mặc định): lỗ khoan lưu trong thuộc tính từng tấm (`hsx/holes`); bảng cắt hiện kích thước cắt + chỉ dán; DXF có lỗ khoan mặt.

### Không làm đơ SketchUp khi gom chi tiết
Gom chi tiết toàn model + xếp ván (`nest_session.rb`) chia làm 3 phần để luồng giao diện không bị chiếm:
1. **Quét theo lát ~6 ms** bằng `UI.start_timer` (nghỉ 10 ms giữa các lát để UI vẽ lại); chỉ đọc thuộc tính tham số, không tạo hình, bỏ qua đối tượng ẩn.
2. **Tính nặng ở luồng nền C++** (`hsx_nest_start` trả về ngay ~0,04 ms; giải từng tủ, gom chi tiết, MaxRects đều ở luồng riêng).
3. Ruby chỉ **thăm dò** `hsx_nest_status` mỗi 120 ms (đọc nguyên tử, có thanh tiến độ + nút Hủy) và lấy kết quả khi xong; toạ độ từng tấm lấy theo yêu cầu (`hsx_nest_sheet`).
Đã kiểm chứng: ThreadSanitizer + AddressSanitizer + UBSan sạch; 30.000 chi tiết xử lý ~134 ms ở luồng nền.

Đã kiểm tra: cú pháp Ruby 2.7+, logic `Params`/thư viện mẫu/co giãn khoang/gọi lõi qua Fiddle, luồng quét + luồng nền bằng giả lập API SketchUp. Chưa chạy trong SketchUp thật.
