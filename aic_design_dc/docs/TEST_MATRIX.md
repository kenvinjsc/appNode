# Test matrix (thật = đã có test tự động chạy được; chưa = kế hoạch)

Fixture chuẩn §10: tủ 1600×560×2200, ván 17.5, hậu 9.5, chân 60; 2 cột, đợt ngang, 2 ngăn kéo trái, 3 ngăn kéo phải, cánh + bản lề.
Layout fixture: `V(0:H(0:L[l=2],0:L[d=2]),0:L[d=3,l=1])` (sẽ chốt ở bước 3).

| # | Kịch bản | Test | Trạng thái |
|---|---|---|---|
| 1 | Rộng 1600→1850→1600 | `tests/test_core.cpp` (co giãn video) | CÓ (không drift JSON) — cần thêm so khớp ID |
| 2 | Cao 2200→1950 | chưa riêng | chưa |
| 3 | Sâu 560→450 | chưa riêng | chưa |
| 4 | Cột trái fixed, phải fill | `layout` size>0 / 0 | có (lõi) |
| 5 | Module neo góc dưới phải | — | chưa (cần anchor) |
| 6 | Chuyển kiểu hậu | Phủ/Âm có; Lọt chưa | một phần |
| 7 | Chuyển kiểu cánh/bản lề | test `video()` 3 kiểu cánh | có |
| 8 | Phụ kiện trước/sau resize | joinery test (đếm) | một phần |
| 9 | Lưu–đóng–mở–sửa tiếp | thuộc tính JSON; `cluster_test` roundtrip spec | một phần; SketchUp thật chưa |
| 10 | Undo/Redo nhiều lần | `start/commit/abort_operation`; mock | SketchUp thật: bên test |
| 11 | Biến dạng, chồng lấn, số đo BOM | overlap + fuzz 400 ca; cutlist | có (không so BOM–scene) |
| 12 | Tủ quay/nested | — | chưa |
| V | `validateCabinet` | — | chưa (bước 2) |
