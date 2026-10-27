# TODO cho Claude trên cloud (dev tiếp AicDesignDC)

Cập nhật: 2026-10-27. Trạng thái dựa trên commit `92b10b3`. Đọc `CLAUDE.md` và `README.md` trước.

## 1. Đã kiểm tra thật / chưa
- ĐÃ chạy (Windows, MSVC, Release): `test_core.exe` → `OK: 30 panels, 15 hinges, 12 cutlist rows`; `ruby -c` sạch.
- ĐÃ chạy trong SketchUp 2025 thật (bản d5c9389, trước commit `92b10b3`): extension nạp được, `hsx_core.dll` nạp được, `Core.solve` trả 29 tấm.
- CHƯA chạy trong SketchUp: bản `92b10b3` (cây khoang, công cụ Đợt/Vách/Ngăn kéo/Cánh, option bar, nest nền). Chưa dựng thử hình thật.
- CHƯA dựng lại được tủ bếp mẫu (model `Tu-bep-01`) bằng plugin. Đây là việc ưu tiên số 1 (mục 3).

## 2. Lỗi môi trường đã sửa ở test (Windows)
`tests/test_core.cpp` ghi file vào `/tmp/...` (không có trên Windows) → FAIL. Đã đổi sang `std::filesystem::temp_directory_path()`
(5 chỗ; riêng `hsx_nest_export_dxf` cần `.string().c_str()`). Assertion giữ nguyên. Nếu cloud (Linux) merge, vẫn chạy được.

## 3. Việc còn thiếu, theo thứ tự ưu tiên

### 3.1 Tủ bếp nhiều module (từ video "kéo FredoScale không sai")
Video mẫu cho thấy tủ bếp gồm 3 khối: tủ cao (có cánh trên, 2 thanh ngang), tủ treo 3 khoang, tủ dưới 4 khoang có thanh giằng trên. Người dùng kéo giãn cả cụm mà ván giữ độ dày.
- Plugin hiện chỉ có MỘT tủ (một `Params`/cây khoang). Cần: nhóm nhiều module tủ (cao / treo / dưới) trong một cụm, co giãn chung, mỗi module dựng lại từ lõi.
- Cần kiểu tủ bếp: tủ dưới KHÔNG nóc, thay bằng 2 thanh giằng trên (trước/sau); tủ treo; tủ cao có hộc trên. Chưa có mẫu trong `library.rb`.
- Kiểm tra: kéo giãn bằng Scale gốc (`observers.rb`) trên cụm nhiều module phải ra đúng kích thước, ván giữ độ dày. Chưa thử.
- Việc cụ thể: thêm `Params` cho dạng tủ (base/wall/tall), mẫu thư viện "Tủ bếp", test lõi (không chồng lấn, số tấm đúng), rồi thử trong SketchUp.

### 3.2 Đối chiếu với plugin tham chiếu "Hùng Sản xuất" (video 1:42)
Quan sát từ khung hình (KHÔNG có âm thanh/phụ đề → phần giải thích nút là SUY ĐOÁN, cần người dùng xác nhận):
- Thanh công cụ ~16 nút: logo H, chọn, mắt (ẩn/hiện), bảng "Thông số", căn/chia, đổ/phủ, bo góc, ... , cài đặt (bánh răng), xuất DXF.
- Thanh tuỳ chọn trong lúc dùng công cụ: `Giới hạn: tắt | Tẩy | Xóa cánh | Phủ`.
- Thanh trạng thái hướng dẫn phím: `gõ 1..9: số cột mỗi khoang | ALT: DỌC | Click: chia khe GIỮA(2) | TAB: đổi khe sắp chia (xà 25)`; `SHIFT: Cài đặt (dày cánh, khe, rộng tối đa 1 cánh)`.
- Hộp thoại "Thông số" riêng.
- Cánh hiển thị đường chéo nét đứt (ký hiệu chiều mở).
- Ván bo tròn góc (cạnh dày bo) như ở phút 1:04.
So với plugin mình (đã có): chia khoang bằng click, gõ số ván, Alt đổi đợt/vách, Shift mở cài đặt, hộp Thông số, bo góc ván gia công, DXF.
Còn thiếu / chưa rõ:
- [ ] "Tẩy" (xoá ván/ngăn vừa thêm trong khoang) và "Xóa cánh" — kiểm tra `opening_tools.rb` đã có chưa.
- [ ] "Giới hạn" (giới hạn vùng áp dụng?) — chưa rõ ý nghĩa, hỏi người dùng.
- [ ] "Xà" khi chia khe (TAB đổi khe sắp chia, mặc định 25) — chia khe/xà ở mức khoang.
- [ ] Ký hiệu chiều mở cánh (đường chéo nét đứt) trên mặt cánh.
- [ ] Nút ẩn/hiện (mắt) nhóm chi tiết.
- [ ] Thanh toolbar đủ nút như bản gốc; icon hiện có chỉ: `draw, params, part, divide, door, drawer`.

### 3.3 Mục "Chưa làm" đã có trong README
Bo góc/cong cho thân tủ liền khối; khấu ván khi các tấm giao cắt (cần boolean); ghép hậu theo khổ 1830.
Giả định cần xác nhận: hồi lên trước 19; cam cách đầu 34 / đầu mối nối 37 / bước ≤320; chốt gỗ Ø8 giữa hai cam.

### 3.4 Hạ tầng
- [ ] CI Windows (MSVC) chạy `test_core` — bắt lỗi kiểu `/tmp` sớm.
- [ ] Test tự động nạp thư viện qua Fiddle trên bản đã cài (script trong CLAUDE.md đã đủ, thêm vào CI).
- [ ] Ngoài lề: ruby `Fiddle` trên SketchUp 2025 (Ruby 3.2) đã chạy được; SketchUp ≤2022 (Ruby 2.7) chưa thử trên máy thật.

## 4. Cách thử nhanh trên máy người dùng (Windows)
1. `cmake -S . -B build` → `cmake --build build --config Release` → `build\Release\test_core.exe`.
2. Chép `ruby\aic_design_dc.rb` + `ruby\aic_design_dc\` + `build\Release\hsx_core.dll` (vào `aic_design_dc\`) vào
   `%APPDATA%\SketchUp\SketchUp 2025\SketchUp\Plugins\`.
3. Trong SketchUp (MCP `supex-win`, cổng 9876): `load` file `aic_design_dc.rb` để nạp không cần khởi động lại.
4. Dựng thử vào model MỚI hoặc nhóm riêng; không sửa model thật của người dùng khi chưa được phép.

## 5. Lưu ý cho agent
- Không đổi tên `hsx_core` hay khóa `parseParams` (xem `CLAUDE.md`).
- Mọi thay đổi lõi C++ kèm test trong `tests/test_core.cpp`; báo rõ cái gì đã chạy thật, cái gì chưa.

## 6. Dựng thử tủ bếp "Tu-bep-01" bằng plugin (2026-10-27, SketchUp 2025 thật)
Đã làm: đọc model (120 nhóm ván của plugin khác, tên kiểu `[2705] THANH GIẰNG`), dựng 10 khối bằng `Builder.build` (5 tủ dưới + 5 tủ treo) vào nhóm "AicDC dựng lại tủ bếp (thử)". Kết quả: ra **50 tấm (chỉ khung rỗng, 5 tấm/khối)**, bố cục và kích thước khung đúng, KHÔNG có cánh/ngăn kéo/thanh giằng.

### Kết cấu tủ bếp gốc (mm, toạ độ model: X rộng, Y sâu, Z cao; tổng 3000 × 591 × 2425, x 1449..4449)
- **Tủ dưới** (5 khối, rộng 400 / 900 / 450 / 800 / 400): sâu 561, đáy dày 18 ở z=75 (trên chân 75), hồi 17–18 cao ~779, hậu 9 ở y=2244, **không có nóc**: thay bằng THANH GIẰNG 70×18 (trước, sau, hai bên) ở z≈796–866 (+ "ĐỈNH" 9 mm ở z861–866 rộng bằng khối). Chân trước (CHÂN TRƯỚC) liền 1282 × 17 × 75.
- **Tủ treo** (5 khối, rộng 400 / 900 / 550 / 600 / 500): đáy ở z=1495, cao 880 (khối 600: đáy z=1677–1692, cao 673), sâu 331 (khối trái 350), có ĐỢT DI ĐỘNG, nóc "ĐỈNH" 18 ở z≈2347–2357, PHÀO 50×18 chạy suốt 1450 trên 2 khối (x1849..3299, z2365).
- **Cánh**: CÁNH TRÁI/PHẢI A 400–448 × 785 (z45..830) cho tủ dưới; 448 × 880 cho tủ treo. 6 cánh trái + 6 cánh phải.
- **Ngăn kéo** (3 ngăn ở khối 450): MẶT NGĂN KÉO, ĐÁY NGĂN KÉO, TNK TRÁI/PHẢI/SAU (thành ngăn kéo), **RAY ÂM 450** (ray giấu).
- Khác: TẤM DỌC ×3, TẤM FIX ×2, "HT: 30"/"HT: 20" ×2 (chưa rõ nghĩa, hỏi người dùng).

### Lỗ hổng plugin phát hiện khi dựng (cần Claude Code sửa)
1. [ ] `Builder.build(model, {cols: [[0,drawers,0,0,doors]]})` KHÔNG tạo cánh/ngăn kéo vì `DEFAULTS[:layout]='V(0:L[])'` (khung rỗng) đè lên `cols`. Cần: hoặc `cols` có hiệu lực khi `layout` rỗng/không truyền, hoặc API Ruby thêm cánh/ngăn kéo theo khoang không cần click chuột (để script/test dựng được).
2. [ ] Loại tủ: thiếu `kind = base|wall|tall`. Tủ dưới phải **bỏ nóc, thêm 2–4 thanh giằng** (kích thước/vị trí tham số hoá); tủ treo cần tham số độ cao treo (z đáy 1495/1692) thay vì phải tự dịch bằng Transformation.
3. [ ] Chân: `plinth` chỉ 60 mỗi khối; bếp dùng chân 75 liền nhiều khối (CHÂN TRƯỚC).
4. [ ] Ngăn kéo **ray âm 450** (TNK trái/phải/sau, đáy, mặt); hiện chỉ có ray 12.7.
5. [ ] Cụm nhiều khối: nhóm + co giãn chung + phào chạy suốt qua nhiều khối (hiện phào chỉ là chi tiết lẻ).
6. [ ] Chưa kiểm tra: bấm chuột bằng DivideTool/DoorTool/DrawerTool (chỉ test qua API), dialog/toolbar mở bằng menu.
7. [ ] Quy ước trục: lõi dùng Y=sâu, Z=cao (đúng). Lưu ý `Sketchup::BoundingBox#height`=Y, `#depth`=Z khi viết test/đo.
Việc dựng thử có thể chạy lại: 5 tủ dưới `{width:W, depth:561, height:866, plinth:75}`, 5 tủ treo `{depth:331, height:880, plinth:0}`, đặt bằng `Geom::Transformation.translation`.
