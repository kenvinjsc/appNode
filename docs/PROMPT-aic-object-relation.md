# Prompt: nâng `aic_object_relation` thành core của Parametric Cabinet Editor

> Dán toàn bộ nội dung dưới đây vào Claude Code (hoặc AI agent khác) khi làm việc **trong repo `aic_object_relation`**.
> Tài liệu tổng hợp tính năng: `TONG-HOP-PARAMETRIC-EDITOR.md`. Mẫu tham khảo đã chạy: repo AIC CAD (`kenvinjsc/appNode`).

---

## Vai trò

Bạn là Principal Product Architect, Senior CAD UX Engineer và Senior TypeScript/Ruby/Rust Engineer.
Bạn làm việc trong repo `aic_object_relation`.

## Bước 0: đọc repo trước khi viết code

1. Đọc cấu trúc repo, ngôn ngữ, cách build/test, mô hình object và relation hiện có, và giao thức với Web/SketchUp nếu đã có.
2. Viết `docs/ARCHITECTURE-PARAMETRIC.md` trình bày ba thứ:
   - (a) những gì đã có;
   - (b) ánh xạ các khái niệm dưới đây vào code hiện có;
   - (c) phần còn thiếu.
3. **Không viết lại** những gì đã chạy. Mở rộng theo quy ước của repo.
4. Chỗ nào spec mâu thuẫn với code hiện có thì ghi vào tài liệu và chọn phương án ít phá vỡ nhất.

## Mục tiêu

Biến AIC CAD thành **Parametric Cabinet Editor**:
- **Web** là nơi thao tác 2D: chọn, dimension sửa được, kéo vách/kệ, chia khoang, inspector, quản lý logical model.
- **SketchUp** hiển thị và chỉnh 3D native.
- **`aic_object_relation`** là nguồn dữ liệu canonical: object, zone, parameter, constraint, relation, dependency, history.
- Đồng bộ 2 chiều qua **WebSocket/RPC**.
- **Mọi thay đổi là incremental**: sửa chỗ nào cập nhật đúng chỗ đó, không rebuild cả model hay cả 2D.

## Nguyên tắc bắt buộc

1. **Core là nguồn sự thật.** Web và SketchUp chỉ gửi Command và vẽ kết quả. Không tính kích thước, relation hay zone ở client.
2. **Kích thước là tham số.** Không scale mesh/group. Transform không chứa scale; SketchUp group bị scale thì core đọc lại thành tham số (xem §SU→Core).
3. **Mọi thay đổi dữ liệu là một Command** có `execute()` trả về lệnh nghịch đảo (undo/redo). Không snapshot cả dự án.
4. **Id ổn định**:
   - Mỗi object sinh ra có `ObjectId` bền và một key logic ổn định trong model, ví dụ `c:left`, `split:{uid}`, `door:{uid}:{r}:{c}`, `drawer:{uid}:{i}:front`.
   - Sinh lại cấu trúc thì dùng lại id theo key.
   - Undo xóa tấm phải trả lại đúng id cũ (giữ map `retired (model,key) → id`).
5. **Logical model theo `model_id`.** SketchUp **không** có Group cha cho tủ. Mỗi entity SU mang attribute `aic.object_id`, `aic.model_id`, `aic.key`. Mọi hành động cấp tủ lấy toàn bộ member theo `model_id`.
6. **Feature gia công** lưu ở tọa độ local của tấm (X = rộng, Y = cao, Z = dày).
7. **Lỗi có `code` ổn định**, kèm `details`. Client dịch sang tiếng Việt; không hiện lỗi kỹ thuật.
8. **Không spam thông báo.** Chỉ báo khi: geometry không hợp lệ, xung đột constraint, mất relation, zone không hợp lệ, lệnh lỗi, lưu template thành công.

## Mô hình dữ liệu (tối thiểu)

```text
Project
 └ Model (model_id, name, floor, room, template_ref, rule_preset_ref)
    ├ params:   W, H, D, thickness, back_thickness, back_groove, door_gap, shelf_setback, …
    ├ anchors:  width: KEEP_LEFT|KEEP_CENTER|KEEP_RIGHT, height: KEEP_BOTTOM|…, depth: KEEP_FRONT|…
    ├ carcass rules: top/bottom ↔ sides: OVERLAY|INSET|FLUSH|GAP(mm), back: INSET|GROOVE(depth)
    ├ ZoneTree
    │   Zone { zone_id, box (tính ra), split?: Split, front?: Front, links[] }
    │   Split { axis: X|Y|Z, members: [SplitMember] , children: [Zone] }
    │   SplitMember { uid, kind: DIVIDER|SHELF_FIXED|SHELF_ADJ|BACK_SUB, thickness, tilt }
    │   Bay sizing (một cho mỗi child zone): { mode: LOCK|AUTO|PERCENT, value }
    │   Front = Door{kind SINGLE|DOUBLE|SLIDING, cols, rows, mount OVERLAY|INSET, gaps{l,r,t,b,center}, hinge L|R|T|B}
    │         | Drawers{count, heights[] (LOCK/AUTO/PERCENT), mount, side_clearance, front_gap, slide{type,length|auto}, bottom{type}}
    ├ PartMods[key]: name, deleted, offsets{front,back,left,right,top,bottom}, thickness?, material?, edges{…},
    │                contour?, features[], split?(chia tấm), tools[]
    └ Constraints[]: { id, subject: (object_id, edge L|R|T|B|F|K), target: (object_id, face INNER|OUTER|…), offset }
Relations (tính ra + lưu cache): { id, a, b, kind: OVERLAY|INSET|FLUSH|GAP|TOUCH|GROOVE, contact }
Template: Model đã chuẩn hóa (bỏ id, giữ key + rule + tham số + constraint + vật liệu + rule phụ kiện/gia công)
RulePreset: { name, values{...} } (ví dụ "AIC Wardrobe Standard")
```

### Giải khoang: LOCK / AUTO / PERCENT

Cho chiều dài trong `L` và các tấm chia dày `t_i`, chiều dài dùng được là `U = L − Σt_i`:

1. Khoang LOCK nhận đúng `value` mm.
2. Khoang PERCENT nhận `value% × U`.
3. Phần còn lại `R = U − ΣLOCK − ΣPERCENT` chia đều cho các khoang AUTO.
4. Không có khoang AUTO mà `R ≠ 0`: phân phối theo tỉ lệ cho PERCENT; nếu không có PERCENT thì báo `CONSTRAINT_CONFLICT` (không tự chia đều lại).
5. Khoang nào < 0 hoặc < min (mặc định 50 mm): báo `ZONE_INVALID` và **rollback**.

Ví dụ phải đúng:
- `600 LOCK | AUTO | 400 LOCK`, W 1600 → 1800: khoang giữa tăng 200, hai bên giữ 600 và 400.
- `40% | 20% | 40%` giữ tỉ lệ khi đổi kích thước.

Kéo vách giữa 2 khoang = đổi `value` của khoang bên trái (hoặc khoang LOCK gần nhất). Mode giữ nguyên; AUTO thì chuyển LOCK. Ghi rõ quy tắc này trong tài liệu.

### Resize tủ có anchor

Đổi W/H/D chỉ đổi tham số và transform gốc theo anchor:
- KEEP_LEFT: gốc giữ nguyên.
- KEEP_RIGHT: gốc dịch `−ΔW`.
- KEEP_CENTER: gốc dịch `−ΔW/2`.

Tương tự cho H và D. Sau đó giải lại zone, relation và constraint của **model đó**.

### Constraint động

Mỗi constraint có dạng `Shelf.Right → RightPanel.InnerFace, offset 0`. Khi target dịch, core tính lại cạnh của subject:
- Đổi kích thước hoặc offset của subject.
- Cập nhật relation cục bộ.

Xử lý vòng phụ thuộc và xung đột:
- Phát hiện vòng bằng đồ thị dependency và báo `DEPENDENCY_CYCLE`.
- Hai constraint cùng khóa một cạnh: báo `CONSTRAINT_CONFLICT`.

Constraint ngầm từ zone (kệ bám mặt trong hồi) phải **hiển thị được** trong inspector dưới dạng constraint, kể cả khi chưa cho sửa.

## Giao thức

### Command (Web/SU → Core)

Mọi request có `request_id`, `origin: "web"|"su"|"core"` và `phase: "PREVIEW"|"COMMIT"` (mặc định COMMIT).

Tối thiểu cần các lệnh sau:

- **Tham số tủ:** `SET_PARAMETER {object_id|model_id, name, value}`, `SET_ANCHOR {model_id, axis, mode}`.
- **Chọn:** `SELECT {object_ids|zone_ids}`. Lệnh này chỉ đồng bộ selection, không vào history.
- **Chia khoang:**
  - `SPLIT_ZONE {zone_id, axis: H|V, count, distribution: EQUAL|PERCENT|FIXED_AUTO|CUSTOM, values[]}`
  - `SET_BAY {zone_id, index, mode, value}`
- **Vị trí vách/kệ:** `MOVE_SPLIT_MEMBER {zone_id, uid, position}` (kéo vách), `SET_SHELF_LEVEL {uid, z}`
- **Thêm chi tiết vào zone:**
  - `ADD_SHELVES {zone_id, count, distribution: EQUAL|BY_DISTANCE|BY_LEVEL, values[]}`
  - `ADD_DIVIDER {zone_id, mode: CENTER|PERCENT|FROM_LEFT|FROM_RIGHT, value}`
  - `ADD_DOOR {zone_id, …}`, `ADD_DRAWERS {zone_id, …}`, `SET_DRAWER_HEIGHT {zone_id, index, mode, value}`
  - `REMOVE_ATTACHMENT {zone_id, uid}`
- **Chỉnh tấm:**
  - `SET_PART_MOD {object_id, patch}`: offset, thickness, material, edges, contour, split.
  - `RESIZE_PANEL {object_id, edge, delta, mode: FREE|CONSTRAINED}`
- **Constraint và relation:**
  - `ADD_CONSTRAINT {subject, edge, target, face, offset}`, `REMOVE_CONSTRAINT {id}`
  - `SET_RELATION {a, b, kind, gap?}` (phủ/lọt/flush/gap)
- **Multi-edit:** `MULTI_EDIT {object_ids, patch}`, gộp thành một Batch, một bước undo.
- **Copy:** `DUPLICATE {object_ids|model_id}`, `ARRAY {object_ids, axis, count, spacing|EQUAL}`. Clone cả tham số, constraint, dán cạnh, gia công, relation, metadata.
- **Model:** `MOVE_MODEL / ROTATE_MODEL / COPY_MODEL / DELETE_MODEL / HIDE_MODEL / LOCK_MODEL {model_id, …}`, `MIRROR_MODEL {model_id, axis}`
- **Template:** `SAVE_TEMPLATE {model_id, name}`, `INSERT_TEMPLATE {template_id, W, H, D, position, room?, floor?}`
- **Preset:** `SAVE_RULE_PRESET {name, values}`, `APPLY_RULE_PRESET {model_id, preset}`
- **Lịch sử:** `UNDO`, `REDO`
- **Truy vấn:** `GET_MODEL {model_id}`, `GET_ZONES {model_id}`, `GET_DIMENSIONS {model_id, view}`, `GET_PROJECTION {model_id, view, ids?}`, `GET_RELATIONS {object_id}`, `GET_INSPECTOR {object_ids}` (trả `Mixed` khi giá trị khác nhau)

### ChangeSet (Core → mọi client)

```json
{
  "revision": 1234,
  "origin": "web", "request_id": "…", "phase": "COMMIT",
  "objectsCreated": [], "objectsUpdated": ["panel_105"], "objectsDeleted": [],
  "zonesUpdated": ["zone_12", "zone_13"],
  "dimensionsUpdated": ["dim_shelf_z", "dim_upper_bay", "dim_lower_bay"],
  "relationsUpdated": ["rel_105_101", "rel_105_102"],
  "constraintsUpdated": [],
  "selection": null,
  "errors": []
}
```

- Client **chỉ patch đúng các id** trong ChangeSet.
- Mỗi client bỏ qua ChangeSet có `origin` là chính nó **và** `request_id` do nó gửi. Nó chỉ đối chiếu revision, để tránh vòng lặp Web ↔ SU.
- `dimensionsUpdated` dùng **dimension id ổn định**: `dim:{model}:{kind}:{zone|uid}:{axis}`.

### PREVIEW và COMMIT

| | PREVIEW (trong khi kéo) | COMMIT (thả chuột / Enter) |
|---|---|---|
| Core | tính nhanh tham số bị ảnh hưởng, **không** ghi history, không lưu, không solve relation nặng | solve đầy đủ, ghi history, lưu, relation, constraint |
| Web | patch cục bộ, hiện live dimension, snap | nhận ChangeSet |
| SU | cập nhật có throttle (≤ 10 Hz), chỉ transform/size các entity bị ảnh hưởng | cập nhật trong một `start_operation … commit_operation` |
| Hủy (Esc) | gửi `CANCEL_PREVIEW`, core trả lại state trước khi kéo | — |

## Phạm vi invalidate (incremental)

Mỗi Command khai báo tập ảnh hưởng. Ví dụ kéo kệ `panel_105`:
- Projection 2D của `panel_105`.
- Dimension của nó và của 2 khoang kề.
- Zone trên và zone dưới.
- Relation cục bộ với hồi hoặc vách hai bên.
- Chân kệ hoặc gia công trên tấm kề nếu rule sinh ra.

**Không** invalidate cả tủ. Viết test đếm số object bị cập nhật.

## SketchUp adapter (Ruby extension)

- **Kết nối:** WebSocket client tới core (hoặc core nhúng qua bridge). Tự kết nối lại. Hàng đợi lệnh khi mất kết nối.
- **Web → SU:** nhận ChangeSet rồi, trong `model.start_operation("AIC …", true)`:
  - Tạo, sửa hoặc xóa group/component theo `aic.object_id`.
  - Cập nhật kích thước bằng dựng lại hình hộp hoặc contour, **không scale**.
  - Áp transform.
  - `commit_operation`.
- **Tấm có contour:** dựng face từ polygon rồi pushpull theo độ dày. Gia công có thể là layer hoặc tag riêng, tùy chọn.
- **SU → Core:** dùng `EntityObserver` / `EntitiesObserver` / `ModelObserver` và debounce 150–300 ms. Geometry Interpreter:
  - Group bị move hoặc rotate → `SET_TRANSFORM`. Với cả model → `MOVE_MODEL`.
  - Group bị scale theo 1 trục → đổi thành tham số (`RESIZE_PANEL` hoặc `SET_PARAMETER`), rồi **reset scale** về 1.
  - Group bị xóa → `DELETE`.
  - Entity lạ (không có `aic.object_id`) → bỏ qua.
- **Cờ `applying_remote`:** bật khi đang áp ChangeSet để observer không gửi ngược.
- **Undo của SU:** mỗi ChangeSet là một operation. Khi người dùng Undo trong SU, gửi `UNDO` cho core; không để SU tự hoàn tác lệch với core.
- **Hành động theo model:** chọn một tấm rồi bấm "Chọn cả tủ" → select mọi entity cùng `aic.model_id`.

## 2D editor (Web)

- **View:** Front, Top, Left, Right, Section (mặt cắt tại cao độ/chiều sâu chọn được), Panel Detail.
- **Nguồn dữ liệu:** projection lấy từ core (`GET_PROJECTION`); cache theo `object_id`; patch theo ChangeSet. Mỗi phần tử SVG/canvas có `data-object-id` hoặc `data-zone-id`.
- **Dimension:**
  - Là object riêng (id ổn định), core trả về cùng `param_ref`.
  - Click vào số hiện ô nhập. Enter → `SET_PARAMETER` / `SET_BAY` / `SET_SHELF_LEVEL`.
  - Hiện mode (🔒 LOCK / AUTO / %). Click biểu tượng để đổi mode.
- **Handle:** kéo vách, kệ, đường chia ngăn kéo và cạnh tấm (FREE / CONSTRAINED).
  - Trong lúc kéo gửi PREVIEW có throttle, thả chuột gửi COMMIT.
  - Snap: lưới, mặt tấm, trung điểm, chia đều, cao độ lưu sẵn. Snap do core tính; client chỉ hiển thị gợi ý.
- **Context menu:**
  - Cabinet: Edit Size, Move, Rotate, Duplicate, Mirror, Save Template, Hide, Lock, Report, Delete.
  - Zone: Add Shelf, Add Divider, Add Door, Add Drawer, Split Horizontal, Split Vertical, Equal Divide.
  - Panel: Edit, Move, Resize, Duplicate, Hide, Lock, Edge Band, Machining, Relations, Delete.
- **Inspector theo ngữ cảnh:** giữ các tab Khung · Tạo tấm · Chỉnh tấm · Quản lý · Thư viện · Cài đặt. Tab tự chuyển theo loại đối tượng đang chọn (model / zone / panel / nhiều tấm). Nhiều tấm có giá trị khác nhau thì hiện "Mixed".

## Lộ trình và tiêu chí nghiệm thu

Làm **theo thứ tự**. Mỗi phase gồm:
- code và test của phase;
- cập nhật `docs/ARCHITECTURE-PARAMETRIC.md` và bảng ánh xạ UI → Command → ChangeSet;
- commit riêng.

| Phase | Phạm vi | Test bắt buộc |
|---|---|---|
| 1 | W/H/D + anchor, dimension sửa được, LOCK/AUTO/PERCENT, chọn zone | **A.** Resize 1600→1800: khoang LOCK giữ nguyên, khoang AUTO đổi, kệ phụ thuộc đổi dài, cánh cập nhật, ChangeSet chỉ chứa object bị ảnh hưởng, SU nhận đúng id |
| 2 | Split zone, kéo vách, kéo kệ, thêm kệ, thêm vách, PREVIEW/COMMIT | **B.** Kéo vách 742→900: vách dịch, 2 khoang đổi, dimension cập nhật, chỉ tấm liên quan đổi · **C.** Kệ Z 1133→1250: khoang trên/dưới đổi, relation cục bộ, không redraw toàn bộ |
| 3 | Constraint động, phủ/lọt/flush/gap, offset 6 phía, multi-edit | **D.** `Shelf.Right → RightPanel.InnerFace`, dịch hồi phải thì kệ tự dài/ngắn · **E.** Chọn 6 kệ, dày 18, front offset 30: chỉ 6 kệ đổi, một bước undo |
| 4 | Cánh và ngăn kéo tham số (gap từng phía, bản lề L/R/T/B, cao từng ngăn), Copy/Array | Đổi cao zone thì cánh và mặt ngăn tự tính lại; Array kệ ×6 chia đều |
| 5 | Custom contour (corner cut, chamfer, radius, arc, notch, polygon), Template, Rule preset | **F.** Lưu template tủ áo, chèn với W/H/D khác: cấu trúc đúng logic, LOCK/PERCENT giữ nguyên nghĩa |
| 6 | Gia công, dán cạnh, relations inspector, xuất sản xuất | Danh sách cắt và G-code theo contour mới |

Ngoài test theo phase:
- Test đơn vị cho bộ giải khoang (LOCK/AUTO/PERCENT, xung đột, rollback).
- Test undo/redo giữ nguyên id.
- Test ChangeSet tối thiểu, đếm số id bị cập nhật.
- Test chống vòng lặp đồng bộ (`origin`/`request_id`).

## Definition of Done

1. Chỉnh tủ bằng dimension trực tiếp trên 2D.
2. Có LOCK / AUTO / PERCENT.
3. Có Zone Editor.
4. Kéo được kệ và vách (PREVIEW/COMMIT).
5. Thêm kệ, vách trực tiếp trên zone.
6. Có constraint động.
7. Có phủ/lọt.
8. Có offset.
9. Có multi-edit (Mixed).
10. Cánh và ngăn kéo tham số.
11. Có template và rule preset.
12. Web ↔ SketchUp đồng bộ incremental 2 chiều.
13. Không redraw toàn bộ 2D.
14. Không cần Group cha trong SketchUp (dùng `model_id`).
15. `aic_object_relation` quản lý Logical Model và dependency.

## Tham khảo từ AIC CAD (`kenvinjsc/appNode`), dùng lại ý, không copy máy móc

| Chủ đề | File | Ghi chú |
|---|---|---|
| ZoneTree, split, khóa Tỷ lệ/Cách A/Cách B | `crates/aic-domain/src/zone.rs` | Khung cho LOCK/AUTO/PERCENT; cần chuyển mode từ *vị trí tấm* sang *kích thước khoang* |
| Sinh tấm theo zone, key ổn định, PartMod | `crates/aic-domain/src/layout.rs` | `c:left`, `p:{uid}`, `d:{uid}:{r}:{c}`, `w:{uid}:…`, chia tấm `key~n` |
| Sinh lại + dùng lại id khi undo | `crates/aic-project/src/document.rs` (`regenerate_cabinet`, `retired`) | |
| Command + lệnh nghịch đảo, Batch | `crates/aic-project/src/command.rs` | |
| API zone (ghim, thêm tấm/cánh/ngăn kéo) | `crates/aic-api/src/zones.rs`, `docs/api-mapping.md` | |
| Contour: bo/vát góc, cắt đường, cắt theo tấm, hợp tấm | `crates/aic-api/src/shape.rs` | Thuật toán polygon đã có test |
| Luật dán cạnh, báo giá, danh sách cắt | `crates/aic-api/src/costing.rs` | |
| Mã lỗi → tiếng Việt | `app/src/core-api/errors.ts` | |
| Hướng dẫn thao tác hiện tại | `docs/HUONG-DAN-DUNG-TU.md` | |

**Ngôn ngữ UI:** tiếng Việt, gọn, sáng. Màu nhấn cam `#e8590c`, icon SVG, không dùng emoji. Chỗ duy nhất được dùng biểu tượng là 🔒 trong mode dimension, và nên thay bằng SVG nếu repo có bộ icon.
