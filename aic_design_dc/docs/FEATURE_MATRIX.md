# Feature matrix (Phase 0) — trạng thái THẬT của mã tại commit khi viết

Ký hiệu: REUSE = đã có dùng được | EXTEND = có nhưng cần mở rộng | MISSING = chưa có.
"Test" = có test tự động trong repo. Chưa có mục nào được kiểm trong SketchUp thật cho bản hiện tại, trừ những mục bên test (Windows, SketchUp 2025) đã ghi trong `TODO_cloud.md` mục 6–9.

| # | Chức năng (master prompt) | Trạng thái | Nơi trong mã | Test | Khoảng trống |
|---|---|---|---|---|---|
| F01/P1 | Khung tủ tham số (W/D/H, dày ván/hậu, chân, hậu) | REUSE | `Params`, `Builder::run` | `test_core` | thiếu `len nóc` (top trim); hậu thiếu kiểu **Lọt** |
| P1 | Stable ID cho từng tấm | MISSING | tấm chỉ có `name` | — | cần `id` ổn định qua resize/lưu-mở |
| P1 | Vai trò tấm | EXTEND | `role`: carcass/back/front/drawer/plinth | có | chưa tách Left/Right/Top/Bottom/Divider/Shelf thành enum |
| F02/P3 | Chia khoang tương tác (đợt/vách, preview, nhập mm, chia đều, lồng nhau) | REUSE | `opening_tools.rb`, `layout.rb`, `layout.cpp` | fuzz 400 ca (lõi) | **chưa thử chuột thật trọn vẹn**; chưa kéo tay nắm; chưa khóa/chọn khoang co giãn trong UI |
| F03/P2 | Resize không dùng Scale đồng nhất | REUSE | `Builder.rebuild`, `observers.rb` | test lõi co giãn | chưa có kịch bản §10 đầy đủ (xem TEST_MATRIX) |
| P2 | Sizing modes fixed/ratio/auto/fill/min-max | EXTEND | `size>0` = fixed, `0` = fill/auto; tất cả fixed → co tỉ lệ | có | thiếu ratio, min-max tường minh |
| P2 | Constraint infeasible → diagnostic + rollback | EXTEND | `distribute` báo lỗi, `rebuild` abort | có | diagnostic chưa có mã rule/part ID |
| P2 | Neo module (bottom-right…) | MISSING | ngăn kéo luôn từ đáy khoang | — | cần `anchor` trong lá |
| F04/P4 | Ngăn kéo thông minh | EXTEND | `leaf()` trong `cabinet.cpp`, ray bi/ray âm | có | khoảng hở ray cứng trong mã (12.7/21), chưa là preset xác thực |
| F05/P4 | Cánh + bản lề (trùm ngoài/nửa/lọt trong) | REUSE | `leaf()`, `hingesFor` | có | quy tắc khoan chén cứng (Ø35, 22.5…), chưa là preset; chưa kiểm hướng mở/ký hiệu |
| F06/P5 | Cam/chốt tự dò, quan hệ chủ–khách | EXTEND | `joinery.cpp` (dò T-joint theo hình học) | có | chưa có đồ thị quan hệ (host/mate ID), chưa đảo mặt/di chuyển/xóa, chưa kiểm xung đột lỗ |
| F07/P6 | Hậu Phủ/Âm/Lọt + hồi lên trước | EXTEND | `BackStyle` Overlay/Inset | có | thiếu **Lọt**; kiểm ray chạm hậu chưa tường minh |
| F08/P7 | Thư viện mẫu | REUSE | `library.rb` (khung rỗng, mẫu đủ, cụm, chi tiết) | cluster_test | chưa tìm kiếm/thumbnail; chưa cập nhật phiên bản mẫu |
| F09/P8 | `validateCabinet` (errors/warnings/rule ID/part ID/đề xuất) | MISSING | chỉ lỗi từ `solve` | — | cần module kiểm tra |
| F10/P9 | Toolbar/inspector/ESC-Enter/Undo | EXTEND | `main.rb`, `dialog.*`, tools | giả lập | Enter/ESC xác nhận-hủy theo preview chưa đầy đủ; chưa nút Kiểm tra/Bản lề/Cam riêng |
| F11 | BOM/DXF/CNC | REUSE | `cutlist`, `exportDxf`, `nest` | có | chưa so BOM–scene sau resize có kiểm tự động |
| F12 | Lưu/mở + `schema_version` + migration | EXTEND | thuộc tính `hsx/params` | cluster_test | chưa có `schema_version`/migration |
| F13 | Hiệu năng, an toàn | REUSE | solve ~0.3 ms; nest nền | perf test | chưa chống reentrant/stale revision |
| F14 | Hướng dẫn tiếng Việt | EXTEND | README | — | `USER_GUIDE_VI.md` chưa có |
| P1 | Tủ quay/mirror/nested | EXTEND | tính trong toạ độ cục bộ | — | chưa test tủ quay 90°, mirror, nested scale |

## Phân loại rủi ro lớn
1. **ID ổn định** chưa có → mọi liên kết (phụ kiện, BOM diff) phụ thuộc tên.
2. **Phụ kiện từ dò hình học** → đổi cấu trúc có thể đổi cả cam; chưa có quan hệ tường minh.
3. **Thông số phần cứng cứng trong mã** → phải chuyển sang preset cấu hình (không bịa số hãng).
4. **Chưa kiểm SketchUp thật** cho nhiều đường chuột.
