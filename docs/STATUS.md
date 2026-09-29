# Trạng thái & việc tiếp theo

Hướng dẫn thao tác dựng tủ từng bước: [HUONG-DAN-DUNG-TU.md](HUONG-DAN-DUNG-TU.md).

## Đã xong

### Core
| Phase | Nội dung | Ghi chú |
|---|---|---|
| 1 | math, domain, scene graph (local/world transform, dirty propagation, reparent, duplicate, delete, visibility, lock), project serialization | |
| 2 | `GeometryKernel` trait, backend `CsgKernel`, panel solid, tessellation (face id + edge id) | OCCT **chưa** tích hợp, xem ADR-0002 |
| 3 | Parametric engine (biểu thức an toàn, cycle, dirty, incremental, constraint), 6 cabinet generator | |
| 4 | Spatial (sort-and-sweep, SAT, snap), assembly graph | parry3d chưa dùng, xem ADR-0003 |
| 5 | Feature gia công (lưu trong tọa độ local), feature suy diễn, flatten 3D→2D, Clipper2 | |
| 6 | Nesting interface + MaxRects, toolpath + G-code | |
| 7 | Rayon cho narrow phase và tessellation, cache mesh theo geometry key | **Chưa** có benchmark 1000+ đối tượng |

### UI
Phase 1–6: app shell, scene tree, viewport Three.js, properties, selection (object/face/edge, multi, box), camera,
tạo tủ (đặt bằng click + nhập W/H/D), sửa tham số (có công thức), move/rotate gizmo với preview + snap từ core,
undo/redo, kích thước, handle kéo W/H/D, hiển thị quan hệ, workspace Gia công (bản vẽ trải phẳng mặt A/B),
Xếp tấm, CNC (đường chạy dao, mô phỏng, xuất G-code), thư viện vật liệu, báo cáo bóc chi tiết (CSV),
phím tắt cấu hình được, context menu, thông báo lỗi tiếng Việt, tác vụ nặng có trạng thái và nút hủy.

Thiết kế tủ kiểu plugin: dự án → tầng → phòng (tab lọc cây/3D, tạo tủ đúng chỗ); Khung (thông tin tủ, luật liên kết,
luật dán cạnh, TAB tạo dãy tủ); Tạo tấm theo vùng (ghim vùng, 6 loại tấm, khóa Tỷ lệ/Cách A/Cách B, chuột phải
"Dựng nhanh"); Chỉnh tấm (khóa chấm đỏ, co giãn); cột Tool (01–04, 06, 08–14, 16–20: bo/vát góc, cắt tự do, cắt theo tấm, hợp tấm, ghép bề dày, chia tấm); Costing Report / Danh sách cắt /
Cabinet List có đơn giá sửa được.
Chưa làm: tool 05, 07, 15; chân đế dạng thanh; căn/phân bố nhiều đối tượng.

## Chưa làm / hạn chế đã biết

1. **Backend OpenCASCADE**: trait đã sẵn, feature `occt` trong `aic-geometry` còn trống. Việc tiếp theo là viết
   `OcctKernel` bằng `opencascade-rs` hoặc `cadrum`, chạy cùng bộ test với `CsgKernel`.
2. **STEP/BREP import/export**: chưa có, cần OCCT.
3. **SQLite/sqlx cho catalog**: vật liệu hiện là danh sách mặc định trong `aic-domain::material` và được lưu trong file dự án.
4. **Snap**: core mới snap theo AABB (mặt, căn, tâm, lưới). Snap theo vertex/edge/midpoint/parallel chưa có.
5. **Chọn cạnh**: dựa trên edge của tessellation (hộp + đường gia công), chưa phải topo BREP.
6. **Nesting**: MaxRects theo hình bao chữ nhật, có vân gỗ và xoay 0/90°. Chưa nest theo hình dạng tự do.
7. **CNC**: chỉ gia công mặt A trên router 3 trục. Khoan mặt B và khoan cạnh được báo là cảnh báo (cần lật tấm / máy khoan ngang).
8. **Hiệu năng**: chưa đo 1000+ đối tượng / 60 FPS. Đã có sẵn chia sẻ BufferGeometry theo geometry key,
   instancing cho phụ kiện, cập nhật tăng dần theo event, render theo yêu cầu.
9. **Vỏ desktop Tauri**: đã `cargo check` nhưng chưa build và chạy thử có giao diện.
