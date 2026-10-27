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
