# ADR-0002: GeometryKernel và backend CSG thuần Rust

**Bối cảnh.** Đặc tả yêu cầu dùng OpenCASCADE. `opencascade-rs`/`cadrum` phải build OCCT từ mã nguồn
(chậm, phụ thuộc hệ thống), và không được để API OCCT lọt vào domain.

**Quyết định.**
- Domain chỉ biết trait `aic_geometry::GeometryKernel` (make_box, make_cylinder, extrude, transform, cut, fuse,
  intersect, volume, faces, tessellate).
- Backend mặc định là `CsgKernel`: BSP-CSG trên đa giác lồi phẳng, mỗi polygon mang `face_id` để việc chọn mặt
  vẫn hoạt động sau phép boolean. Kết quả chính xác với hộp và lăng trụ; hình trụ được xấp xỉ bằng đa giác (24 cạnh).
- Feature `occt` được giữ chỗ cho `OcctKernel`, sẽ chạy cùng bộ test (thể tích, face id).

**Hệ quả.** Toàn bộ pipeline (tấm có lỗ/hốc/rãnh, tessellation, chọn mặt) chạy được ngay mà không cần OCCT.
STEP/BREP, fillet, sweep, loft vẫn phải chờ backend OCCT.
