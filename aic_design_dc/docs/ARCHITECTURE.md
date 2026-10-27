# Kiến trúc AicDesignDC (khảo sát Phase 0, cập nhật theo mã hiện có)

Phạm vi: plugin SketchUp `AicDesignDC` trong `aic_design_dc/` (nhánh `claude/adoring-lamport-hkoxn7`).
Repo còn một ứng dụng riêng "AIC CAD" (Rust + web, nhánh `claude/cloud-deployment-fsttxz`: `crates/`, `app/src/features/cabinet/`). Hai mã nguồn
KHÔNG dùng chung module; tài liệu này chỉ mô tả plugin SketchUp. Chưa có adapter OneClick/Fuji/ABF trong repo (tìm không thấy).

## Lớp
```
SketchUp (Ruby tools/UI)  ──Fiddle──▶  C ABI hsx_*  ──▶  Lõi C++17 (thuần số, không phụ thuộc SketchUp)
```
- Ruby (`ruby/aic_design_dc/`): `main.rb` (menu/toolbar), `draw_tool.rb`, `opening_tools.rb` (Đợt/Vách/Ngăn kéo/Cánh), `option_bar.rb`,
  `dialog.rb/html` (inspector), `layout.rb` (cây khoang), `cluster.rb` (cụm nhiều khối), `bridge.rb` (Builder: dựng/dựng lại hình), `observers.rb` (Scale gốc → tính lại), `library.rb`, `nest_session.rb`.
- Lõi (`core/`): `cabinet.cpp` (Builder đệ quy theo cây khoang), `layout.cpp` (parser cây), `joinery.cpp` (cam/chốt + dán cạnh), `board.cpp`, `parts.cpp`, `nest.cpp`, `export.cpp` (cutlist/DXF/JSON), `capi.cpp`.

## Mô hình ngữ nghĩa (nguồn chân lý)
- `Params` (C++ `cabinet.hpp`; Ruby `params.rb`): rộng/sâu/cao, dày ván/hậu, chân, kiểu hậu (Phủ/Âm), kiểu cánh, khe, hồi lên trước, loại tủ (`cab_type`), treo (`hang_z`), thanh giằng, ray, tấm ốp/đỉnh, cam/dán cạnh…
- **Cây khoang** (`layout`): `L[...]` lá | `V(size:node,..)` vách đứng | `H(size:node,..)` đợt ngang; `size` 0 = tự chia (fill), >0 = cố định.
  Lá: `d` ngăn kéo, `p` cao ngăn, `s` đợt, `l` cánh, `m` kiểu cánh, `h` bản lề, `f` tấm fix, `x` ống hút mùi, `r` bản lề cánh đơn, `g` cánh cách đáy.
- Lưu trong SketchUp: `Group` thuộc tính `hsx/params` (JSON). Hình học (các Group con) được sinh lại từ `Params`, không phải nguồn chân lý.
- Cụm: `Group` thuộc tính `hsx/cluster` (JSON spec: hàng, khối, phào).

## Luồng sửa (giao dịch)
`Builder.rebuild`: `start_operation` → `Core.solve(params mới)` (lỗi thì `abort_operation`, tủ cũ còn nguyên) → `erase!` + dựng lại → `commit_operation` (1 Undo).
Scale gốc: `observers.rb` bắt Scale, đưa transformation về scale 1, đổi rộng/sâu/cao trong tham số, dựng lại (ván giữ độ dày).

## Đơn vị, trục
mm; X rộng, Y sâu (mặt trước y=0), Z cao; tủ dựng trong toạ độ cục bộ của Group; `hang_z` nâng cả tủ.

## Giới hạn đã biết (đối chiếu yêu cầu master prompt)
Xem `FEATURE_MATRIX.md`. Tóm tắt: chưa có ID ổn định cho tấm; chưa có chế độ kích thước ratio/min-max; chưa có neo module (bottom-right);
phụ kiện sinh từ dò hình học chứ chưa có đồ thị quan hệ chủ/khách; chưa có `validateCabinet`; hậu mới có Phủ/Âm (chưa Lọt); lưu chưa có `schema_version`.
