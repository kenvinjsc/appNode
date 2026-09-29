# ADR-0001: Core là nguồn dữ liệu gốc, UI chỉ hiển thị

**Quyết định.** Toàn bộ dữ liệu dự án (domain, tham số, quan hệ, feature) nằm trong `aic-project::Document`
do `aic-api::Engine` quản lý. UI chỉ gửi `Request` (command hoặc query) và nhận `Response` kèm `CoreEvent`.

- Mọi thay đổi đi qua `Command::execute`, lệnh này trả về lệnh nghịch đảo. Undo/redo không chụp lại toàn bộ dự án.
  Các thao tác phá hủy (xóa, đổi cấu trúc tủ) chỉ lưu `Snapshot` của cây con bị ảnh hưởng.
- Transform không có scale. Kích thước tấm luôn lấy từ domain (`width_mm`, `height_mm`, `thickness_mm`),
  không bao giờ suy ra từ ma trận world.
- Mesh chỉ là cache hiển thị, trong không gian định nghĩa của đối tượng, dùng chung theo `geometry_key`.
  Khi di chuyển chỉ gửi ma trận world, không tessellate lại.
- Một scene node ứng với một domain object và dùng chung id (`NodeId = ObjectId`). Id này cũng là render/selection id,
  ổn định trong suốt vòng đời dự án.
- Panel không lưu transform riêng: vị trí là tham số `x/y/z` (có thể là biểu thức) cộng với góc xoay trên node.
  Nhờ vậy không có hai nguồn dữ liệu cho cùng một vị trí.
- Feature suy diễn (khoét bản lề, chốt gỗ, chốt đợt) **không lưu** mà tính lại từ vai trò của tấm và quan hệ lắp ghép.
  Chỉ feature người dùng thêm mới được lưu, và luôn ở tọa độ local của tấm.
