# Kịch bản test AIC CAD

Tài liệu này gom toàn bộ kịch bản kiểm thử theo module: dùng cho test tay trước khi giao bản, và đối chiếu với test tự động
đã có. Mỗi kịch bản có mã `TC-<module>.<số>`; cột **Tự động** ghi tên test Rust (`crates/…/tests.rs`) hoặc test E2E
(`app/e2e/*.spec.ts`). Kịch bản không có test tự động phải chạy tay.

## 1. Cách chạy

| Lớp | Lệnh | Thời gian |
|---|---|---|
| Core (Rust, ~100 test) | `cargo test --workspace && cargo clippy --workspace --all-targets` | ~1 phút |
| UI unit (vitest) | `cd app && npm test && npm run typecheck && npm run build` | ~20 giây |
| E2E (Playwright, trình duyệt thật) | `cd app && npm run e2e` | ~3 phút |

- E2E tự bật `aic-dev-server` (`:8787`) và Vite (`:5173`) nếu chưa chạy. Có Chromium riêng thì đặt
  `PW_CHROMIUM=/đường/dẫn/chrome`. Báo cáo HTML ở `app/e2e-report/`, ảnh / trace khi lỗi ở `app/test-results/`.
- Mỗi test E2E tạo dự án trống qua API rồi nạp trang, app tự dựng **Dự án mẫu** (TủQA01 ở PN1 · Tầng 2; BếpDưới01–03,
  BếpTrên01 ở Bếp · Tầng 1). Core giữ một dự án chung nên E2E chạy tuần tự.
- Quy ước E2E: thao tác người dùng đi qua giao diện; kiểm tra số liệu gọi thẳng core (`/api`). Thao tác làm thay đổi hiển
  thị (undo, chia khoang…) phải đi qua giao diện, vì UI chỉ nhận sự kiện từ request của chính nó.

## 2. Dữ liệu chuẩn cho test tay

| Mã | Dữ liệu | Dùng cho |
|---|---|---|
| D-MAU | Tệp → Dự án mẫu | hầu hết kịch bản |
| D-BEP | 3 tủ bếp dưới 800 + 600 + 900, cao chân 100, 1 tủ trên 1600 | dãy tủ, báo giá mét dài |
| D-TUAO | Tủ áo 1800 × 2400 × 600, 3 khoang, 4 kệ di động, 4 cánh | chốt 32, m² mặt đứng |
| D-LON | Nhân dãy 60 tủ (chuột phải → Nhân dãy tủ sang phải) | hiệu năng |

## 3. Ma trận phủ

| Module | Kịch bản | Tự động | Tay |
|---|---|---|---|
| 01 Dự án, tầng / phòng, lưu / mở, undo | TC-01.1–01.6 | E2E 01, Rust `save_load_identical`, `floors_and_rooms_…` | 01.5, 01.6 |
| 02 Tạo tủ, template, dãy liền nhau | TC-02.1–02.5 | Rust `row_of_cabinets_…`, `parametric_f_…`, E2E 06.4 | 02.1, 02.2 |
| 03 Thao tác 3D (chọn, kéo khối, handle, bắt dính) | TC-03.1–03.7 | Rust `move_objects_…`, `resize_stretch_…`, E2E 06.2–06.3 | 03.1–03.7 |
| 04 Sửa trực tiếp 2D | TC-04.1–04.7 | Rust `parametric_a/b/c/…`, E2E 06.1 | 04.2–04.7 |
| 05 Khoang & tấm (Tạo tấm, Dựng nhanh, Chia khoang) | TC-05.1–05.6 | Rust `zone_workflow_…`, `split_zone_…`, E2E 02 | 05.1, 05.4 |
| 06 Cánh, ngăn kéo, tay nắm, ray, tủ góc | TC-06.1–06.9 | Rust `drawer_heights_…`, `handle_types_…`, `blind_corner_…`, `inner_drawers_…`, E2E 04.2, 08 | 06.1, 06.6 |
| 07 Quan hệ tấm, ràng buộc, offset | TC-07.1–07.4 | Rust `relations_…`, `parametric_d_…` | 07.1–07.4 |
| 08 Tool (bo góc, cắt, hợp, chia, contour) | TC-08.1–08.5 | Rust `shape_tools_…`, `chia_tam_…`, `contour_…` | 08.1–08.5 |
| 09 Kết cấu & chuẩn xưởng | TC-09.1–09.9 | Rust `structure_tabs_…`, `shop_standard_…`, `shelves_snap_…`, `joint_types_…`, `base_types_…`, E2E 03 | 09.1, 09.9 |
| 10 Dãy tủ (mặt đá, len liền, tấm lấp) | TC-10.1–10.5 | Rust `run_countertop_…`, E2E 04.1 | 10.3, 10.5 |
| 11 Vật liệu & dán cạnh | TC-11.1–11.6 | Rust `edge_bands_per_panel_group`, `material_sets_…`, E2E 03.3, 09 | 11.1, 11.4 |
| 12 Sản xuất (gia công, xếp tấm, CNC) | TC-12.1–12.5 | Rust `packs_…`, `program_for_one_part`, E2E 07 | 12.3–12.5 |
| 13 Báo cáo (costing, báo giá, cắt gộp, nhãn) | TC-13.1–13.6 | Rust `costing_report_…`, `quote_…`, E2E 05 | 13.5, 13.6 |
| 14 Lỗi, dữ liệu sai, độ bền | TC-14.1–14.6 | Rust `constraint_violation_…`, `bad_request_…`, E2E 02.2 | 14.4–14.6 |
| 15 Hồi quy (lỗi đã sửa) | TC-15.1–15.8 | xem từng dòng | |

---

## 4. Kịch bản chi tiết

Mẫu: **Tiền điều kiện → Bước → Kết quả mong đợi**. "Một undo" = một lần Ctrl+Z trả lại toàn bộ thao tác.

### 01. Dự án, tầng / phòng, lưu / mở, undo

- **TC-01.1 Khởi động** (E2E 01.1). Mở app lần đầu → Dự án mẫu có đủ 5 tủ, thanh trạng thái "Sẵn sàng", không lỗi trang.
- **TC-01.2 Lọc tầng / phòng** (E2E 01.2). Bấm tab "Tầng 2" → cây và 3D chỉ còn TủQA01; bấm "Mọi tầng" → hiện lại tất cả.
- **TC-01.3 Xóa, undo, redo** (E2E 01.3). Chọn BếpDưới03 → Delete → mất khỏi cây; Ctrl+Z → trở lại đúng vị trí; Ctrl+Y → mất lại.
- **TC-01.4 Lưu / mở** (E2E 01.4, Rust `save_load_identical`). Lưu dự án → mở lại → số đối tượng, kích thước, khoang, dãy, chuẩn
  xưởng giống hệt.
- **TC-01.5 Thêm tầng / phòng** (tay). "+ Tầng", "+ Phòng" → tạo tủ mới khi đang ở phòng đó → tủ nằm trong vùng riêng của phòng,
  tên tự đánh số theo kiểu khung.
- **TC-01.6 Mở tệp hỏng** (tay). Mở một `.json` bất kỳ → thông báo "Không mở được dự án…", dự án đang mở không bị mất.

### 02. Tạo tủ, template, dãy liền nhau

- **TC-02.1 Tạo tủ bằng click** (tay). Tủ ▾ → Tủ bếp dưới → nhấp sàn → popup nhập W/H/D → tủ đúng kích thước, đúng phòng.
- **TC-02.2 TAB tạo tủ nối dãy** (tay). Chọn tủ → TAB → tủ mới liền bên phải, cùng cao / sâu.
- **TC-02.3 Nhân dãy** (Rust `parametric_f_…`). Chuột phải tủ → Nhân dãy tủ sang phải 3 → 3 tủ liền nhau, một undo.
- **TC-02.4 Template** (E2E 06.4). Lưu tủ áo làm template → chèn 1200 × 2200 × 580 → khoang giữ chế độ KHÓA / AUTO / %, cánh,
  kệ tính lại theo kích thước mới.
- **TC-02.5 Lật gương** (Rust `parametric_f_…`). Chuột phải → Lật gương → khoang đảo trái ↔ phải, bản lề đổi phía.

### 03. Thao tác 3D

- **TC-03.1 Chọn hai cấp** (tay). Click tủ → chọn tủ; click lần 2 → chọn tấm; Ctrl+click chọn thêm; kéo khung chọn nhiều.
- **TC-03.2 Kéo khối** (tay; core: Rust `move_objects_by_vector_one_undo`, E2E 06.3). Chọn tủ → bấm giữ thân tủ và kéo → tủ trượt
  trên sàn, bắt dính tủ bên cạnh / tường; đang kéo gõ `300` + Enter → dời đúng 300 mm theo trục đang kéo; Esc hủy; một undo.
- **TC-03.3 Công cụ Di chuyển (M) / Xoay (R)** (tay). Gizmo kéo theo trục, snap; X/Y/Z nhập ở Chỉnh tấm.
- **TC-03.4 Handle kích thước + chế độ dãn** (E2E 06.2, Rust `resize_stretch_…`). Tủ áo 3 khoang → ô Dãn "đều tất cả khoang" → kéo
  rộng 1600 → 1800: mọi khoang tăng theo tỷ lệ; "chỉ khoang sát cạnh kéo": chỉ khoang phía kéo tăng 200.
- **TC-03.5 Bắt dính khi co kéo** (tay). Kéo handle rộng tủ trên tới gần mép tủ bên cạnh / tường → dừng đúng mép, có đường gạch cam;
  giữ Alt → kéo tự do.
- **TC-03.6 Kích thước sửa được trên 3D** (tay). Chọn 1 tủ → bấm số kích thước trên 3D → nhập → tủ đổi, tủ dãy bên phải bị đẩy.
- **TC-03.7 Đổi rộng tủ trong dãy** (Rust `row_of_cabinets_follows_a_width_change`). Tủ giữa dãy 600 → 700 → các tủ bên phải dịch
  100, không chồng nhau.

### 04. Sửa trực tiếp trên bản vẽ 2D

- **TC-04.1 Nhập kích thước khoang** (E2E 06.1). Bấm số khoang → nhập 600 → khoang thành KHÓA 600, khoang AUTO bên cạnh nhận phần còn lại.
- **TC-04.2 Chế độ khoang** (tay; Rust `lock_auto_lock_keeps_locked_bays`). Bấm nhãn KHÓA / % / AUTO để xoay vòng; tủ 1600 → 1800
  với `600 KHÓA · AUTO · 400 KHÓA` → chỉ khoang giữa tăng 200.
- **TC-04.3 Nhập %** (tay; Rust `percent_keeps_ratio_…`). Nhập `40%` → khoang thành %, giữ tỷ lệ khi đổi rộng tủ.
- **TC-04.4 Kéo vách / kệ** (tay; Rust `parametric_b_c_…`). Kéo vách → xem trước số đo 2 khoang kề; thả mới lưu; Shift = bước 10;
  gõ số khi kéo; chỉ 2 khoang kề thay đổi.
- **TC-04.5 Kéo 4 cạnh tấm** (tay; Rust `relations_…_edge_drag`). Chọn tấm → kéo handle cạnh → bắt dính cạnh chi tiết khác; chế độ
  "Giữ ràng buộc" giữ quan hệ.
- **TC-04.6 View Trái / Mặt cắt** (tay). Đổi view → kích thước sâu đúng; mặt cắt kéo thanh vị trí cắt.
- **TC-04.7 Khoang quá nhỏ** (tay; Rust `percent_keeps_ratio_and_conflict_is_reported`). Nhập khoang lớn hơn tủ → từ chối, thông báo
  tiếng Việt, tủ giữ nguyên.

### 05. Khoang & tấm

- **TC-05.1 Tạo tấm theo vùng** (tay; Rust `zone_workflow_tao_tam_chinh_tam`). Tab Tạo tấm → ghim vùng → thêm kệ khóa "cách trên 500"
  → kệ đúng vị trí; Ctrl+click ghim nhiều vùng → thêm vào tất cả.
- **TC-05.2 Chia khoang theo công thức** (E2E 02.1, Rust `split_zone_…`). Phím K → công thức `300`, Trên xuống dưới = Có → bấm khoang
  dưới 2D → khoang trên 300 KHÓA + kệ cố định; Chia dọc `/2`, Tạo tấm = Không → hai khoang con nét đứt, không sinh tấm.
- **TC-05.3 Công thức** (Rust `split_zone_…`). `500,300` · `30%,*` · `3*400` · `/3` đều cho số khoang đúng; `abc` → báo lỗi công thức.
- **TC-05.4 Chia khoang trong 3D** (tay). Chọn tủ → K → bấm khoang trong 3D (khoang sáng lên khi rê) → chia đúng khoang đó.
- **TC-05.5 Gộp khoang** (tay). Chuột phải khoang → Gộp với khoang kế bên → mất đường chia, khoang con gộp lại, một undo.
- **TC-05.6 Mẫu vùng** (Rust `zone_preset_…`). Lưu vùng có vách + kệ + cánh + thanh treo → áp vào tủ khác → nội dung giống.

### 06. Cánh, ngăn kéo, tay nắm, ray, tủ góc

- **TC-06.1 Cánh đôi / lùa / lật** (tay). Dựng nhanh → Cánh đôi, Cửa lùa 2 cánh, Cánh lật → hình và số bản lề đúng.
- **TC-06.2 Khe từng phía, cao từng ngăn** (Rust `drawer_heights_and_door_side_gaps`). Khe trái 3, phải 1 → kích thước cánh đúng;
  ngăn kéo KHÓA 150 + AUTO.
- **TC-06.3 Vị trí tay nắm theo loại tủ** (Rust `handle_types_…`). Bếp dưới → tay nắm nửa trên cánh; bếp trên → nửa dưới.
- **TC-06.4 Loại tay nắm** (Rust `handle_types_…`, E2E 03.2). Push-open → không còn tay nắm, báo giá "Nhấn mở"; Núm → 1 lỗ.
- **TC-06.5 Loại ray** (Rust `handle_types_…`). Ray âm → hở hông 5, mã RAYAM; Tandem → không còn thành hộc gỗ, chỉ đáy + hậu hộc.
- **TC-06.6 Tủ góc L mù** (E2E 04.2, Rust `blind_corner_…`). Tủ ▾ → Tủ góc L mù (góc phải) → tấm mù bên phải không bản lề / tay
  nắm, cánh 450 bên trái; khe giữa tấm mù và cánh ≈ khe cánh; cùng phòng với tủ đang chọn.
- **TC-06.8 Ngăn kéo trong** (E2E 08, Rust `inner_drawers_behind_doors_and_false_front`). Chuột phải khoang → Ngăn kéo trong × 2 →
  2 MặtNgănTrong nằm sau cánh ≥ 20 mm, không tay nắm, vẫn có hộc; một undo.
- **TC-06.9 Mặt giả tủ chậu** (E2E 08). Chuột phải khoang → Mặt giả → chỉ MặtGiả, không hộc / ray / tay nắm.
- **TC-06.7 Cánh rộng sai** (Rust `blind_corner_…`). Tủ góc 600 với cánh 500 → từ chối, thông báo tiếng Việt.

### 07. Quan hệ tấm, ràng buộc, offset

- **TC-07.1 Lọt / Phủ / Bằng mặt / Khe** (Rust `relations_overlay_inset_gap_flush_and_edge_drag`). Chọn 2 tấm → chuột phải → mỗi
  kiểu → kích thước tấm 1 đổi đúng.
- **TC-07.2 Ràng buộc bám mặt tấm khác** (Rust `parametric_d_dynamic_anchor_…`). Kệ.Phải → HồiPhải.Mặt trong → dời hồi → kệ theo.
- **TC-07.3 Offset / co giãn** (tay). Co giãn trên +20 → tấm dài thêm 20, một undo.
- **TC-07.4 Xem quan hệ** (tay). Shift+R → đường nối quan hệ hiển thị đúng cặp tấm.

### 08. Tool

- **TC-08.1 Bo / vát góc** (Rust `shape_tools_…`). Tool bo góc R50 → đường bao đúng, CNC có contour.
- **TC-08.2 Cắt tự do, cắt theo tấm** (Rust `cut_line_…`, `cut_by_box_…`). Tấm bị cắt giữ phần lớn hơn.
- **TC-08.3 Hợp tấm** (Rust `shape_tools_corner_cut_merge`). 2 tấm cùng mặt phẳng, cùng dày → thành 1 tấm; khác dày → từ chối.
- **TC-08.4 Chia tấm** (Rust `chia_tam_splits_a_part_into_pieces`). Chia 3, khe 3 → 3 tấm đúng kích thước.
- **TC-08.5 Contour cung / đa giác** (Rust `contour_arc_and_free_polygon_on_a_part`). Đa giác tự cắt → báo lỗi tiếng Việt.

### 09. Kết cấu & chuẩn xưởng

- **TC-09.1 Tab kết cấu** (tay; Rust `structure_tabs_…`). Chuột phải tủ → Thuộc tính kết cấu → đủ tab: Thông số chung, Liên kết, Hậu,
  Thanh giằng, Chân / treo, Kệ & chốt tầng, Cánh & tay nắm, Dán cạnh, Ngăn kéo; sửa ô nào → tủ đổi ngay, một undo.
- **TC-09.2 Hàng lỗ hệ 32** (E2E 03.1, Rust `shelves_snap_to_32mm_pin_row`). Lỗ chốt tầng = Hàng lỗ hệ 32 → hồi có hàng lỗ Ø5 bước 32;
  kệ di động nằm trên bội 32 (lỗ đầu 64 + 5); kéo kệ tới số lẻ → bắt lỗ gần nhất.
- **TC-09.3 Bảng số bản lề** (Rust `shop_standard_…`). `900=2,1600=3,1800=4,5` → cánh 2300 có 5 bản lề; báo giá = số chén.
- **TC-09.4 Đế bản lề, lỗ tay nắm** (Rust `handle_types_…`). Bật → hồi có 2 lỗ Ø5 mỗi bản lề; cánh có 2 lỗ tay nắm.
- **TC-09.5 Liên kết thùng** (Rust `joint_types_…`, E2E 03.2). Cam + chốt → báo giá có "Cam", lỗ cam Ø15 trên tấm nóc / đáy; Vít →
  "Vít liên kết"; Ke → "Ke góc", không lỗ; undo trả lại.
- **TC-09.6 Chân / treo** (Rust `base_types_…`). Len 3 mặt (đáy phủ) → ChânHôngTrái / Phải; Chân nhựa → 6 chân cho tủ 900; Tủ treo
  → 2 ke treo + thanh treo tường.
- **TC-09.7 Ngăn kéo sâu** (tay). Cao hộc tối đa 350 → ngăn nồi 400 có hộc 350 (không bị cắt ở 250).
- **TC-09.8 Lưu / áp chuẩn xưởng** (E2E 03.1). Lưu chuẩn "Xưởng A" → chọn 3 tủ khác → Áp → cả 3 đổi, kích thước tủ không đổi,
  một undo; chuẩn có trong thư viện ở dự án khác.
- **TC-09.9 Tủ cũ không đổi** (tay). Mở dự án lưu trước bản chuẩn xưởng → hình, danh sách cắt giống trước (trừ tay nắm tự đặt
  theo loại tủ).

### 10. Dãy tủ

- **TC-10.1 Tạo dãy** (E2E 04.1, Rust `run_countertop_…`). Chọn 3 tủ bếp dưới → Tạo dãy tủ → MặtĐá dài = tổng rộng (+ tấm lấp),
  sâu = sâu tủ + nhô trước 20, dày 20, vật liệu đá.
- **TC-10.2 Tự cập nhật** (E2E 04.1). Đổi rộng tủ giữa dãy +100 → mặt đá +100, tủ bên phải dịch; một undo trả cả hai.
- **TC-10.3 Len chân liền, tấm lấp, che trần** (tay). Bật len liền → tủ trong dãy mất len riêng, có LenChânDãy; tấm lấp phải 50;
  cao độ trần 2700 cho dãy tủ trên → CheTrần cao đúng.
- **TC-10.4 Khoét chậu / bếp** (E2E 04.1, Rust `run_countertop_…`). + Khoét chậu → mặt đá có lỗ bo góc; sửa "cách mép trái" → lỗ
  dời; lỗ nằm ngoài mặt đá → bỏ qua.
- **TC-10.5 Xóa dãy** (tay). Xóa dãy → mất mặt đá / len liền / tấm lấp, tủ có lại len riêng, một undo.

### 11. Vật liệu & dán cạnh

- **TC-11.1 Vật liệu tủ / tấm** (tay). Đổi vật liệu thùng / cánh của tủ → danh sách cắt, báo giá theo vật liệu mới.
- **TC-11.2 Dán cạnh theo nhóm** (E2E 03.3, Rust `edge_bands_per_panel_group`). Cánh ABS 2 mm toàn bộ → kích thước cắt cánh trừ 4 mm
  mỗi chiều, báo giá có dòng ABS-2.
- **TC-11.3 Hậu không dán** (Rust `edge_bands_per_panel_group`). Hậu 9 mm → không dán (theo vai trò, không theo độ dày).
- **TC-11.5 Vật liệu Việt Nam** (E2E 09). Thư viện có MFC lõi xanh (khổ 2440 × 1830), Acrylic, Laminate, Veneer; mở dự án cũ vẫn có.
- **TC-11.6 Bộ vật liệu cả phòng** (E2E 09, Rust `material_sets_apply_to_a_room_one_undo`). Bảng kết cấu → Bộ vật liệu "Bếp chống ẩm"
  → Cả phòng → mọi tủ phòng Bếp đổi thùng MFC lõi xanh, cánh Acrylic, hậu HDF lõi xanh, chỉ ABS; phòng khác không đổi; một undo.
- **TC-11.4 Ghi đè cạnh** (tay). Chỉnh tấm → bật / tắt dán từng cạnh → chỉ tấm đó đổi.

### 12. Sản xuất

- **TC-12.1 Trải phẳng** (E2E 07.1). Hồi tủ áo → bản vẽ mặt A / B có lỗ, rãnh hậu đúng tọa độ local.
- **TC-12.2 Xếp tấm** (E2E 07.2, Rust `packs_without_overlap`). Không chồng, không tấm nào ngoài khổ; tấm quá khổ → báo "không xếp được".
- **TC-12.3 G-code** (E2E 07.2, Rust `program_for_one_part`). Sinh chương trình có G0 / G1, thời gian ước tính; tải về được.
- **TC-12.4 Mô phỏng CNC** (tay). Play / pause / step đường chạy dao đúng thứ tự dụng cụ.
- **TC-12.5 Hàng lỗ 32 trong gia công** (tay). Bật ROW_32 → hồi có hàng lỗ; 3D vẫn mượt (lỗ chỉ vẽ vòng tròn).

### 13. Báo cáo

- **TC-13.1 Costing** (Rust `costing_report_wardrobe`). Tấm theo vật liệu × dày (m²), dán cạnh theo mã (m), phụ kiện (bản lề, ray…);
  sửa đơn giá → tổng đổi, một undo.
- **TC-13.2 Báo giá** (E2E 05.1, Rust `quote_linear_facade_and_cut_groups`). Bếp theo mét dài (2.3 m × 4.5 tr), tủ áo theo m² mặt đứng
  (4.32 m² × 3.2 tr); đổi VAT 8 → 10 → tổng tăng; đổi cách tính 1 tủ sang Bóc chi tiết.
- **TC-13.3 Danh sách cắt gộp** (E2E 05.2). Bật "Gộp tấm giống nhau" → ít dòng hơn, SL gộp, danh sách mã tấm.
- **TC-13.4 CSV** (E2E 05.2). Xuất CSV (gộp / không gộp) mở được bằng Excel, tiếng Việt đúng dấu.
- **TC-13.5 In nhãn** (tay). In nhãn → cửa sổ in A4, nhãn 60 × 40 có mã, tên, kích thước cắt, vật liệu, ký hiệu cạnh dán tô cam.
- **TC-13.7 In báo giá / PDF** (E2E 05 `TC-13.7`, vitest `numberWords`). Nhập khách hàng → In báo giá → trang A4 có tên khách,
  2 nhóm phòng, tổng bằng tổng core, dòng "Bằng chữ … đồng"; xuất PDF > 10 KB.
- **TC-13.6 Cabinet List** (tay). Bấm dòng → chọn tủ trong 3D.

### 14. Lỗi, dữ liệu sai, độ bền

- **TC-14.1 Ràng buộc kích thước** (Rust `constraint_violation_is_rejected`). Rộng nhỏ hơn 2 hồi → từ chối, thông báo tiếng Việt.
- **TC-14.2 Request sai** (Rust `bad_request_is_reported`). Core trả `code` ổn định, UI không hiện chữ kỹ thuật.
- **TC-14.3 Công thức vượt khoang** (E2E 02.2). Chia 5000 → toast "Không thể…", khoang giữ nguyên.
- **TC-14.4 Mất kết nối core** (tay). Tắt `aic-dev-server` → thông báo "Không kết nối được lõi CAD…".
- **TC-14.5 Hiệu năng** (tay, D-LON). 60 tủ: xoay / zoom mượt; đổi rộng 1 tủ < 1 giây; mở báo cáo < 5 giây.
- **TC-14.6 Undo dài** (tay). 30 thao tác liên tiếp → undo hết về trạng thái đầu, redo lại đủ.

### 15. Hồi quy (lỗi đã sửa, không được tái diễn)

| Mã | Lỗi cũ | Kiểm tra |
|---|---|---|
| TC-15.1 | Nhập kích thước trong khoang % làm dãn mọi khoang | Rust `typed_size_in_percent_split_goes_to_the_neighbour` |
| TC-15.2 | Chia khoang ảo làm 2 cánh kề nhau phủ chồng | Rust `blind_corner_…` (khe giữa tấm mù và cánh) |
| TC-15.3 | Bật hàng lỗ 32 làm core treo khi dựng hình | TC-12.5; E2E 03.1 (giao diện vẫn phản hồi) |
| TC-15.4 | Báo giá tính cam nhưng không có lỗ cam | Rust `joint_types_…` |
| TC-15.5 | Hai luật số bản lề khác nhau | Rust `shop_standard_…` (số chén = số bản lề báo giá) |
| TC-15.6 | Tay nắm luôn ở giữa cánh | Rust `handle_types_…` |
| TC-15.7 | Hậu không phải 8.6 mm vẫn bị dán cạnh | Rust `edge_bands_per_panel_group` |
| TC-15.8 | Tủ góc tạo cạnh tủ không vào cùng phòng | E2E 04.2 |

## 5. Quy trình trước khi giao bản

1. `cargo test --workspace`, `cargo clippy --workspace --all-targets` không có cảnh báo mới.
2. `cd app && npm test && npm run typecheck && npm run build`.
3. `cd app && npm run e2e` — tất cả qua.
4. Chạy tay các kịch bản cột **Tay** của module có thay đổi, cộng TC-01.5, TC-03.2, TC-04.4, TC-13.5, TC-14.5.
5. Ghi kết quả (bản, ngày, người test, kịch bản lỗi + ảnh) vào mô tả bản giao.
