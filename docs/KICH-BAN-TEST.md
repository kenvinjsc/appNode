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
| D-LON | Nhân dãy 60 tủ (chuột phải → Nhân dãy tủ…) | hiệu năng |

## 3. Ma trận phủ

| Module | Kịch bản | Tự động | Tay |
|---|---|---|---|
| 01 Dự án, tầng / phòng, lưu / mở, undo | TC-01.1–01.6 | E2E 01, Rust `save_load_identical`, `floors_and_rooms_…` | 01.5, 01.6 |
| 02 Tạo tủ, template, dãy liền nhau, mẫu dựng sẵn | TC-02.1–02.6 | Rust `row_of_cabinets_…`, `parametric_f_…`, E2E 06.4 | 02.1, 02.2 |
| 03 Thao tác 3D (chọn, kéo khối, handle, bắt dính) | TC-03.1–03.7 | Rust `move_objects_…`, `resize_stretch_…`, E2E 06.2–06.3 | 03.1–03.7 |
| 04 Sửa trực tiếp 2D | TC-04.1–04.7 | Rust `parametric_a/b/c/…`, E2E 06.1 | 04.2–04.7 |
| 05 Khoang & tấm (Tạo tấm, Dựng nhanh, Chia khoang) | TC-05.1–05.6 | Rust `zone_workflow_…`, `split_zone_…`, E2E 02 | 05.1, 05.4 |
| 06 Cánh, ngăn kéo, tay nắm, ray, tủ góc | TC-06.1–06.10 | Rust `drawer_heights_…`, `handle_types_…`, `blind_corner_…`, `inner_drawers_…`, `diagonal_corner_…`, E2E 04, 08 | 06.1, 06.6 |
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
- **TC-02.3 Nhân dãy** (Rust `parametric_f_…`). Chuột phải tủ → Nhân dãy tủ… → 3 → 3 tủ liền nhau, một undo.
- **TC-02.4 Template** (E2E 06.4). Lưu tủ áo làm template → chèn 1200 × 2200 × 580 → khoang giữ chế độ KHÓA / AUTO / %, cánh,
  kệ tính lại theo kích thước mới.
- **TC-02.6 Mẫu bếp dựng sẵn** (E2E 11, Rust `kitchen_products_insert_with_one_undo`). Tủ ▾ → Tủ lò 600 kịch trần cạnh BếpDưới03 →
  600 × 2300 × 580, phòng Bếp · Tầng 1, 2 ngăn kéo + khoang lò 600 + cánh lật; bếp dưới 800 có 6 chân, vật liệu MFC lõi xanh;
  bếp trên có ke treo, tay nắm nửa dưới; một undo.
- **TC-02.7 Giường** (E2E 18, Rust `bed_generator_frame_slats_beam_and_side_drawers`). Tủ ▾ → Giường 1600 × 2000 → đầu /
  đuôi rộng 1600, vai dài 2000 + 2 × dày ván, 14 nan dát, đà giữa, 6 chân. Thuộc tính kết cấu → Giường → Hộc kéo 2 bên → 4 hộc,
  ray ≤ nửa rộng giường, chân còn 4 góc. Đổi rộng 1800 → mọi chi tiết giải lại. Báo giá: "Theo chiếc". Rộng nệm 3000 → báo lỗi.
- **TC-02.8 Bàn** (E2E 19, Rust `desk_with_drawer_unit_hutch_and_cable_hole`). Tủ ▾ → Bàn học 1200 → mặt 1200 × 600 dày 25,
  khoét luồn dây Ø60, hộc phải 3 ngăn rộng 400, kệ trên 2 tầng, yếm. Đổi rộng 1400 → hộc vẫn 400 (khóa). Đỡ trái = Chân sắt → 2
  chân, mất chân tấm.
- **TC-02.9 Vách ốp / lam** (E2E 20, Rust `wall_cladding_modules_follow_width_and_battens`). Tủ ▾ → Vách TV 3000 × 2700 →
  5 tấm (3000 − 4 × 3) / 5 × 2700; rộng 3200 → tấm giãn đều; tab Vách ốp → Chia cột `/6` → 6 tấm; công thức sai → báo lỗi.
  Khoét hộp điện (danh sách Khoét như tab Hậu) → lỗ trên đúng tấm chứa tâm. Vách lam 1200 → lam 40 / khe 25 trên khung xương.
  Báo giá: m² mặt đứng.
- **TC-02.10 Mẫu dựng sẵn** (E2E 26, Rust `builtin_templates_insert_with_params_one_undo_each`). Tủ ▾ → Mẫu dựng sẵn → Phòng ngủ →
  Tủ áo 4 cánh 1800 → Rộng 2000, bật Kịch trần (trần 2700, che trần 50) → Chèn: tủ 2000 × 2650, 2 khoang đều, tầng trên 400,
  4 cánh dưới rộng đều, 2 khoang treo, 6 ngăn kéo trong; Ctrl+Z bỏ cả mẫu. Chèn lần lượt cả 14 mẫu: không mẫu nào lỗi khoang.
- **TC-02.11 Bàn đảo** (E2E 27, Rust `island_front_drawers_rear_doors_face_back_no_back_panel`). Mẫu dựng sẵn → Bàn đảo 1800 →
  không tấm hậu, vách giữa (HậuPhụ) chia khoang trước / sau; ngăn kéo khoang trước ở mặt trước, cánh khoang sau quay ra sau;
  mặt đá nhô 300 phía ghế; ốp hông 2 bên; đứng trước dãy bếp (không chồng tủ).
- **TC-02.12 Nhân dãy tủ** (E2E 29, Rust `array_cabinet_by_size_formula_on_any_axis`). Chuột phải BếpTrên01 → Nhân dãy tủ… →
  Kích thước từng tủ `400,600,800` → 3 tủ mới liền bên phải, rộng đúng thứ tự; Ctrl+Z bỏ cả 3. Trục Y (chồng lên) khe 10 được;
  công thức sai → báo lỗi.
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
- **TC-03.8 Căn chỉnh** (E2E 14, Rust `align_distribute_rotate_and_snap_to_wall`). 3 tủ lệch mặt trước → Căn trước → cùng mặt trước,
  một undo; Chia đều → khe bằng nhau; Xoay 90° → rộng / sâu đổi chỗ; Sát tường khe 5 → cách tường sau 5.
- **TC-03.9 Phào & ốp** (E2E 17, Rust `cornice_end_panels_and_scribe_follow_the_cabinet`). Thuộc tính kết cấu → Phào & ốp → Mặt có
  phào = 3 mặt → 3 thanh PhàoNóc (tên có "vát 45°"); rộng 1500 → 1600 thì phào trước dài thêm 100, phào hông không đổi. Ốp hông
  trái chạm sàn, nẹp trái 40 → phào trước dài thêm dày ốp + 40. Một undo bỏ từng thay đổi.
- **TC-03.7 Đổi rộng tủ trong dãy** (Rust `row_of_cabinets_follows_a_width_change`). Tủ giữa dãy 600 → 700 → các tủ bên phải dịch
  100, không chồng nhau.
- **TC-03.8 Khoét hậu, hậu ốp** (E2E 16, Rust `back_cutouts_overlay_back_and_split_at_fixed_shelves`). Thuộc tính kết cấu → Hậu →
  + Ống nước Ø60 → lỗ tròn giữa hậu (CNC: đường cắt trong); đổi rộng tủ → lỗ vẫn giữa. Bật Hậu ốp bắt vít → hậu rộng bằng tủ,
  hồi ngắn lại một độ dày hậu, báo giá có "Vít bắt hậu". Hậu lọt + Hậu chia theo kệ cố định + 1 kệ cố định → 2 tấm hậu, kệ
  chạy tới mép sau.

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
- **TC-04.8 Xem trước do core** (E2E 12, Rust `preview_is_a_dry_run_without_history_or_revision`). Bật hàng lỗ 32 → kéo kệ trên 2D →
  số đang kéo là vị trí đã bắt lỗ (bội 32 + 69), revision không đổi khi đang kéo, redo còn nguyên; thả mới lưu; kéo quá khoang → tay nắm đỏ.
- **TC-04.9 Số đo màu cam** (E2E 13, Rust `edit_dims_are_computed_by_core_and_editable`). View Bên: bấm Sâu 600 → 580 → tủ sâu 580;
  view Trước: tay nắm cách đầu cánh 60 → 80; hai undo trả lại.
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
- **TC-05.9 Kệ giày nghiêng** (E2E 15, Rust `tilted_shoe_shelves_and_partial_divider`). Chuột phải khoang TủQA01 → Kệ giày nghiêng
  15° × 4 → thêm 4 ThanhChặnGót, kệ xoay [-75,0,0], không chốt kệ; Ctrl+Z bỏ cả nhóm. Đặt "Nghiêng trước-sau" = 0 cho một kệ →
  mất thanh chặn của kệ đó. Vách ngăn đặt "Vách lửng: cao" = 400 → vách cao 400 từ đáy (−400: treo từ nóc).
- **TC-05.10 Phụ kiện khoang** (E2E 23, Rust `accessories_check_the_zone_and_warn_without_blocking_resize`). Tủ 600 → chuột phải
  khoang → Giá bát 800 → vẫn thêm, thông báo "không vừa khoang", khoang tô đỏ trên 2D. Đổi rộng 800 (lọt lòng 765,6) → hết đỏ;
  báo giá có "Giá bát đĩa 800" + 1 bộ ray. Thu nhỏ lại không bị chặn, chỉ đỏ lại. Đèn LED: mét dài + nguồn. Mã lạ → từ chối.
- **TC-05.11 Khoang thiết bị** (E2E 24, Rust `appliance_bay_oven_with_support_vent_and_fit_check`). Tủ 600 × 2300 chia `720,610,*`
  → khoang giữa + Khoang lò 600 → thanh đỡ thiết bị, khe thoát nhiệt khoét trên hậu, lò 595 hiển thị. Khoang 720 + Tủ lạnh âm →
  "Khoang nhỏ hơn kích thước lọt lòng của thiết bị", không đổi dữ liệu. Mẫu Tủ lò 600 kịch trần có sẵn khoang lò.

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
- **TC-06.10 Tủ góc chéo** (E2E 04, Rust `diagonal_corner_…`). Tủ ▾ → Tủ góc chéo (bếp dưới) → đáy / nóc / kệ 5 cạnh, 2 hậu,
  cánh xiên 45° rộng √2 × (900 − 580) − 2 khe; tab Tủ góc chéo đổi số kệ; sâu tay ≥ rộng − 150 → từ chối.
- **TC-06.11 Cánh lùa theo hệ ray** (E2E 21, Rust `sliding_doors_follow_the_track_system_with_alu_frame_and_glass`). Tủ 2400,
  3 cánh lùa, chồng 35 → mỗi cánh (rộng ray + 2 × 35) / 3, xen kẽ 2 ray. Chỉnh tấm cánh → Khung = Nhôm bản 45 → 4 thanh khung / cánh;
  Ô nhét = Gương → 3 tấm gương (phụ kiện, không vào xếp tấm); báo giá có profile nhôm (m) và kính (m²). Chọn thanh khung vẫn sửa
  được hệ ray.
- **TC-06.12 Cánh lật / gập / kính** (E2E 22, Rust `lift_up_door_hk_on_wall_cabinet_and_height_check`). Tủ trên 800 × 400 →
  chuột phải khoang → Cánh lật tay nâng HK → 1 cánh, 2 lỗ chén ở mép trên, 1 bộ tay nâng HK (lực S/M/L theo khối lượng cánh)
  vào báo giá. Khoang cao 300 → "Khoang quá thấp … cho tay nâng", không đổi dữ liệu. Cánh gập 2 lá (HF) + khung nhôm bản 20 →
  2 lá × 4 thanh khung.
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
- **TC-08.6 Tool theo tham số** (E2E 10, Rust `tool_features_are_parametric_and_one_undo`). Khấu góc TR 100 × 100 trên 2 hồi →
  một undo; đổi sâu tủ 560 → 600 → khấu vẫn ở góc trên phải; khấu bề mặt neo góc phải / trên; rãnh LED tính từ mép cuối.
- **TC-08.7 Xuất file máy** (E2E 28, Rust `export_machine_files_dxf_layers_mpr_cix`). Tủ liên kết cam + chốt → chọn hồi → Chỉnh
  tấm → Gia công → Xuất DXF → tải file `.dxf` (tên không dấu). DXF có layer `CUT`, `DRILL_15_12.5` (cam), `DRILL_B_…` (mặt B, đã
  lật), `HDRILL_8_25` (chốt khoan cạnh). MPR có `_BSX=` / `<102 \BohrVert\`, CIX có `BEGIN MAINDATA`, `BV`, `BH`.
- **TC-08.8 Nối vân** (E2E 31, Rust `grain_matched_doors_are_nested_side_by_side`, nesting `grain_group_parts_are_placed_adjacent_in_order`).
  Chọn 2 cánh (Ctrl+click) → chuột phải → Nối vân 2 tấm → Xếp tấm: 2 cánh liền nhau (cách đúng khoảng dao), cùng tấm ván, cùng
  hướng, theo thứ tự trái → phải. Nhóm quá khổ ván → xếp rời, không mất tấm. Khác vật liệu → báo lỗi. Một undo bỏ nhóm.
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
- **TC-12.6 Thư viện nhóm** (E2E 30, Rust `team_library_shared_folder_between_two_machines`). Máy A: Cài đặt → Thư viện nhóm → thêm
  nguồn "Công ty" (thư mục chung) → lưu chuẩn "MFC 18" → Thuộc tính kết cấu → Đẩy lên nhóm. Máy B trỏ cùng thư mục (chỉ đọc) →
  thấy và áp được "MFC 18"; file thư viện máy B không chép mục nguồn; đẩy từ máy B → "Nguồn thư viện này chỉ đọc".

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
- **TC-13.8 Bản vẽ in** (E2E 25, Rust `drawing_sheet_kitchen_elevation_on_one_a3`). Chọn một tủ phòng Bếp → nút máy in trên thanh 2D
  → A3 ngang, Mặt đứng + Mặt bằng → 1 trang: mặt đứng tủ dưới + tủ trên (nét đứt chiều mở cánh), chuỗi rộng từng tủ + tổng
  (= tổng rộng tủ dưới), cao từng loại tủ, mặt bằng có tên tủ, khung tên (dự án, phòng, người vẽ, ngày, tỷ lệ chuẩn, trang).
  Bật Chi tiết từng tủ → mỗi tủ mặt trước (ẩn cánh) + mặt bên, tự thêm trang, cùng tỷ lệ.
- **TC-13.6 Cabinet List** (tay). Bấm dòng → chọn tủ trong 3D.

### 14. Lỗi, dữ liệu sai, độ bền

- **TC-14.1 Ràng buộc kích thước** (Rust `constraint_violation_is_rejected`). Rộng nhỏ hơn 2 hồi → từ chối, thông báo tiếng Việt.
- **TC-14.2 Request sai** (Rust `bad_request_is_reported`). Core trả `code` ổn định, UI không hiện chữ kỹ thuật.
- **TC-14.3 Công thức vượt khoang** (E2E 02.2). Chia 5000 → toast "Không thể…", khoang giữ nguyên.
- **TC-14.4 Mất kết nối core** (tay). Tắt `aic-dev-server` → thông báo "Không kết nối được lõi CAD…".
- **TC-14.5 Hiệu năng** (tay, D-LON). 60 tủ: xoay / zoom mượt; đổi rộng 1 tủ < 1 giây; mở báo cáo < 5 giây.
- **TC-14.7 Cập nhật từng phần** (Rust `zones_changed_events_only_for_touched_cabinets`, vitest `useZones.test`). Kéo vách tủ A →
  chỉ `ZonesChanged{A}`; đổi vật liệu / dời tủ B → không có `ZonesChanged`; undo phát lại đối xứng; 2D chỉ tải lại khoang tủ đang sửa.
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
