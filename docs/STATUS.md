# Trạng thái & việc tiếp theo

Hướng dẫn thao tác dựng tủ từng bước: [HUONG-DAN-DUNG-TU.md](HUONG-DAN-DUNG-TU.md). Kịch bản test: [KICH-BAN-TEST.md](KICH-BAN-TEST.md) (`cd app && npm run e2e`).

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

### Đợt P0 xưởng (theo `DE-XUAT-TINH-NANG-V2.md`)
| Mã | Nội dung | Ghi chú |
|---|---|---|
| D01 | Chuẩn xưởng `Cabinet.rules.shop` (kệ, chốt, bản lề, tay nắm, ngăn kéo, liên kết), lưu / áp chuẩn (mẫu nhóm `all`) | mặc định = giá trị cũ |
| D02 | Hàng lỗ chốt hệ 32, kệ di động bắt lỗ | hình học chỉ vẽ vòng tròn khi > 16 lỗ |
| D04/D05 | Loại tay nắm (thanh/núm/push-open/không), khoan lỗ tay nắm, đế bản lề trên hồi; ray bi / âm / tandem | chưa: catalog phụ kiện riêng (ngăn kéo trong, mặt giả: đã có) |
| D06 | Liên kết chốt gỗ / cam + chốt / vít / ke, lỗ thật theo tủ; báo giá đếm theo lỗ | khoan cạnh vẫn là cảnh báo CNC |
| D07 | Tủ góc L mù (trái / phải) | chưa: góc chéo, mâm xoay |
| D08 | Chân tủ: len trước / 3 mặt, chân nhựa (+ len kẹp), tủ treo | |
| D09 | Dãy tủ: mặt đá, len chân liền, tấm lấp, che trần; tự sinh lại khi tủ đổi | chưa khoét chậu / bếp trên mặt đá |
| D10 | Dán cạnh theo nhóm tấm, chỉ ABS/PVC, hậu không dán theo vai trò | chưa: bộ vật liệu, loại vật liệu VN |
| D11 | Mã tấm, danh sách cắt gộp, in nhãn 60×40, CSV | chưa: QR, profile CSV cấu hình |
| D12 | Báo giá mét dài / m² mặt đứng / bóc chi tiết theo phòng, hao hụt, công, lợi nhuận, VAT | chưa: xuất PDF báo giá |

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
Parametric editor (Phase 1–2): khoang LOCK/AUTO/% (`Split.bays`), neo W/H/D, bản vẽ 2D sửa được (W/H tủ, kích thước khoang,
chế độ khoang), kéo vách/kệ có xem trước + commit, ghim vùng và menu chuột phải trên 2D, Chia đều lại.
Phase 3–5: multi-edit ("Nhiều giá trị", một bước undo), offset 6 phía, inspector theo ngữ cảnh, template tủ (lưu/chèn
giải lại), rule preset, lật gương, nhân dãy tủ, nhân tấm.
Constraint động tường minh (cạnh → mặt tấm khác + offset).
Khe cánh từng phía; cao từng ngăn kéo (KHÓA/%/AUTO) + kéo đường chia ngăn trên 2D.
Quan hệ 2 tấm Phủ/Lọt/Bằng mặt/Khe, kéo 4 cạnh tấm trên 2D (giữ ràng buộc / tự do), view Trái/Phải/Mặt cắt dọc/ngang,
cung cạnh và biên dạng đa giác tự do.
Còn lại của spec Parametric: đồng bộ SketchUp (dự án `aic_object_relation`).
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
