# 01. Kiến trúc

## Hai nửa

```
app/ (React 18 + Three.js 0.169 + zustand, Vite)          crates/ (Rust, nguồn dữ liệu gốc)
┌────────────────────────────────────┐   JSON    ┌───────────────────────────────────────┐
│ UI action                          │ ───────▶ │ aic-api::Engine::dispatch_json         │
│  → app/src/core-api/commands.ts    │  POST    │   Request → handle → Command (undo)    │
│  → transport.ts (Tauri IPC | /api) │  /api    │   → Document regenerate → ChangeSet    │
│ ◀── Response {result, events[] …}  │ ◀─────── │   → Response + CoreEvent[]             │
│  events.ts → sceneSync / cây / 2D  │          └───────────────────────────────────────┘
└────────────────────────────────────┘
```

- **Trình duyệt (dev)**: `crates/aic-dev-server` (tiny_http) giữ một `Engine` duy nhất, nhận `POST /api`
  trên `127.0.0.1:$AIC_PORT` (mặc định 8790; `AIC_ADDR` ghi đè cả địa chỉ, `AIC_STATIC` phục vụ bản build).
  Vite proxy `/api` tới cổng đó (`app/vite.config.ts`).
- **Desktop**: `app/src-tauri` gói cùng `Engine` trong tiến trình, một lệnh IPC `dispatch`.
  `transport.ts` tự chọn: có `__TAURI_INTERNALS__` thì dùng IPC, không thì `fetch('/api')`.
- Request được **xếp hàng tuần tự** phía UI (core đơn luồng). Core chỉ có **một dự án** đang mở.

## Crate

| Crate | Vai trò | File chính |
|---|---|---|
| `aic-math` | Vec/Transform3D (không scale), AABB/OBB, Polygon2D | `transform.rs`, `bbox.rs` |
| `aic-domain` | Mô hình nghiệp vụ, **không** phụ thuộc kernel: tủ, khoang, tấm, feature gia công, sản phẩm, layout generator | `cabinet.rs`, `zone.rs`, `structure.rs`, `layout.rs`, `layout/products.rs`, `product.rs` |
| `aic-parametric` | Biểu thức an toàn, đồ thị tham số, cycle, constraint | `expr.rs`, `graph.rs` |
| `aic-geometry` | Trait `GeometryKernel`, `CsgKernel` (hộp + contour + lỗ → mesh có face/edge id) | `kernel.rs`, `csg.rs`, `panel.rs` |
| `aic-spatial` | Broad phase sort-and-sweep, SAT, snap AABB | `broad.rs`, `snap.rs` |
| `aic-assembly` | Đồ thị quan hệ tiếp xúc giữa tấm (rayon) | `lib.rs` |
| `aic-manufacturing` | Feature suy diễn (khoan liên kết), trải phẳng, Clipper2, G-code, xuất DXF/MPR/CIX | `features.rs`, `cnc.rs`, `export.rs` |
| `aic-nesting` | MaxRects, vân gỗ, nhóm nối vân | `lib.rs` |
| `aic-project` | `Document`, `Command`, `History`, file dự án, `CoreError` (mã ổn định) | `document.rs`, `command.rs`, `history.rs`, `error.rs` |
| `aic-api` | `Engine`: request/response, mọi nghiệp vụ cấp ứng dụng | xem [03-API.md](03-API.md) |
| `aic-dev-server` | HTTP cho trình duyệt | `main.rs` |

Phụ thuộc đi một chiều: `math → domain → (geometry, spatial, parametric) → assembly → manufacturing/nesting → project → api`.

## Quy tắc bắt buộc (lặp lại từ CLAUDE.md, kèm lý do)

1. **Core là nguồn dữ liệu gốc.** UI không tính kích thước, quan hệ, boolean, nesting, G-code. Muốn hiện một con số
   (kể cả khi đang kéo chuột) thì gọi core (`preview`). Lý do: desktop / web / test cùng một kết quả.
2. **Mọi thay đổi là `aic_project::Command`**, `execute` trả lệnh nghịch đảo. Không snapshot cả dự án.
   Nhiều thay đổi cho một thao tác người dùng → `Command::Batch` hoặc `history.mark()` + `squash()` (một undo).
3. **Kích thước là tham số**, không scale mesh; `Transform3D` không có scale.
4. **Feature gia công lưu ở tọa độ local của tấm** (gốc góc tấm, X dài, Y rộng, Z dày).
5. **Domain không biết kernel**: hình học qua trait `aic_geometry::GeometryKernel` (để thay bằng OCCT sau).
6. **Lỗi có `code` ổn định** (`CONSTRAINT_VIOLATED` + `details.constraint`), UI dịch ở `app/src/core-api/errors.ts`.
   Không hiện thông điệp kỹ thuật cho người dùng.
7. **Thêm request**: `protocol.rs` → `lib.rs::handle` → `app/src/core-api/{types,commands,queries}.ts` → `docs/api-mapping.md`.

## Mô hình "tủ là định nghĩa logic"

Tủ (`Cabinet`) **không** lưu danh sách tấm. Nó lưu tham số (W/H/D, dày ván…) + cây khoang (`Zone`) + luật kết cấu
(`StructureRules`) + chỉnh sửa theo tấm (`PartMod`). Mỗi lần đổi, `Document::regenerate_cabinet` chạy
`aic_domain::layout::build` để sinh lại danh sách `Part` và đối chiếu với đối tượng tấm cũ theo **khóa ổn định**
(`Part.key`, ví dụ `c:left`, `z3:shelf1`, `c:island_top`). Tấm giữ nguyên id → UI chỉ cập nhật mesh đổi.
Chi tiết ở [02-CORE.md](02-CORE.md).
