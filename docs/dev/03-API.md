# 03. Lớp API (`crates/aic-api`)

## Engine

`aic_api::Engine` giữ `Document`, `History`, `CsgKernel`, cache mesh theo `geometry_key`, cache quan hệ / khoan liên kết
/ nesting theo `revision`, thư viện dùng chung (`library`, `merged` nguồn nhóm). Một engine = một dự án đang mở.

```
dispatch_json(&str) → serde Request (tag "cmd", snake_case)
dispatch(req):
    mark = history.mark()
    r = handle(req)                    // match lớn trong lib.rs
    nếu có dãy tủ và lệnh có sửa: sync_runs() rồi squash(mark) | rollback(mark)
    respond(r): take_changes() → CoreEvent[] + Response{ok, result, events, error, revision, can_undo, can_redo}
```

## Module

| File | Phụ trách |
|---|---|
| `protocol.rs` | `Request` (mọi lệnh), `Response`, `CoreEvent`, `ApiError` |
| `lib.rs` | `Engine`, `dispatch`, `handle` (match mọi request), `preview`, `set_parameter`, `resize_cabinet`, snap |
| `zones.rs` | Sửa khoang / tủ: `edit_cabinet`, `edit_cabinet_checked`, `set_zone_property` (định tuyến khóa thuộc tính), zone_add_*, split, bay |
| `properties.rs` | Cây đối tượng, bảng thuộc tính (`get_properties`, multi) sinh field cho UI |
| `structure_api.rs` | Tab "Thuộc tính kết cấu" (`get_structure`), mẫu theo tab, chuẩn xưởng `s_*` |
| `furniture.rs` | `create_furniture` (BED / DESK / CLADDING / ISLAND), khóa `bed_*` `desk_*` `cl_*`, tab sản phẩm |
| `products.rs` | Danh mục mẫu dựng sẵn D30 (`PRODUCTS`, `Param`), `insert_product` |
| `templates.rs` | Template tủ, rule preset, mirror, array |
| `runs.rs` | Dãy tủ (mặt đá, len liền, tấm lấp), sinh lại sau mỗi lệnh |
| `arrange.rs` | Căn / phân bố / xoay / sát tường |
| `shape.rs` | Tool hình dạng tấm (bo góc, cắt, đa giác, cung cạnh, hợp tấm) |
| `relations_edit.rs` | Quan hệ 2 tấm, kéo cạnh tấm |
| `render.rs` | `get_render_objects`: mesh chia sẻ theo `geometry_key`, ma trận thế giới, `look` (GLASS / MIRROR) |
| `mfg.rs` | Quan hệ lắp ráp, khoan liên kết, trải phẳng, danh sách tấm, nesting, CNC (cache theo revision) |
| `costing.rs` | Luật dán cạnh, báo giá, danh sách cắt, Cabinet List |
| `drawing.rs` | Bản vẽ in A3/A4 (`get_drawing_sheet`) |
| `library.rs`, `library_sources.rs` | Thư viện máy + nguồn nhóm (thư mục chung) |
| `room_rules.rs` | Loại phòng và luật áp khi tạo tủ |
| `tests.rs` | Test tích hợp qua `dispatch` (JSON thật) |

## Request / Response

```json
→ {"cmd": "set_parameter", "id": 12, "name": "width", "value": "800"}
← {"ok": true, "result": {...}, "events": [{"type": "ObjectChanged", "ids": [..]}, ...],
   "error": null, "revision": 42, "can_undo": true, "can_redo": false}
← lỗi: {"ok": false, "error": {"code": "CONSTRAINT_VIOLATED", "message": "...", "details": {"constraint": "ZONE_TOO_SMALL"}}}
```

Bảng đầy đủ request ↔ thao tác UI: `docs/api-mapping.md`. Test nhanh bằng curl:

```bash
curl -s localhost:8790/api -d '{"cmd":"get_scene_tree"}' | head -c 400
```

## Sự kiện (`CoreEvent`)

`ObjectCreated/Deleted/Changed/GeometryChanged/TransformChanged {ids}`, `SceneTreeChanged`,
`ZonesChanged {cabinets}` (2D / Tạo tấm chỉ tải lại khoang tủ đó), `SelectionInvalidated`, `ProjectLoaded`.
Phần lớn sinh tự động từ `ChangeSet` của Document; lệnh đặc biệt đẩy vào `pending_events`.

## Lỗi

`aic_project::CoreError` (`error.rs`) serde `tag = "code"`: `NotFound`, `InvalidParameter`, `ConstraintViolated
{constraint, message}`, `Locked`, `DependencyCycle`, `InvalidFeature`, `NothingTo {action}`… Nghiệp vụ mới thường dùng
`CoreError::ConstraintViolated { constraint: "MA_MOI".into(), message }` rồi thêm bản dịch vào
`CONSTRAINTS` trong `app/src/core-api/errors.ts`. Helper `zones::bad(key, reason)` cho tham số sai.

## Khóa thuộc tính: một cửa `set_parameter`

Hầu hết ô nhập trong UI gửi `set_parameter {id, name, value: string}`. Đường đi trong `lib.rs::set_parameter`:

1. `zones.rs::set_zone_property(id, name, value)` thử trước, định tuyến theo **tiền tố khóa**:
   - Trên tủ: `bed_*` `desk_*` `cl_*` → `furniture.rs::set_product_property`;
     `back_*` `rt_*` `tr_*` `island_*` `plinth_*` `leg_count` `hang_rail` `pricing` `dg_*` … → sửa `c.rules`
     qua `edit_cabinet_checked`; `s_*` → chuẩn xưởng (`structure_api::set_shop_field`, tự suy kiểu từ serde);
     `room`, `floor`…
   - Trên tấm sinh ra: tìm phần tử khoang tương ứng (kệ: `tilt_fb`, `extent`; cánh: `door_*`, `door_slide_*`,
     `door_lift`, `door_glass_*`; ngăn kéo…).
2. Không khớp: `name`, `rx/ry/rz`, vật liệu, `edge_*`, `width/height/depth` (→ `resize_cabinet`), còn lại là tham số
   thường (`Command::SetParameter`).

Bảng thuộc tính / tab kết cấu do **core sinh** (`properties.rs`, `structure_api.rs::structure_tabs`,
`furniture.rs::product_tabs`) bằng các helper `num`, `flag`, `text`, `select`, `section`. UI chỉ vẽ field và gửi lại
`key`. Thêm một tùy chọn mới = thêm field ở hàm sinh tab + nhánh xử lý khóa; thường **không phải sửa UI**.

## `edit_cabinet` và `edit_cabinet_checked`

```rust
self.edit_cabinet_checked(cab, "Nhãn undo", |c: &mut Cabinet| { c.rules.x = ...; Ok(()) })
```

Clone định nghĩa → sửa → `Command::SetCabinet`. Bản `_checked` dựng thử layout và **từ chối** nếu sinh khoang lỗi
mới (`ZONE_TOO_SMALL`, hoặc `LIFT_HEIGHT` nếu do cánh lật). Dùng bản checked cho mọi thay đổi kích thước / kết cấu.

## Preview

`{"cmd": "preview", "cabinet": id, "request": {...}}` chạy thử một số request sửa tủ (MoveSplitPanel, SetBay,
ResizeCabinet, SetParameter, ResizePanelSide, MoveDrawerDivider, SetDrawerHeight, EqualizeSplit) rồi rollback:
không undo, không revision, không sự kiện. UI dùng khi kéo chuột để hiện số do core tính.

## Thao tác nhiều bước = một undo

Mẫu chung (xem `products.rs::insert_product`, `furniture.rs::create_furniture`, `room_rules`):

```rust
let mark = self.history.mark();
let r = (|| { /* nhiều handle(...) / exec_cmd(...) */ })();
match r { Ok(v) => { self.history.squash(mark, "Nhãn"); Ok(v) }
          Err(e) => { self.history.rollback(&mut self.doc, mark); Err(e) } }
```
