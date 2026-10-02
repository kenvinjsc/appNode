# Tài liệu phát triển AIC CAD (cho người / Claude làm tiếp)

Bộ tài liệu này giúp một phiên làm việc mới (người hoặc Claude Code) hiểu mã nguồn và làm tiếp mà không phải đọc lại
lịch sử. Đọc theo thứ tự dưới đây; mỗi file ngắn, có đường dẫn tới mã thật.

| File | Nội dung | Đọc khi |
|---|---|---|
| [01-KIEN-TRUC.md](01-KIEN-TRUC.md) | Bức tranh tổng: crate, luồng UI → core → sự kiện, quy tắc bắt buộc | Luôn đọc đầu tiên |
| [02-CORE.md](02-CORE.md) | Mô hình dữ liệu: Document, Command/History, Cabinet, Zone, StructureRules, layout generator, PartMod | Sửa / thêm hình học, kết cấu tủ |
| [03-API.md](03-API.md) | `Engine`, `Request`/`Response`, sự kiện, mã lỗi, khóa thuộc tính (`set_parameter`), preview, một-undo | Thêm request, thuộc tính |
| [04-UI.md](04-UI.md) | React app: core-api, uiStore, viewport Three.js, 2D, bảng thuộc tính sinh từ core | Sửa giao diện |
| [05-CONG-THUC.md](05-CONG-THUC.md) | Công thức từng bước cho các việc hay gặp (thêm request, thêm tab kết cấu, thêm sản phẩm, thêm mã lỗi…) | Bắt tay làm tính năng |
| [06-TEST.md](06-TEST.md) | Test Rust, vitest, E2E Playwright: helper, quy ước, cách chạy trong môi trường cloud | Trước khi commit |
| [07-BAY-VA-BAI-HOC.md](07-BAY-VA-BAI-HOC.md) | Lỗi đã gặp và cách tránh | Khi test đỏ khó hiểu |
| [08-VIEC-TIEP-THEO.md](08-VIEC-TIEP-THEO.md) | Việc còn mở, xếp theo ưu tiên, kèm điểm bắt đầu trong mã | Chọn việc để làm |

Tài liệu khác (không lặp lại ở đây):

- `CLAUDE.md`: quy tắc kiến trúc ngắn và lệnh chạy.
- `docs/STATUS.md`: đã làm gì (Phase 1–7, D01–D34) và hạn chế đã biết.
- `docs/api-mapping.md`: bảng UI action → request → command → event (mọi request).
- `docs/DE-XUAT-TINH-NANG-V2.md`: đặc tả 34 đề xuất D01–D34 (yêu cầu nghiệp vụ gốc).
- `docs/KICH-BAN-TEST.md`: kịch bản test nghiệp vụ (TC-xx), ánh xạ sang `app/e2e/*.spec.ts`.
- `docs/HUONG-DAN-DUNG-TU.md`: hướng dẫn người dùng dựng tủ.
- `docs/adr/`: quyết định kiến trúc (kernel hình học, spatial, parametric).

## Bắt đầu nhanh (5 phút)

```bash
cargo test --workspace                       # ~126 test core (62 ở aic-api), phải sạch
cargo clippy --workspace --all-targets       # phải 0 cảnh báo
cargo run -p aic-dev-server --release        # core HTTP :8790 (AIC_PORT để đổi)
cd app && npm install && npm run dev         # UI :5173, proxy /api → core
cd app && npm run e2e                        # 50 kịch bản E2E (tự bật core + Vite)
```

Mở http://127.0.0.1:5173: dự án trống sẽ tự dựng **Dự án mẫu** (TủQA01, BếpDưới01–03, BếpTrên01).

## Quy ước làm việc

- Ngôn ngữ: giao diện, thông báo, tài liệu, commit: **tiếng Việt**. Tên kiểu / hàm Rust, TS: tiếng Anh;
  tên tấm sinh ra (hiện cho người dùng) tiếng Việt viết liền: `HồiTrái`, `KệDiĐộng`, `CửaĐôi`, `Hậu`.
- Mỗi tính năng: core + test Rust → request → UI → E2E → cập nhật `docs/api-mapping.md`, `docs/STATUS.md`.
- Trước commit: `cargo test --workspace`, `cargo clippy --workspace --all-targets` (0 cảnh báo),
  `cd app && npm test && npm run typecheck && npm run build`, và E2E liên quan (tốt nhất là cả bộ).
- Không đưa tên model AI vào commit / mã.
