# ADR-0004: Parametric engine

- Biểu thức được tokenize và parse (Pratt) thành AST. Không dùng `eval` chuỗi. Hỗ trợ `+ - * /`, phép so sánh,
  `&& || !`, các hàm `min max clamp if abs round floor ceil sqrt`, hậu tố `mm`, và dấu `=` ở đầu kiểu bảng tính.
- Tham chiếu: `width` hoặc `self.width` (của chính đối tượng), `cabinet.x` (tủ tổ tiên gần nhất), `parent.x`,
  `room.x`, `#12.x` (tuyệt đối). Khi lưu, biểu thức được đổi sang khóa tuyệt đối `#id.name`; nguồn gốc vẫn giữ để
  hiển thị và lưu file.
- Graph: có cạnh ngược (dependents), phát hiện vòng lặp bằng DFS trước khi ghi, và khi tính lại chỉ duyệt tô-pô
  (Kahn) trên phần bị ảnh hưởng.
- Giao dịch: `set_many` sẽ rollback nếu có vòng lặp, nếu khóa được sửa bị lỗi tính toán, nếu một tham số phụ thuộc
  đang đúng bị hỏng, hoặc nếu vi phạm constraint. Constraint cũng là biểu thức (ví dụ `width > 2 * thickness + 10`)
  kèm mã lỗi để UI dịch sang tiếng Việt.
- Tủ: tham số gốc (width, height, …) và tham số suy ra (inner_width, shelf_pitch, …) sinh từ generator. Tấm con
  dùng biểu thức như `cabinet.inner_width`. Đổi kích thước chỉ tính lại tham số; đổi cấu trúc (số đợt, số cánh, …)
  thì sinh lại tủ, và chi tiết được khớp theo (vai trò, chỉ số) nên id và các giá trị người dùng đã sửa vẫn giữ nguyên.
