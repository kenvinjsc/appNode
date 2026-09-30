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
| D07 | Tủ góc L mù (trái / phải), tủ góc chéo (bếp dưới / trên, tấm 5 cạnh, cánh 45°) | chưa: mâm xoay, tay nắm cánh chéo |
| D08 | Chân tủ: len trước / 3 mặt, chân nhựa (+ len kẹp), tủ treo | |
| D09 | Dãy tủ: mặt đá, len chân liền, tấm lấp, che trần; tự sinh lại khi tủ đổi | chưa khoét chậu / bếp trên mặt đá |
| D10 | Dán cạnh theo nhóm tấm, chỉ ABS/PVC, hậu không dán theo vai trò | bộ vật liệu (dựng sẵn + tự lưu, áp cả phòng), vật liệu VN (MFC, MFC lõi xanh, Acrylic, Laminate, Veneer) |
| D11 | Mã tấm, danh sách cắt gộp, in nhãn 60×40, CSV | chưa: QR, profile CSV cấu hình |
| D24 | Tool 03 / 08 / 16 / 18 gửi ý định, core tính theo kích thước tấm (khấu góc theo góc khi tủ đổi cỡ), nhiều tấm một undo | |
| D30 | 14 mẫu dựng sẵn §7 (bếp 4, tủ áo 4, giường 2, bàn học, kệ TV treo, tủ giày, tủ lavabo) có tham số hiển thị (kịch trần, số kệ, số cánh lùa, cánh lật…), lưới Mẫu dựng sẵn theo phòng + form W/H/D | chưa: ảnh thu nhỏ; bàn đảo (D23, Đợt 3) |
| D20 | Sự kiện `ZonesChanged` theo tủ; 2D / Tạo tấm chỉ tải lại khoang của tủ bị đổi; tấm 2D vẽ bằng `React.memo` | chưa: patch khoang trong sự kiện (vẫn gọi `get_zones` cho tủ đó) |
| D25 | Request `preview` (chạy thử, không undo / revision / sự kiện); kéo vách / kệ 2D hiện số core tính | chưa: preview khi kéo cạnh tấm, kéo kích thước tủ 3D |
| D03 | Số đo sửa trực tiếp trên 2D do core tính: sâu, lùi kệ (Bên / Phải), cao chân, dày ván, khe cánh, tay nắm cách đầu cánh (Trước) | chưa: thanh treo, khe tấm lấp, tổng dãy |
| D27 | Căn trái / phải / trên / dưới / trước, chia đều, xoay ±90°, sát tường (tab Chỉnh sửa) | chưa: phím tắt riêng |
| D19 | Khung sản phẩm (`rules.product`, `create_furniture`) + giường: đầu (phẳng/bọc/nan), vai, đuôi, dát nan/tấm, đà giữa, 4/6 chân, hộc kéo 2 bên/đuôi, nâng hơi; báo giá theo chiếc | chưa: sửa lọt nệm trực tiếp trên 2D Trên |
| D21 | Bàn học / làm việc / trang điểm: mặt bàn + khoét dây, chân tấm / hộc tủ (dùng lại ngăn kéo) / chân sắt, yếm, hộc bàn phím, kệ trên, gương | chưa: sửa vị trí lỗ dây trực tiếp trên 2D |
| D22 | Vách ốp / vách TV / lam: lưới tấm theo công thức cột × hàng, khe bóng / soi V, khung xương, lam dọc/ngang, khoét hộp điện; kệ TV treo = tủ thấp + chân Treo | chưa: kéo đường chia mô-đun trên 2D |
| D16 | Cánh lùa theo hệ ray: 2/3 ray, chồng, trừ cao, khung nhôm 20/45 + nẹp ngang, ô nhét ván/kính/gương; profile (m) + kính (m²) vào báo giá | chưa: catalog hệ ray theo hãng; kính hiển thị trong suốt |
| D17 | Cánh lật (HK/HL/ben hơi) và gập 2 lá (HF): bản lề trên, tay nâng theo khối lượng cánh, kiểm tra cao khoang (LIFT_HEIGHT); cánh mở / lật khung nhôm kính | chưa: khoan lắp tay nâng trên hồi |
| D18 | Phụ kiện khoang theo catalog (giá bát, rổ gia vị, khay thìa, giá kéo, giá giày / quần, LED): kiểm tra vừa khoang (cảnh báo + tô đỏ 2D, không chặn resize), ray + báo giá theo mã, LED theo mét + nguồn | chưa: khoan lắp giá kéo trên hồi |
| D34 | Khoang thiết bị (lò 600, vi sóng 380, tủ lạnh âm, máy rửa bát, máy giặt): kiểm tra lọt lòng khi thêm (APPLIANCE_FIT), thanh đỡ, khe thoát nhiệt tự khoét hậu; mẫu tủ lò dùng khoang lò. Sửa `parse_split_formula` nhận `*` đứng riêng | chưa: tấm ốp tủ lạnh |
| D29 | Bản vẽ in A3/A4 có khung tên: core chiếu mặt đứng theo hướng tường, mặt bằng, chi tiết tủ (trước ẩn cánh + bên), chuỗi kích thước, nét mở cánh, tỷ lệ chuẩn tự chọn, xếp trang; UI in SVG | chưa: mặt cắt trong bản vẽ in, mẫu khung tên tuỳ chỉnh |
| D23 | Bàn đảo / quầy bar: tủ mở 2 mặt (vách giữa theo chiều sâu, khoang sau dựng rồi xoay 180° → cánh / ngăn kéo quay ra sau), không hậu, mặt đá nhô phía ghế, tab Bàn đảo, mẫu Bàn đảo 1800 | chưa: 2D view Mặt sau để ghim khoang sau |
| D26 | Xuất file máy theo tấm: DXF theo layer (cắt, khoan mặt A / B, khoan cạnh, hốc, rãnh), MPR (Homag), CIX (Biesse); lật mặt B, gốc trên / dưới | chưa: xuất theo sheet nesting; kiểm tra với máy thật |
| D13 | Phào nóc 1–3 mặt + phào chân, ốp hông (chạm sàn / nhô trước), nẹp che khe; tab "Phào & ốp" + lưu mẫu tab | chưa: phào theo cả dãy tủ (D09), vát 45° thành contour CNC |
| D15 | Khoét hậu (ổ điện / ống / thoát nhiệt, neo theo lòng tủ, CNC cắt trong), hậu ốp bắt vít (+ vít vào báo giá), hậu chia theo kệ cố định | chưa: vẽ / kéo lỗ trên 2D |
| D14 | Kệ nghiêng thật (xoay quanh trục X, dài theo cos θ, thanh chặn gót khi ≥ 5°, bỏ chốt kệ), vách lửng (`extent`), menu "Kệ giày nghiêng 15° × 4" | chưa: kệ góc L trong tủ góc mù |
| D12 | Báo giá mét dài / m² mặt đứng / bóc chi tiết theo phòng, hao hụt, công, lợi nhuận, VAT | in báo giá A4 / PDF có tổng bằng chữ |

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
