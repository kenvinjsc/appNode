# Kế hoạch triển khai (theo master prompt, mỗi bước 1 commit, báo PASS/FAIL thật)

| Bước | Nội dung | Tiêu chí nghiệm thu |
|---|---|---|
| 0 | Khảo sát + 4 tài liệu (file này) | Có trong `docs/` |
| 1 | Lõi: kiểu hậu **Lọt**; **ID ổn định** cho tấm; `schema_version` + migration Ruby | test: ID duy nhất, không đổi sau resize thuần; 3 kiểu hậu không chồng tấm |
| 2 | `validateCabinet` (C++ `core/validate.cpp`, JSON `diagnostics`) | lỗi có rule ID, part ID, đề xuất; fuzz không báo sai |
| 3 | Resize engine: sizing ratio/min-max; neo module; diagnostic rõ khi infeasible | kịch bản §10 #1–#5 pass |
| 4 | Ngăn kéo/cánh: preset phần cứng cấu hình (ray, bản lề) thay vì hằng số cứng | số lấy từ preset; đổi preset đổi kết quả |
| 5 | Đồ thị quan hệ phụ kiện (host/mate ID), đảo mặt/di chuyển/xóa cam | sau resize vị trí tính lại từ host; không trùng lỗ |
| 6 | UI: nút Kiểm tra, Enter/ESC, khóa khoang co giãn, hiển thị diagnostic | thử trong SketchUp (bên test) |
| 7 | Test §10 đầy đủ + golden fixtures + `USER_GUIDE_VI.md` + `MIGRATION_NOTES.md` | báo cáo cuối |

Quy tắc: không đánh PASS nếu chỉ đọc mã; mục chạy trong SketchUp thật chỉ bên test (Windows, SketchUp 2025) xác nhận.
