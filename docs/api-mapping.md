# Ánh xạ UI Action → Core Command → Response → UI Update

Mọi request là JSON `{"cmd": "...", ...}` gửi tới `Engine::dispatch_json`
(Tauri: lệnh IPC `dispatch`; trình duyệt: `POST /api`). Response luôn có dạng
`{ ok, result, events[], error{code,message,details}, revision, can_undo, can_redo }`.

| UI action | Request (`cmd`) | Core | Events | UI update |
|---|---|---|---|---|
| Ribbon Tủ → click vị trí → nhập W/H/D | `create_cabinet {kind, position, overrides}` | `Command::CreateCabinet` → generator → solve tham số | ObjectCreated, SceneTreeChanged | `get_render_objects(ids, known_keys)` → thêm mesh; làm mới cây; chọn tủ mới |
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
