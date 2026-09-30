# Ánh xạ UI Action → Core Command → Response → UI Update

Mọi request là JSON `{"cmd": "...", ...}` gửi tới `Engine::dispatch_json`
(Tauri: lệnh IPC `dispatch`; trình duyệt: `POST /api`). Response luôn có dạng
`{ ok, result, events[], error{code,message,details}, revision, can_undo, can_redo }`.

| UI action | Request (`cmd`) | Core | Events | UI update |
|---|---|---|---|---|
| Ribbon Tủ → click vị trí → nhập W/H/D | `create_cabinet {kind, position, overrides}` | `Command::CreateCabinet` → generator → solve tham số | ObjectCreated, SceneTreeChanged | `get_render_objects(ids, known_keys)` → thêm mesh; làm mới cây; chọn tủ mới |
| Khung → [TAB] Tạo tủ (theo tab tầng/phòng) | `create_cabinet {kind, position?, overrides{…, top_style, bottom_style, edge_rule, back_groove}, name?, room?, floor?, after?}` | Tên tự đánh số; `after` → đặt bên phải tủ đó; không có `position` → nối dãy của (tầng, phòng), phòng/tầng mới có khu riêng | ObjectCreated, SceneTreeChanged | Cây có `floor`, `room` trên node tủ → tab Tầng/Phòng |
| Đổi tầng / phòng của tủ (đổi tên tab) | `set_parameter {id, name: "floor" \| "room", value}` | `Command::SetCabinet` (hoàn tác được) | ObjectChanged | Làm mới tab |
| Tạo tấm: ghim vùng, [TAB] Thêm / chuột phải → Dựng nhanh | `get_zones {cabinet}`, `zone_add_panels`, `zone_add_doors`, `zone_add_drawers`, `zone_add_link`, `zone_remove` | `Command::SetCabinet` → regenerate | ObjectCreated/Deleted/Changed | Vẽ vùng, cập nhật cây |
| Chỉnh tấm: co giãn, độ dày, vật liệu tấm sinh | `set_part_mod {id, patch}` | PartMod theo key ổn định | ObjectChanged | |
| Tool 04/09/10 (cắt tự do, cắt theo tấm, bo/vát góc) | `shape_tool {ids, op: CORNERS \| CUT_LINE \| CUT_BY_PANEL}` | Contour ngoài / contour trong / pocket, lưu ở tọa độ local (part mod hoặc feature) | ObjectChanged | Mesh mới (CSG), CNC chạy theo biên dạng |
| Tool 06 Hợp tấm | `merge_panels {ids}` | Kéo dài tấm đầu, xóa các tấm còn lại (cùng mặt phẳng, cùng dày) | ObjectChanged, ObjectDeleted | |
| Tool 20 Chia tấm / 14 Ghép bề dày | `set_part_mod {id, patch: {split \| thickness}}` | PartMod.split → các tấm con `key~n` | ObjectCreated/Changed | |
| 2D: bấm số kích thước khoang, nhập mm hoặc `40%` | `set_bay {cabinet, zone, index, mode?: LOCK \| AUTO \| PERCENT, value?}` | `Split.bays`; khoang AUTO / khoang kề hấp thụ; từ chối nếu khoang < 1 mm (`CONSTRAINT_VIOLATED ZONE_TOO_SMALL`) | ObjectChanged (chỉ tấm bị ảnh hưởng) | 2D + 3D cập nhật |
| 2D: kéo vách / kệ (PREVIEW cục bộ, thả chuột = COMMIT) | `move_split_panel {id, before}` | Chỉ 2 khoang kề đổi; khoang KHÓA giữ nguyên | ObjectChanged | |
| Menu vùng: Chia đều lại | `equalize_split {cabinet, zone}` | Mọi khoang AUTO | | |
| 2D: bấm W / H tủ; Chỉnh tấm: Neo rộng/cao/sâu | `set_parameter {id, name: width \| height \| depth \| anchor_w \| anchor_h \| anchor_d}` | Đổi tham số + dời gốc theo neo + dịch các tủ liền kề cùng dãy (một bước undo), không scale | TransformChanged, ObjectChanged | |
| Chọn nhiều đối tượng → Chỉnh tấm (giá trị khác nhau = "Nhiều giá trị") | `get_properties_multi {ids}`, `set_parameter_multi {ids, name, value}` | Một bước undo, lỗi một đối tượng thì hoàn tác cả nhóm | ObjectChanged | |
| Chỉnh tấm → Offset (lùi mặt trước/sau/trái/phải/trên/dưới) | `set_parameter {id, name: off_front …}` | `PartMod.offsets` (hướng tủ) → cạnh tấm theo góc xoay, hoặc dời theo chiều dày | ObjectChanged | |
| Chuột phải tủ → Lưu làm template; Cài đặt → Template | `save_template {cabinet, name}`, `delete_template`, `get_templates` | `ProjectSettings.templates`: định nghĩa logic (zone + khoang KHÓA/%/AUTO, cánh, ngăn kéo, part mod, luật, vật liệu) + tham số | SettingsChanged | |
| Khung → Template + [TAB] | `insert_template {name, width?, height?, depth?, room?, floor?, after?}` | Tạo tủ + tham số + định nghĩa, giải lại cho W/H/D mới; một bước undo; không vừa → `ZONE_TOO_SMALL`, không tạo gì | ObjectCreated | |
| Rule preset (có sẵn "AIC Wardrobe Standard", "AIC Bếp dưới", hoặc lưu từ tủ) | `save_rule_preset`, `delete_rule_preset`, `apply_rule_preset {ids, name}` | Tham số kết cấu + kiểu nóc/đáy + luật dán cạnh; một bước undo | ObjectChanged | |
| Chuột phải: Lật gương / Nhân dãy tủ / Nhân tấm | `mirror_cabinet {id}`, `array_cabinet {id, count, axis, gap}`, `array_split_panel {id, count}` | Lật zone + bản lề + mod trái/phải; nhân bản cả định nghĩa; thêm tấm cùng loại, chia đều | | |
| Chỉnh tấm → Ràng buộc: cạnh → tấm đích / mặt trong-ngoài / offset | `set_part_mod {id, patch: {add_anchor: {edge, target, face, offset}} \| {remove_anchor: i}}` | `PartMod.anchors`, giải sau các mod khác: cạnh kéo tới mặt đích ± offset; đích dịch thì tấm tự dài/ngắn | ObjectChanged | |
| 2D: số cao mặt ngăn kéo / nhãn chế độ / kéo đường chia ngăn | `set_drawer_height {cabinet, uid, index, mode?, value?}`, `move_drawer_divider {cabinet, uid, index, before}` | `DrawerSpec.heights` (KHÓA/%/AUTO, dưới → trên); hộc ngăn theo mặt | ObjectChanged | |
| Chỉnh tấm (cánh): Khe trái/phải/dưới/trên, khe giữa cánh | `set_parameter {id, name: door_gap_left …}` | `DoorSpec.side_gaps` | ObjectChanged | |
| Quan hệ 2 tấm (chuột phải khi chọn 2 tấm / Tool 23) | `set_relation {a, b, kind: INSET \| OVERLAY \| FLUSH \| GAP \| NONE, gap}` | Tìm cạnh A hướng về B (và B về A), dựng anchor hai phía (mặt trong / ngoài / bằng mặt) | ObjectChanged | |
| 2D: kéo 4 cạnh tấm (Giữ ràng buộc / Tự do) | `resize_panel_side {id, side: LEFT..FRONT, delta, constrained}` | Có ràng buộc: đổi offset của anchor; tự do: bỏ anchor cạnh đó, đổi offset tấm | ObjectChanged | |
| Tool 21 Cung cạnh / 22 Biên dạng tự do | `shape_tool {ids, op: EDGE_ARC {edge, sagitta} \| POLYGON {points, mode: OUTLINE \| SUBTRACT \| HOLE}}` | Cung tròn (sai số dây 0.1 mm); đa giác kiểm tra tự cắt (Clipper2) | ObjectChanged | Mesh + CNC theo biên dạng |
| Báo cáo | `get_costing`, `set_price {key, value}` | Bóc m², mét chỉ, phụ kiện | SettingsChanged | ReportWindow |
| Sửa ô Rộng/Cao/… (số hoặc `= biểu thức`) | `set_parameter {id, name, value}` | `Command::SetParameter` → `ParamGraph::set_many` (incremental) | GeometryChanged (chỉ các tấm đổi kích thước), TransformChanged, ObjectChanged | lấy lại mesh theo key mới; ma trận mới; properties |
| Số đợt/cánh/ngăn kéo, kiểu nóc/đáy, tấm hậu | `set_parameter` (tham số cấu trúc) | sinh lại tủ, khớp (vai trò, chỉ số) để giữ id; lệnh nghịch đảo `ReplaceSubtree` | ObjectCreated/Deleted/Changed, SceneTreeChanged | như trên |
| Kéo handle W/H/D của tủ | `set_parameter` khi nhả chuột | như trên | như trên | preview khung ghost chỉ ở client |
| Gizmo Move/Rotate | `snap {id, delta}` khi kéo; `set_transform {id, transform}` khi nhả chuột | `aic_spatial::snap_translation`; `Command::SetTransform` (x/y/z được ghi đè thành số) | TransformChanged cho cả cây con | preview bằng ma trận; khi có event thì lấy ma trận thật |
| Xóa / Sao chép (Delete, Ctrl+D) | `delete_objects {ids}` / `duplicate_objects {ids}` | `Batch` của Delete/Duplicate (id mới, lệch vị trí) | ObjectDeleted + SelectionInvalidated / ObjectCreated | gỡ/thêm mesh; bỏ chọn id đã xóa |
| Ẩn/hiện, Khóa | `set_visible` / `set_locked {ids, …}` | Scene node (kế thừa xuống con) | ObjectChanged, SceneTreeChanged | trạng thái hiển thị |
| Kéo thả trong cây | `reparent {id, parent}` | giữ vị trí world; từ chối chi tiết sinh tự động | SceneTreeChanged, ObjectChanged | cây |
| Chọn vật liệu | `set_material {id, material, slot}` | tấm hoặc cả slot của tủ (thùng/cánh/hậu) | GeometryChanged | màu/vân (chỉ hiển thị) |
| Dán cạnh | `set_edge_band {id, edge, enabled}` hoặc `set_parameter edge_left=on` | `Command::SetEdgeBand` | GeometryChanged | properties, bản vẽ trải phẳng |
| Undo/Redo | `undo` / `redo` | `History` chạy lệnh nghịch đảo | tùy lệnh | như trên |
| Mở/Lưu | `load_project {project}` / `save_project` | `ProjectFile` (format `aic-project`, version 1, có migrate) | ProjectLoaded | đồng bộ toàn bộ scene |
| Xem quan hệ | `get_relations {id?}` | sweep → SAT → vùng tiếp xúc (cache theo revision) | — | vẽ polygon tiếp xúc, mũi tên, tooltip, lọc |
| Gia công | `get_manufacturing {id}` | flatten + feature suy diễn (vai trò, liên kết) | — | bản vẽ trải phẳng, danh sách feature |
| Báo cáo | `get_parts` | danh sách chi tiết + tổng theo vật liệu | — | bảng, CSV |
| Xếp tấm | `run_nesting {material, settings}` | `MaxRectsNester` (vân, xoay, khoảng cách, lề) | — | vẽ vị trí đặt, số tấm, % hao hụt |
| CNC | `generate_cnc {material, sheet_id}` | toolpath (Clipper2 offset), G-code | — | đường chạy dao, mô phỏng, G-code |
| Truy vấn khác | `get_scene_tree`, `get_properties`, `get_render_objects`, `get_materials`, `get_bounds`, `get_transform`, `get_status` | — | — | — |

## Mã lỗi (UI dịch sang tiếng Việt, xem `app/src/core-api/errors.ts`)

`CONSTRAINT_VIOLATED` (details.constraint: `WIDTH_LESS_THAN_SIDES`, `HEIGHT_TOO_SMALL`, `DEPTH_TOO_SMALL`,
`THICKNESS_OUT_OF_RANGE`, `TOO_MANY_SHELVES`, `DOOR_TOO_NARROW`), `DEPENDENCY_CYCLE`, `INVALID_PARAMETER`, `LOCKED`,
`NOT_FOUND`, `INVALID_TRANSFORM`, `INVALID_REPARENT`, `GEOMETRY_BOOLEAN_FAILED`, `INVALID_FEATURE`,
`UNKNOWN_MATERIAL`, `UNSUPPORTED_VERSION`, `INVALID_PROJECT`, `NOTHING_TO`.
