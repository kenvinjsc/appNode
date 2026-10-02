# 06. Test

## Ba tầng

| Tầng | Lệnh | Ở đâu | Khi nào |
|---|---|---|---|
| Rust | `cargo test --workspace` | test đơn vị trong từng crate; tích hợp API ở `crates/aic-api/src/tests.rs` | Mọi thay đổi core |
| Vitest | `cd app && npm test` | `app/src/**/*.test.ts` (chỉ `src/`, cấu hình ở `vite.config.ts`) | Hàm thuần UI (phím tắt, số thành chữ, dịch lỗi, gizmo) |
| E2E | `cd app && npm run e2e` | `app/e2e/NN-ten.spec.ts` (Playwright, Chromium) | Mọi tính năng có thao tác người dùng |

Kiểm tra đầy đủ trước khi commit:

```bash
cargo test --workspace 2>&1 | grep -E 'FAILED|panicked|error\[' ; cargo clippy --workspace --all-targets 2>&1 | grep -c warning
cd app && npm test && npm run typecheck && npm run build && npm run e2e
```

## Test Rust qua API

```rust
let mut e = Engine::new();
let r = call(&mut e, json!({"cmd": "create_cabinet", "kind": "WARDROBE"}));
assert!(r.ok, "{:?}", r.error);
let cab: ObjectId = serde_json::from_value(r.result["id"].clone()).unwrap();
let l = e.doc.cabinet_layout(cab).unwrap();          // đọc layout trực tiếp
assert!(l.parts.iter().any(|p| p.name.starts_with("KệDiĐộng")));
assert!(call(&mut e, json!({"cmd": "undo"})).ok);    // luôn kiểm tra undo
```

Kiểm tra lỗi bằng mã: `assert_eq!(r.error.unwrap().details["constraint"], "ZONE_TOO_SMALL")`.

## E2E (Playwright)

- `app/playwright.config.ts`: tự bật core (`cargo run -p aic-dev-server --release`, cổng `AIC_PORT` mặc định 8790) và Vite
  (5173), `reuseExistingServer`. **`workers: 1`**: core có một dự án chung nên test chạy tuần tự.
- WebGL chạy bằng swiftshader. Trong container cloud, Chromium có sẵn ở `/opt/pw-browsers` (biến
  `PLAYWRIGHT_BROWSERS_PATH`); không chạy `playwright install`. Có thể chỉ định `PW_CHROMIUM=/đường/dẫn/chromium`.
- Lần đầu build core release mất vài phút; bật sẵn core ở terminal khác cho nhanh.

Helper (`app/e2e/helpers.ts`):

| Helper | Làm gì |
|---|---|
| `freshSample(page)` | `create_project` + reload → app tự dựng Dự án mẫu (TủQA01, BếpDưới01–03, BếpTrên01). Gọi đầu mỗi test |
| `api(page, body)` | Gọi thẳng core, trả `result` (ném lỗi nếu `ok=false`). Dùng để **chuẩn bị** và **kiểm tra** dữ liệu |
| `tree`, `flatten`, `idOf(name)`, `countNamed(prefix, under?)` | Đọc cây đối tượng |
| `pick(name, add?)` | Bấm tên trong cây (Ctrl để chọn thêm) |
| `menu(name, item)` | Chuột phải tên trong cây → chọn mục |
| `undo(page)` | Ctrl+Z qua giao diện (di chuột vào viewport trước để có focus) |

Quy ước: thao tác người dùng đi qua giao diện thật, kiểm tra kết quả qua `api` (kích thước, số tấm, `get_bounds`).
Đặt tên test theo mã kịch bản `TC-xx.y` trong `docs/KICH-BAN-TEST.md`. Số file tăng dần (`34-…spec.ts` là file tiếp theo).
Dùng `expect.poll(...)` khi chờ core cập nhật, tránh `waitForTimeout` dài.

Chạy một file: `npx playwright test e2e/33-mat-sau.spec.ts`. Báo cáo lỗi: `app/e2e-report/`, ảnh / trace trong `app/test-results/`.
