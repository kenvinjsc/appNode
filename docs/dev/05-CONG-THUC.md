# 05. Công thức cho việc hay gặp

Mỗi công thức liệt kê đúng các file cần đụng, theo thứ tự. Làm xong mỗi bước chạy test của bước đó.

## A. Thêm một request mới

1. `crates/aic-api/src/protocol.rs`: thêm biến thể vào `enum Request` (tag `cmd`, snake_case tự động).
   Trường tùy chọn dùng `#[serde(default)]`. Ghi chú `///` tiếng Việt ngắn.
2. `crates/aic-api/src/lib.rs::handle`: thêm nhánh `match`, gọi hàm trong module phù hợp (đừng nhồi logic vào `lib.rs`).
   Sửa dữ liệu → qua `exec_cmd` / `edit_cabinet_checked`; nhiều bước → `mark` + `squash` / `rollback`.
3. `crates/aic-api/src/tests.rs`: test qua `call(&mut e, json!({...}))` (JSON thật, bắt lỗi serde), kiểm tra cả **undo**.
4. `app/src/core-api/types.ts` (kiểu kết quả), `commands.ts` (sửa) hoặc `queries.ts` (đọc).
5. Nếu có lỗi mới: thêm mã vào `CONSTRAINTS` trong `app/src/core-api/errors.ts`.
6. `docs/api-mapping.md`: thêm một dòng (UI action | request | core | events | UI update).

## B. Thêm một tùy chọn kết cấu tủ (ô trong "Thuộc tính kết cấu")

1. Trường mới trong struct luật (`crates/aic-domain/src/structure.rs`), có `#[serde(default)]` + giá trị mặc định
   **giữ hành vi cũ**.
2. Dùng trong layout (`crates/aic-domain/src/layout.rs`: `carcass`, `zone`, `backs`, `trims`…). Tấm mới cần **khóa mới ổn định**.
3. Field hiển thị: `crates/aic-api/src/structure_api.rs::structure_tabs` (`num` / `flag` / `select` / `text`, khóa có tiền tố
   của nhóm: `back_`, `rt_`, `tr_`, `island_`…).
4. Xử lý khóa: `crates/aic-api/src/zones.rs::set_zone_property` (nhánh tiền tố tương ứng, trong `edit_cabinet_checked`).
   Nếu là chuẩn xưởng `s_*` thì chỉ cần thêm trường vào `ShopRules`: `set_shop_field` tự suy kiểu.
5. Nếu cần trả thêm dữ liệu cho UI: `get_structure`.
6. Test Rust + E2E (mở hộp thoại kết cấu, đổi field, kiểm tra tấm bằng `api(page, …)`).

## C. Thêm một loại tấm / phần tử khoang (link, phụ kiện)

- Phụ kiện catalog: thêm vào `ACCESSORIES` (`crates/aic-domain/src/zone.rs`) (kích thước lọt lòng, ray, giá) → layout `link()` tự kiểm tra vừa khoang,
  báo giá theo mã. Thiết bị: `APPLIANCES` (cùng file).
- Loại link mới: `LinkKind` trong `zone.rs` → nhánh trong `layout.rs::link()` → menu Dựng nhanh (`Chrome.tsx` ZoneMenu)
  gửi `zone_add_link`.

## D. Thêm một sản phẩm không phải tủ hộp (như giường / bàn / vách ốp)

1. `crates/aic-domain/src/product.rs`: spec + biến thể `Product::Xxx`.
2. `crates/aic-domain/src/layout/products.rs`: hàm sinh tấm, gọi từ `products::build`.
3. `crates/aic-api/src/furniture.rs`: nhánh trong `create_furniture` (kích thước mặc định hợp lý, tránh `DEPTH_TOO_SMALL`),
   tiền tố khóa mới trong `set_product_property` + `product_tabs`; thêm tiền tố vào điều kiện đầu `set_zone_property`.
4. Báo giá: `PricingMode::Piece` nếu bán theo chiếc.
5. Menu / mẫu: `products.rs::PRODUCTS` (mẫu dựng sẵn) và/hoặc menu Tủ ▾ trong Ribbon.

## E. Thêm một mẫu dựng sẵn (D30)

`crates/aic-api/src/products.rs`: thêm `Product { key, name, room, summary, size: [W, H, D], params: &[Param…] }` vào `PRODUCTS` và nhánh dựng
trong `insert_product` (chuỗi request có sẵn, tự gộp một undo). UI `TemplateGallery.tsx` tự hiện theo phòng.
Test đếm số mẫu (`tests.rs`) cần cập nhật (ví dụ số mẫu phòng Bếp).

## F. Thêm mã lỗi

Core: `CoreError::ConstraintViolated { constraint: "TEN_MA".into(), message: … }`.
UI: `errors.ts` → `CONSTRAINTS.TEN_MA = 'Câu tiếng Việt nói rõ cách sửa.'`. Có test `errors.test.ts` cho bảng dịch.

## G. Thêm tab / hộp thoại UI

- Trạng thái mở / đóng trong `uiStore.ts` (ví dụ `arrayOf: ObjectId | null`), component trong `features/<nhóm>/`,
  gắn vào `App.tsx` hoặc `Chrome.tsx`. Mục menu chuột phải: `Chrome.tsx`. Phím tắt: `shared/shortcuts.ts` + `app/Shortcuts.tsx`.
- Icon: thêm SVG vào `shared/icons.tsx`.

## H. Thêm định dạng xuất / bản vẽ

- File máy: `crates/aic-manufacturing/src/export.rs` (dữ liệu tấm + feature local → chuỗi), request `export_machine`.
- Bản vẽ in: `crates/aic-api/src/drawing.rs` (chiếu → đoạn thẳng / chữ / kích thước theo mm giấy), UI chỉ vẽ SVG.

## I. Cập nhật tài liệu sau mỗi tính năng

`docs/STATUS.md` (bảng D / phần chưa làm), `docs/api-mapping.md`, `docs/KICH-BAN-TEST.md` (TC mới), nếu là thao tác
người dùng thì `docs/HUONG-DAN-DUNG-TU.md`. Việc còn mở: `docs/dev/08-VIEC-TIEP-THEO.md`.
