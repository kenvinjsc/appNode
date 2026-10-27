# Migration notes

## Schema dữ liệu tủ trong SketchUp
Thuộc tính nhóm `hsx/schema` (số nguyên):
- *(thiếu)* hoặc `1`: tủ dựng bằng `cols` + `loft_*`, chưa có `layout`. `Params.migrate` đặt `layout=''` để lõi dùng `cols/loft` (không biến thành khung rỗng).
- `2` (hiện tại): có `layout` (cây khoang), loại tủ (`cab_type`…), ID tấm (`hsx/id` trên từng tấm).
Mỗi lần `Builder.build/rebuild` ghi `hsx/schema = 2`. Không xóa thuộc tính cũ.

## ID tấm
`id` = tên cấu trúc (khoang + vai trò), trùng tên thì `#n`. Ổn định khi đổi rộng/sâu/cao; đổi cấu trúc (thêm/bớt vách, ngăn kéo) có thể tạo/mất ID tương ứng.
Tủ dựng trước phiên bản này chưa có `hsx/id` trên tấm; tự có sau lần sửa tiếp theo (dựng lại).

## Kiểu hậu
`back_style`: `overlay` (Phủ), `inset` (Âm, lùi 8 mm), `flush` (Lọt: hậu giữa hai hông, sát mép sau, hông giữ sâu D). Giá trị cũ không đổi nghĩa.

## Rollback
Mỗi sửa là một `start_operation`; lỗi giải thì `abort_operation` (tủ cũ còn nguyên). Muốn lùi bản plugin: tủ cũ vẫn đọc được vì khóa cũ không đổi.
