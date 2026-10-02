# Checklist kiểm thử thủ công (UI)

Thiết lập: `cargo run -p aic-dev-server --release` và `cd app && npm run dev`.

## Phase 1: khung app, cây, viewport, thuộc tính, chọn, camera
- [ ] Mở app lần đầu thì dự án mẫu tự tạo; cây hiển thị Room 01 và 6 tủ
- [ ] Click chi tiết: lần đầu chọn tủ, lần hai chọn chi tiết; Ctrl+click thêm/bớt; Shift+kéo quét chọn
- [ ] Hover: viền cam nhạt; chọn: viền cam; khóa: mờ; ẩn: không hiển thị
- [ ] Chế độ chọn Mặt/Cạnh (tab Chỉnh sửa) tô sáng đúng mặt/cạnh
- [ ] Phím 1–5, 0, F, Shift+F; chuyển Phối cảnh/Trực giao
## Phase 2: tạo tủ, sửa tham số, move/rotate, undo/redo
- [ ] Tủ ▾ → Tủ áo → click sàn → sửa W/H/D → Tạo tủ
- [ ] Rộng = 800 thì đợt, nóc, đáy, cánh cập nhật ngay; nhập 20 thì có thông báo “Chiều rộng không thể nhỏ hơn tổng chiều dày hai hồi.”
- [ ] Ô của tấm nhập `= cabinet.inner_width - 100` thì hiện ƒx; sửa rộng tủ thì tấm tự cập nhật; `= width + 1` báo lỗi vòng phụ thuộc
- [ ] Số đợt 4 → 2 → Ctrl+Z khôi phục đúng 4 đợt, id không đổi
- [ ] M: kéo trục X thì có preview, hint bắt dính, thả chuột thì lưu; Ctrl+Z trả về vị trí cũ
## Phase 3: snap, kích thước, quan hệ
- [ ] Kéo tủ sát tủ khác thì bắt dính mặt (đường cam), không thì bắt lưới
- [ ] Chọn 1 tủ có 3 kích thước; chọn 2 đối tượng có khoảng cách giữa chúng
- [ ] Shift+R: vùng tiếp xúc xanh/lam/đỏ; tooltip đúng; lọc ở status bar
## Phase 4: Gia công
- [ ] Hồi tủ: chốt gỗ, khoan cạnh, chốt đợt; cánh: 2–3 lỗ khoét bản lề Ø35 ở mặt B
- [ ] Đổi Mặt A/B thì bản vẽ lật; hover dòng trong bảng thì feature tô sáng
## Phase 5: Xếp tấm
- [ ] Chạy xếp tấm: không chồng lấn, có số tấm và % hao hụt; vật liệu có vân thì chi tiết không xoay sai vân
## Phase 6: CNC
- [ ] Đường chạy dao theo màu dao; Play/Step/tốc độ; dòng G-code hiện hành được tô; Xuất G-code tải về `.nc`
## Khác
- [ ] Lưu (Ctrl+S) → Mới → Mở lại tệp → dự án giống hệt
- [ ] Menu chuột phải: đổi tên, sao chép, ẩn, khóa, xóa, chọn cha/con, quan hệ, gia công
- [ ] Tắt dev server → thông báo “Không kết nối được lõi CAD.”
