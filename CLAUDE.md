# AIC CAD: hướng dẫn cho Claude Code

Phần mềm CAD nội thất gỗ. Đặc tả gốc: UI/UX + 3D viewer và CAD Core (Rust). Trạng thái & việc tiếp theo: `docs/STATUS.md`.

## Quy tắc kiến trúc (bắt buộc)
- Core (`crates/`) là nguồn dữ liệu gốc. UI (`app/`) chỉ gửi `Request` qua `app/src/core-api` và vẽ kết quả.
  Không đặt logic CAD (boolean, quan hệ, nesting, G-code, tính kích thước) trong React/Three.js.
- Mọi thay đổi dữ liệu là một `aic_project::Command` (execute trả về lệnh nghịch đảo). Không snapshot toàn dự án.
- Không đổi kích thước bằng scale mesh; kích thước là tham số. Transform không có scale.
- Feature gia công lưu ở tọa độ local của tấm.
- Domain không phụ thuộc kernel: hình học đi qua trait `aic_geometry::GeometryKernel`.
- Lỗi core có `code` ổn định; UI dịch trong `app/src/core-api/errors.ts`, không hiển thị lỗi kỹ thuật cho người dùng.
- Thêm request mới: `crates/aic-api/src/protocol.rs` → `lib.rs::handle` → `app/src/core-api/{types,commands,queries}.ts`
  → cập nhật `docs/api-mapping.md`.

## Lệnh
```bash
cargo test --workspace && cargo clippy --workspace --all-targets
cargo run -p aic-dev-server --release          # core cho trình duyệt, :8790 (AIC_PORT)
cd app && npm run dev                          # UI :5173 (proxy /api)
cd app && npm test && npm run typecheck && npm run build
cd app && npm run e2e                          # kịch bản E2E (docs/KICH-BAN-TEST.md), tự bật core + Vite
cd app/src-tauri && cargo check                # vỏ desktop (cần WebKitGTK)
```
UI viết tiếng Việt, gọn, sáng, nhấn màu cam `#e8590c`, icon SVG trong `app/src/shared/icons.tsx` (không dùng emoji).
