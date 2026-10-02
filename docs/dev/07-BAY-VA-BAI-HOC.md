# 07. Bẫy đã gặp và bài học

## Core

- **Đếm cứng trong test**: thêm mẫu / sản phẩm làm đổi số đếm (ví dụ số mẫu phòng Bếp trong `tests.rs`). Chạy
  `cargo test --workspace` **trước mỗi commit**; đừng commit khi còn đỏ. Mẹo: `out=$(cargo test --workspace 2>&1 | grep -E 'FAILED|panicked')`
  và chỉ commit khi `out` rỗng.
- **Khóa tấm đổi = mất sửa đổi**: `PartMod` gắn theo `Part.key`. Đừng đổi khóa có sẵn khi refactor layout.
- **Trường mới thiếu `#[serde(default)]`** → file dự án / thư viện cũ không mở được.
- **Không kiểm tra khoang** khi đổi kích thước: dùng `edit_cabinet_checked`, không phải `edit_cabinet`.
- **Hộp bao thế giới gồm cả phào / ốp / mặt đá**: `get_bounds` của tủ có phào lớn hơn W×H×D. Test so kích thước tủ
  nên đọc tham số (`get_properties`) hoặc lọc tấm.
- **Đặt vị trí sản phẩm mới**: bàn đảo đặt cách dãy tủ của phòng 1000 mm phía trước để không chồng tủ khác; sản phẩm mới
  cũng phải kiểm tra chồng lấn.
- **Nhóm nối vân rộng hơn khổ ván**: nesting phải xếp rời từng tấm thay vì báo lỗi.
- **Mặc định phải hợp lệ**: vách ốp sâu mặc định 60 vì nhỏ hơn bị `DEPTH_TOO_SMALL`.
- **Luật phòng**: loại đoán theo tên (`room_rules::guess`) chỉ kéo theo luật WC; luật Bếp / Thờ chỉ áp khi người dùng đặt loại phòng rõ ràng (tránh đổi tủ bất ngờ).
- `parse_split_formula` phải nhận `*` đứng riêng (AUTO).
- Clippy phải 0 cảnh báo; `allow` chỉ khi có chú thích lý do (đã có: `large_enum_variant` cho `DomainObject` / `Request`,
  `needless_range_loop` ở `snap.rs`).

## UI / E2E

- **Locator trùng chữ**: `getByText('Khung')` khớp nhiều chỗ → dùng `{ exact: true }`, `.first()`, hoặc thu hẹp theo vùng
  (`page.locator('.panel.drawing')`). Helper `menu()` có thể bấm nhầm dòng gợi ý chứa cùng chữ.
- **Checkbox tự vẽ**: `check()` có thể không bật; dùng `click()` rồi kiểm tra qua `api`.
- **Ctrl+Z mất focus** khi ô nhập đang focus: dùng helper `undo()` (di chuột vào viewport trước).
- **Menu dài bị cắt đáy màn hình**: menu phải có `maxHeight` + cuộn, kẹp trong cửa sổ.
- **Tải file mất tên**: `<a download>` phải gắn vào DOM trước khi click; tên bỏ dấu.
- **Lớp phủ 2D chồng nhau** (khoang trước / sau của bàn đảo): thứ tự vẽ quyết định cái nào nhận click; vẽ cái cần bấm sau cùng.
  Khi click phần tử bị che một phần, dùng `{ force: true }`.
- `vite.config.ts` không có kiểu Node → dùng `declare const process` để đọc `AIC_PORT`.
- Vitest phải giới hạn `src/` (nếu không sẽ chạy nhầm `e2e/*.spec.ts`).
- Khi dùng `sed` sửa nhiều file test cùng lúc, kiểm tra `git diff` để không chèn nhầm dòng vào file khác.

## Môi trường cloud

- Cổng core 8790 (đổi bằng `AIC_PORT`); nếu cổng bận, đặt `AIC_PORT` cho cả core, Vite và Playwright.
- `cargo check` cho `app/src-tauri` cần WebKitGTK; bản desktop chưa được chạy có giao diện.
