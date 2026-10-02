# 08. Việc tiếp theo

Toàn bộ D01–D34 đã xong (xem `docs/STATUS.md`). Dưới đây là việc còn mở, xếp theo giá trị cho xưởng, kèm điểm bắt đầu.
Làm xong việc nào thì xóa / sửa dòng đó và cập nhật `docs/STATUS.md`.

## Ưu tiên 1: chắc chắn chạy được ngoài xưởng

| Việc | Điểm bắt đầu | Ghi chú |
|---|---|---|
| Thử file MPR (Homag) / CIX (Biesse) trên máy thật, sửa theo phản hồi | `crates/aic-manufacturing/src/export.rs` | Cần file mẫu từ xưởng; thêm test so chuỗi với file mẫu |
| Xuất file máy theo sheet nesting (không chỉ theo tấm) | `export.rs` + `aic-nesting` (vị trí, xoay) | |
| Build + chạy bản desktop Tauri | `app/src-tauri`, `npm run tauri build` | Cần WebKitGTK; kiểm tra thư viện `~/.aic-cad/library.json` |
| Đo hiệu năng 1000+ đối tượng / 60 FPS | Viết bench tạo nhiều tủ qua `Engine`; đo `get_render_objects`, regenerate; UI đo FPS | Tối ưu khi có số liệu |

## Ưu tiên 2: sửa trực tiếp trên 2D (thay cho ô nhập số)

Đều theo mẫu đã có ở `features/drawing2d/EditLayer.tsx`: kéo → `preview` → thả → request commit.

| Việc | Dữ liệu core |
|---|---|
| Kéo / vẽ lỗ khoét hậu | `BackRule.cutouts` (`SetBackCutouts`) |
| Kéo vị trí lỗ luồn dây bàn | `DeskSpec` (`desk_*`) |
| Kéo đường chia mô-đun vách ốp | `CladdingSpec` công thức cột × hàng (`cl_*`) |
| Sửa lọt nệm giường trên view Trên | `BedSpec` (`bed_*`) |
| Số đo sửa được: thanh treo, khe tấm lấp, tổng dãy | D03 còn thiếu |

## Ưu tiên 3: tính năng bổ sung

- Ảnh thu nhỏ cho mẫu dựng sẵn (`TemplateGallery.tsx`; có thể render offscreen bằng ViewportEngine).
- Nguồn thư viện nhóm qua URL (HTTP), so sánh phiên bản khi trùng tên (`library_sources.rs`).
- Bảng luật theo loại phòng do người dùng cấu hình (`room_rules.rs`, lưu ở thư viện).
- Catalog hệ ray lùa theo hãng (D16), khoan lắp tay nâng / giá kéo trên hồi (D17, D18), tấm ốp tủ lạnh (D34).
- Phào chạy theo cả dãy tủ (D13 + D09), vát 45° thành contour CNC.
- Mặt cắt trong bản vẽ in, khung tên tùy chỉnh (D29).
- QR trên nhãn, profile CSV cấu hình được (D11).
- Tool 05, 07, 15 chưa làm; chân đế dạng thanh.

## Ưu tiên 4: nền tảng (lớn, cần ADR trước)

- Backend OpenCASCADE (`OcctKernel` sau trait `GeometryKernel`, feature `occt` trong `aic-geometry`), rồi STEP/BREP.
  Xem `docs/adr/0002-geometry-kernel.md`.
- Snap theo đỉnh / cạnh / trung điểm (hiện chỉ AABB), `aic-spatial/src/snap.rs`.
- Nesting theo hình dạng tự do (hiện MaxRects chữ nhật).
- CNC: G-code mặt B / khoan cạnh (hiện cảnh báo, dùng file máy D26 thay thế).
- Catalog vật liệu bằng SQLite.
