# 02. Core: mô hình dữ liệu

## Document (`crates/aic-project/src/document.rs`)

```rust
pub struct Document {
    meta, settings: ProjectSettings,      // settings: giá, template, rule preset, dãy tủ, loại phòng …
    scene: Scene,                         // cây node + transform local/world (aic-domain/src/scene.rs)
    objects: BTreeMap<ObjectId, DomainObject>,   // Cabinet | Panel | Hardware | Room | …
    params: ParamGraph,                   // tham số + biểu thức + constraint (aic-parametric)
    materials, ids, overrides, generated, revision,
    changes: ChangeSet,                   // gom thay đổi → CoreEvent
    retired,                              // id tấm đã mất theo (tủ, key): tấm quay lại thì lấy lại id cũ
    zone_sig,                             // hash khoang mỗi tủ → phát ZonesChanged chỉ khi khoang thật sự đổi
}
```

- `ProjectSettings`: `prices`, `screws`, `templates`, `presets` (rule preset), `runs` (dãy tủ),
  `room_types` (tên phòng → loại, D33). Dữ liệu dùng chung **mọi dự án** (mẫu nhóm, chuẩn xưởng, thư viện) nằm
  ở `aic-api/src/library.rs` (file `$AIC_LIBRARY` hoặc `~/.aic-cad/library.json`), không ở Document.
- `cabinet_values(id)` đọc tham số đã giải; `cabinet_layout(id)` chạy layout (không đổi dữ liệu) để đọc
  `bays`, `zones`, `problems`, `misfits`…
- `regenerate_cabinet(id)` (gọi tự động sau command đổi tủ): chạy layout, so khớp `Part.key` với tấm cũ,
  tạo / sửa / xóa đối tượng tấm, ghi vào `changes`.

## Command & History (`command.rs`, `history.rs`)

- `Command` là enum (CreateCabinet, CreatePanel, DeleteObject, SetParameter, SetTransform, SetCabinet,
  SetPrice, SetRoomType, SetTemplate, SetRulePreset, SetRun, Batch, Restore…). `execute(doc)` trả lệnh nghịch đảo.
- Phần lớn tính năng tủ đi qua **một** lệnh: `Command::SetCabinet { id, cabinet: Box<Cabinet>, label }`
  (thay cả định nghĩa tủ; nghịch đảo là định nghĩa cũ). Không cần thêm biến thể Command cho mỗi tính năng.
- `History`: `execute`, `undo`, `redo`, và bộ ba dùng để **gộp nhiều lệnh thành một undo**:

```rust
let mark = self.history.mark();
// … nhiều exec_cmd(...)
self.history.squash(mark, "Nhãn hiện ở Undo");      // thành công
self.history.rollback(&mut self.doc, mark);         // lỗi giữa chừng → trả về như cũ
```

  `Engine::dispatch` dùng cơ chế này để sinh lại dãy tủ (D09) trong cùng bước undo; `preview` dùng
  `rollback` + `take_redo/restore_redo` để chạy thử không để lại dấu vết.

## Tủ (`aic-domain`)

| Kiểu | File | Ý nghĩa |
|---|---|---|
| `Cabinet` / `CabinetSpec` / `CabinetKind` | `cabinet.rs` | Tủ: loại (WARDROBE, BASE, WALL…), vật liệu theo slot, phòng / tầng, `zones: ZoneTree`, `rules: StructureRules`, `mods: BTreeMap<String, PartMod>` |
| `Zone`, `Split`, `SplitPanel`, `Bay` | `zone.rs` | Cây khoang: mỗi `Zone` có `split` (chia theo trục, `panels` + `children` + `bays` LOCK/AUTO/PERCENT), `front` (`Front::Doors(DoorSpec)` / `Front::Drawers(DrawerSpec)`), `links` (thanh treo, phụ kiện, khoang thiết bị…). Mọi phần tử có `Uid` ổn định |
| `StructureRules` | `structure.rs` | Luật kết cấu theo tủ: `back` (hậu, chia, khoét), `top_rails`, chân / len, `shop` (chuẩn xưởng D01), `trim` (phào D13), `island` (D23), `diagonal` (tủ góc chéo), `product` (giường / bàn / vách ốp), `pricing` |
| `Product` + spec | `product.rs` | `Bed`, `Desk`, `Cladding`: sản phẩm không phải tủ hộp |
| `MachiningFeature` | `feature.rs` | Khoan, hốc, rãnh, contour (ngoài / trong), tọa độ local tấm |
| `PartMod` | `layout.rs` | Sửa theo tấm (khóa = `Part.key`): đổi tên, xóa, co giãn 4 cạnh, offset 6 phía, dày, vật liệu, dán cạnh, feature thêm, anchor (ràng buộc động), chia tấm, nhóm nối vân |

Mọi trường mới trong các struct lưu file phải có `#[serde(default)]` (và `skip_serializing_if` nếu là Option / rỗng)
để **file dự án cũ vẫn mở được**.

## Layout generator (`crates/aic-domain/src/layout.rs`)

`pub fn build(cab: &Cabinet, v: CabinetValues) -> Layout` là hàm thuần (không I/O, không id). Thứ tự:

```
diagonal?  → diagonal_corner()             (tủ góc chéo, trả luôn)
product?   → layout/products.rs::build()  (bed / desk / cladding, trả luôn)
carcass()  → hồi, nóc, đáy, chân/len (base), giằng (top_rails)  → ZBox lòng tủ
zone()     → đệ quy cây khoang: split_panel / tilted_shelf, doors / sliding_doors / framed_leaf,
             drawers, link (phụ kiện, khoang thiết bị → Ctx.vents)
backs()    → hậu (sau zone vì cần kệ cố định, khoét, khe thoát nhiệt)
trims()    → phào, ốp hông, nẹp
island     → mặt đá c:island_top
apply_mods → PartMod (co giãn, offset, anchor, chia tấm …)
```

`Layout` trả về: `parts: Vec<Part>` (key, name, kind, size, translation, rotation_deg), `zones` (hộp khoang cho UI ghim),
`positions`, `bays` / `front_bays` (số đo sửa được trên 2D), `fittings` (phụ kiện đếm cho báo giá), `problems`
(khoang không giải được), `misfits` (phụ kiện không vừa: chỉ cảnh báo).

**Khóa tấm (`Part.key`) phải ổn định** giữa các lần dựng. Quy ước tiền tố:

| Tiền tố | Ví dụ | Nguồn |
|---|---|---|
| `c:` | `c:left`, `c:top`, `c:back`, `c:plinth`, `c:island_top` | Thân tủ |
| `p:{uid}` | `p:17` | Tấm chia (vách / kệ) của `SplitPanel` |
| `d:{uid}:{r}:{c}` | `d:5:0:1` | Cánh / mặt ngăn kéo |
| `h:`, `t:`, `s:`, `w:` | `h:5:0:0` | Tay nắm, ray lùa, thanh chặn, hộc ngăn kéo |
| `l:{uid}:…` | `l:9:support` | Theo `Link` |
| `p:dg{i}` | | Tủ góc chéo |

Đổi khóa = tấm mất id, mất `PartMod` người dùng đã sửa. Thêm tấm mới thì đặt khóa mới, đừng đổi khóa cũ.

Tọa độ tủ: gốc ở góc trái-dưới-sau, X rộng, Y cao, Z sâu (mặt trước ở `z = depth`). Tấm nằm ngang dùng
`ROT_HORIZONTAL`. Bàn đảo: khoang sau dựng như khoang thường rồi xoay 180° quanh trục Y.

## Tham số (`aic-parametric`)

Tham số tủ (`width`, `height`, `depth`, `thickness`, `plinth_height`…) nằm trong `ParamGraph` với khóa `#id.name`,
có thể là biểu thức (`=a.width/2`). Constraint (ví dụ `WIDTH_LESS_THAN_SIDES`) trả mã ổn định. Đổi W/H/D tủ đi qua
`resize_cabinet` (neo + dịch tủ kề cùng dãy, từ chối nếu khoang không giải được).
