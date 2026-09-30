# Đề xuất tính năng V2: AIC CAD cho xưởng nội thất gỗ Việt Nam

> Phạm vi: phân tích repo (core `crates/`, UI `app/`, tài liệu `docs/`) ở trạng thái ngày 30/09/2026, gồm cả phần
> **Chia khoang (mới)** đang được làm song song: `parse_split_formula`, `split_with_bays`, `SplitKind::VirtualH/VirtualV`
> (`crates/aic-domain/src/zone.rs`), request `split_zone` (`crates/aic-api/src/protocol.rs`, `zones.rs:261`),
> `app/src/features/cabinet/SplitDialog.tsx`.
> Đề bài gốc: `docs/PROMPT-phan-tich-tinh-nang.md`. Tài liệu này **không** sửa code.
> Mọi kết luận "chưa có" đều đã grep trong `crates/*/src` và `app/src`.
>
> Ký hiệu: **CÓ** · **CÓ NHƯNG CHƯA ĐẠT WORKFLOW SẢN XUẤT** (viết tắt **CÓ\*** trong bảng: có trong code nhưng hardcode,
> không có UI, không có preset, không sửa trực tiếp được hoặc không có undo) · **CHƯA**.
> Mức ưu tiên: P0 thiếu thì không dựng được sản phẩm thật · P1 hay dùng · P2 nâng cao. Độ khó: S ≤ 2 ngày · M 3–5 ngày · L > 1 tuần.

---

# 1. Tóm tắt

Phần tủ thùng đã dùng được khá tốt: khoang KHÓA/%/AUTO, 2D sửa trực tiếp, template, rule preset, ràng buộc cạnh, quan hệ
phủ/lọt, thuộc tính kết cấu (hậu, giằng, len chân), dán cạnh theo luật, costing, nesting, G-code.
Cái còn thiếu nằm ở **lớp quy chuẩn xưởng** và **các sản phẩm không phải tủ hộp chữ nhật**.

**Các khoảng trống lớn nhất:**

1. **Không có "Chuẩn xưởng"**. Khoảng 30 giá trị sản xuất bị viết cứng trong `layout.rs`, `zone.rs`, `cabinet.rs`, gồm
   chốt tầng 37 mm, bản lề Ø35 sâu 13 cách mép 22.5, số bản lề theo bậc 900/1600, hộc ngăn kéo 17.2/8.6, ray bi lấy
   theo sâu − 10, lùa chồng 30, tay nắm 160 đặt giữa cánh… Xưởng dùng chuẩn khác thì không đổi được.
2. **Liên kết và khoan chưa khớp thực tế**. Core chỉ sinh chốt gỗ Ø8 (`JointSettings::default()` cố định,
   `crates/aic-api/src/mfg.rs:76`), không sinh lỗ cam/minifix, nhưng báo giá vẫn tính "Cam & Dowel" theo số chốt
   (`costing.rs:266–279`). Kệ di động chỉ có 2 lỗ chốt mỗi bên, không có hàng lỗ hệ 32.
3. **Không có đối tượng cấp dãy/phòng**. Thiếu mặt đá, chỉ chân chạy suốt, tấm lấp (filler), tấm che trần, phào, và
   báo giá theo **mét dài**. Tủ bếp chữ L/U phải chỉnh tay.
4. **Thiếu tủ góc và các sản phẩm ngoài tủ hộp**. Chỉ có 6 `CabinetKind` (`cabinet.rs:13–20`). Không có tủ góc L,
   góc chéo, giường, bàn, kệ TV treo, vách ốp/lam, bàn đảo mở 2 mặt. Grep "giường/bed/desk/countertop/filler/cornice"
   không ra kết quả.
5. **Cánh, ngăn kéo, tay nắm chỉ có một kiểu phụ kiện**: tay nắm thanh 160 hoặc không có tay nắm; ray bi; bản lề
   thẳng. Chưa có ray âm giảm chấn, tandem box, push-open, tay nắm âm profile, cánh lật tay nâng, bản lề trùm nửa/lọt,
   khoan đế bản lề trên hồi.
6. **Báo giá và nhãn chưa theo cách xưởng làm**: chưa có hao hụt, công, lợi nhuận, báo giá theo phòng/mét dài/m² mặt
   đứng, gộp chi tiết giống nhau, nhãn tấm có mã.
7. **Vật liệu chưa theo thị trường Việt Nam**: thiếu MFC lõi xanh, Acrylic, Laminate. Khổ ván cố định 2440×1220
   (`material.rs:38–39`). Chưa có bộ vật liệu và dán cạnh theo nhóm tấm (ví dụ cánh ABS 2 mm, thùng PVC 1 mm).
8. **Chưa có incremental thật cho 2D**: core dựng lại toàn bộ tủ rồi so khớp theo part key. `ChangeSet` chỉ có
   object/geometry/transform, chưa có `zones/dimensions/relations`. Drawing2D vẽ lại cả view mỗi `rev`, `useZones`
   tải lại toàn bộ `get_zones`.

**Trả lời 5 câu hỏi chính:**

| # | Câu hỏi | Trả lời ngắn |
|---|---|---|
| 1 | Thiếu gì để tùy biến tủ mà không phải chỉnh tay? | (a) **Chuẩn xưởng** có tên, áp theo dự án/tủ, thay giá trị cứng. (b) **Loại phụ kiện** (bản lề, ray, tay nắm) có kích thước và luật khoan. (c) **Tấm ngoài thùng tủ** sinh theo luật: len chân 3 mặt, tấm lấp, phào, ốp hông, che trần. (d) **Hàng lỗ hệ 32** và kệ nghiêng thật (`tilt_deg` đã lưu nhưng `layout.rs::split_panel` bỏ qua). (e) Dán cạnh theo nhóm tấm. |
| 2 | Thao tác nào nên thành direct edit 2D? | Số **lùi kệ** / offset, **khe cánh** từng phía, **cao chân**, **cao ngăn kéo** (đã có), **sâu D** ở view Bên, **cao độ tay nắm**, **vị trí thanh treo**, **khe tấm lấp**, kéo **tay nắm resize tủ** ở 2D (hiện chỉ có ở 3D), snap kéo kệ theo **bước 32** và theo cao độ lưu sẵn. |
| 3 | Giá trị hardcode nào cần thành parameter? | 37 (mép chốt tầng), 5 (lỗ chốt dưới kệ), 1.0/0.5 (hở kệ di động), 22.5/100/Ø35/13 và bảng 900/1600 (bản lề), 30 (chồng lùa), 160/40/6/80 (tay nắm), 17.2/8.6/13/3/3 (ngăn kéo), 60–250/15/40 (cao hộc), 10 (chọn ray), 0.5 và `t−4` (rãnh hậu), 50 (len chân), JointSettings (Ø8, 12/25, 50, 300), 2440×1220, `skip_thicknesses=[8.6]`. Có mâu thuẫn **lùi kệ 20 hay 30**. Chi tiết ở §3. |
| 4 | Loại nội thất nào chưa dựng tốt? | Chưa dựng được: tủ bếp góc L/góc chéo/mâm xoay, **giường** (thường và hộc kéo), **bàn** (học, làm việc, trang điểm), **kệ TV treo** có hộc, **vách ốp TV/lam**, bàn đảo mở 2 mặt, quầy bar, tủ lavabo có khoét ống, tủ thờ. Dựng được nhưng phải chỉnh tay: tủ áo cánh lùa, tủ giày kệ nghiêng, tủ lò/vi sóng (khoang trống + thanh đỡ), tủ kịch trần 2 tầng. |
| 5 | Feature P0 nào cần để đủ dùng cho xưởng thật? | (1) Chuẩn xưởng + sửa các điểm cứng. (2) Liên kết cam/chốt sinh khoan thật + hàng lỗ hệ 32. (3) Phụ kiện có loại: bản lề, ray, tay nắm, push-open. (4) Tấm ngoài thùng: len chân 3 mặt/chân nhựa, tấm lấp, che trần. (5) Tủ góc L bếp + Dãy tủ (mặt đá, chỉ chân). (6) Báo giá mét dài/m² + hao hụt + công. (7) Nhãn tấm + danh sách cắt gộp. (8) Vật liệu VN + dán cạnh theo nhóm tấm. |

---

# 2. Bảng hiện trạng (15 nhóm)

| # | Nhóm | Đã có | Thiếu tùy chọn | Chưa có | File liên quan |
|---|---|---|---|---|---|
| 1 | **Khung tủ** | 6 kiểu khung BếpDưới/BếpTrên/TủQA/Kệ/TủNgănKéo/Tủ1Cánh. Nóc phủ/lọt/thanh giằng, đáy phủ/lọt. Neo W/H/D. Resize theo KEEP/PROPORTIONAL/EDGE. Lật gương. Nhân dãy tủ. Dãy tủ tự dịch theo. | **CÓ\*** Plinth chỉ có mặt trước (`layout.rs:521`) và chỉ cho Base/Wardrobe/Drawer. Wall không có thông tin treo. Hồi luôn full sâu và full cao, không có hồi lửng/âm/vát. Kiểu khung chỉ là 1 enum, không có tham số riêng cho từng kiểu. | Tủ góc L, góc chéo, mâm xoay. Tủ cong. Tủ mở 2 mặt (bàn đảo). Tủ chân (chân nhựa tăng chỉnh). Hồi thừa (hồi ra chạm sàn/che chân). Hồi nhô trước. | `crates/aic-domain/src/cabinet.rs:13–95`, `layout.rs:441–531`, `structure.rs`, `app/src/features/cabinet/FrameTab.tsx:11–17` |
| 2 | **Kết cấu phụ** | Thanh giằng trên trước/sau/bổ sung (ngang/đứng, âm mặt). Thanh chặn cánh chữ L/thẳng. Len chân (cao, giật vào). | **CÓ\*** Thanh chặn có giá trị cứng 75/25/40/20.2 (`zone.rs:412`) và chỉ đặt ở cạnh trên. Không có giằng dưới/giằng sau cho tủ không hậu. | Phào nóc, phào chân, nẹp, ốp hông (end panel), tấm che hở trần, **tấm lấp (filler)** sát tường, thanh treo tủ bếp trên (ke/thanh treo). | `structure.rs:47–101`, `layout.rs:544–586`, `layout.rs:826–841`, `structure_api.rs:67–87` |
| 3 | **Hậu** | Hậu lọt / hậu rãnh (C, B, I, khe hở). Hở 4 phía. Chia dọc theo công thức (`600`, `3x`). Nóc/đáy trùm hậu. Hậu phụ (BackSub). Rãnh sinh trên hồi, nóc, đáy. | **CÓ\*** Dung sai rãnh 0.5 và sâu `min(C, t−4)` viết cứng (`layout.rs:494, 501`). Chưa có hậu ốp (bắt vít sau, không rãnh). Chia hậu chỉ theo chiều dọc. | Khoét hậu (ổ điện, ống nước, lỗ thoát nhiệt tủ lò). Hậu đôi. Hậu dừng tại kệ cố định. Chia hậu ngang. | `structure.rs:7–46`, `layout.rs:486–518`, `structure_api.rs:48–65` |
| 4 | **Khoang & tấm chia** | Cây zone. Khoang KHÓA/%/AUTO (`Split.bays`). Chia đều lại. Kéo vách/kệ trên 2D. Nhân tấm. Mẫu vùng. **Mới**: Chia khoang theo công thức (`500,30%,*`, `3*400`, `/3`), chia ảo VirtualH/V, `split_zone`, SplitDialog. Kệ di động/cố định, hông giữa. | **CÓ\*** Kệ nghiêng: `tilt_deg` lưu và nhập được (`CreateTab.tsx:74`, `zones.rs:663`) nhưng **không dùng khi sinh tấm** (`layout.rs:728–766`). Kệ di động luôn hụt 1 mm (`w−1.0`, `x+0.5`, `layout.rs:742`). Snap khi kéo chỉ có "chia đều" và Shift 10 mm (`EditLayer.tsx:457`). | Hàng lỗ chốt tầng hệ 32. Snap theo bước 32 / cao độ lưu sẵn. Vách lửng (không chạm nóc). Kệ góc (L). Kệ lùi theo từng tấm như 1 tham số zone (hiện dùng offset). | `crates/aic-domain/src/zone.rs`, `layout.rs:588–766`, `crates/aic-api/src/zones.rs`, `app/src/features/cabinet/{CreateTab,SplitDialog}.tsx`, `app/src/features/drawing2d/EditLayer.tsx` |
| 5 | **Cánh** | Đơn/đôi/lùa. Phủ bì/lọt lòng (nửa phủ tự động ở vách giữa, `front_rect`). Lưới cột × hàng. Lề trái/phải/trên/dưới. Khe chung + khe từng phía. Dày cánh. Thanh chặn. Lỗ chén bản lề. Đảo phủ/lọt (tool 13). | **CÓ\*** Số bản lề theo bậc cứng ≤900→2, ≤1600→3, còn lại 4 (`layout.rs:769`), không tính theo rộng hoặc khối lượng. Mép chén 22.5 và cách đầu 100 viết cứng. Lùa: chồng 30 cứng, ray cao 20/10 cứng (`layout.rs:847–858`). Có hai luật bản lề khác nhau (xem §3). | Loại bản lề (thẳng/cong/trùm nửa/lọt, 110°/165°, bật). Khoan đế bản lề trên hồi (`DrillPurpose::HingeScrew` có nhưng không sinh). Cánh lật lên có tay nâng, cánh gập. Cánh kính, khung nhôm. Chiều mở hiển thị trên 2D. | `zone.rs:360–445`, `layout.rs:769–902`, `crates/aic-manufacturing/src/features.rs:57–84`, `crates/aic-api/src/properties.rs:160–190` |
| 6 | **Ngăn kéo** | Số tầng × cột. Phủ/lọt. Khe. Hộc gỗ (thành, hậu, đầu, đáy). Ray bi tự chọn theo sâu (250–600). Cao từng ngăn KHÓA/%/AUTO. Kéo đường chia trên 2D. | **CÓ\*** Hộc dày 17.2 và đáy 8.6 cứng trong `DrawerSpec::new` (`zone.rs:469–481`), không theo dày ván của tủ. Cao hộc `clamp(cell−40, 60, 250)`, đáy hộc cách +15 (`layout.rs:977–978`). Ray chọn theo `≤ sâu − 10` (`layout.rs:948`). Đáy hộc đặt chồng, không có rãnh. | Ray âm giảm chấn, tandem/box kim loại (Blum/Hafele/Garis). Ngăn kéo trong (sau cánh). Mặt ngăn giả. Chia hộc (khay chia). Khoan lỗ ray trên hồi và lỗ bắt mặt. | `zone.rs:448–490`, `layout.rs:904–989` |
| 7 | **Phụ kiện** | Tay nắm thanh 160 (bật/tắt cả tủ). Thanh oval + chén. Ray lùa. Chốt tầng (đếm). Vít theo luật (`ScrewRule`). | **CÓ\*** Tay nắm chỉ có 1 mã `HDL-BAR-160` đặt giữa chiều cao cánh (`layout.rs:889–897`); bản cũ `cabinet.rs:417–424` còn phân biệt tủ trên/dưới, bản zone đã mất. Oval cứng `w−4`, 30×15 (`layout.rs:996–998`). `HardwareKind::Leg` có hình học (`aic-geometry/src/panel.rs:186`) nhưng không có generator. | Push-open, tay nắm âm profile nhôm (Gola), núm. Giá bát đĩa, rổ gia vị, giá giày, giá kéo quần, đèn LED (rãnh có tool 16 nhưng không có nguồn/dây). Chân nhựa tăng chỉnh. Ke góc, ke treo tủ bếp trên. Catalog phụ kiện có kích thước khoang tối thiểu. | `layout.rs:291–302, 889–1003`, `crates/aic-domain/src/object.rs:191–197`, `crates/aic-project/src/document.rs:95–115` |
| 8 | **Vật liệu & dán cạnh** | 11 vật liệu mặc định. Slot thùng/cánh/hậu. Vật liệu từng tấm (PartMod). Luật dán cạnh (hở bỏ khuất / toàn bộ / không), bỏ độ dày, ngưỡng, bỏ cạnh ngắn, ghi đè từng cạnh. Kích thước cắt đã trừ độ dày chỉ (`costing.rs:131`). | **CÓ\*** Một mã chỉ cho cả tủ (`EdgeRule.band_code`, `object.rs:79–103`). Mặc định `skip_thicknesses=[8.6]` theo số (hậu 9 mm hay 5 mm thì vẫn bị dán). `MaterialKind` không có MFC/HMR. Khổ ván cứng 2440×1220. | Bộ vật liệu (material set) theo tủ/phòng. Dán cạnh theo **nhóm tấm** (cánh, thùng, kệ trước). Nối vân nhiều cánh. Chỉ dày ≥ 1 mm có trừ pre-milling. Vật liệu bề mặt 2 mặt khác nhau (A/B). | `crates/aic-domain/src/material.rs`, `object.rs:66–103`, `crates/aic-api/src/costing.rs:65–100`, `app/src/features/cabinet/FrameTab.tsx:56–61`, `app/src/features/materials/MaterialBrowser.tsx` |
| 9 | **Gia công & sản xuất** | Feature local (drill/edge drill/pocket/groove/contour). Suy diễn chốt gỗ và chốt tầng theo tiếp xúc. Trải phẳng A/B. Nesting MaxRects (vân, xoay). Toolpath + G-code mặt A. Tool 03/04/08–12/16/18/20–22. | **CÓ\*** `JointSettings` (Ø8, sâu 12/25, cách đầu 50, bước ≤300) không sửa được (`mfg.rs:76`). Khoan mặt B / khoan cạnh chỉ cảnh báo (STATUS L7). Tool 03/08/16/18 tính tọa độ feature **trong UI** theo kích thước hiện tại và lặp từng tấm, không gom thành một bước undo (`ToolColumn.tsx:154–163, 171, 198, 324`). | Cam/minifix (`DrillPurpose::CamLock` có nhưng không sinh). Nhãn tấm (mã, QR). Xuất DXF/MPR/CIX/CSV máy cắt. Tool 05 (Cắt có liên kết), 07 (Ghép bề mặt), 15 (Mộng đan tay) đang "Sắp có" (`ToolColumn.tsx:18,20,28`). | `crates/aic-manufacturing/src/{features,cnc,flatten}.rs`, `crates/aic-api/src/mfg.rs`, `crates/aic-nesting/src/lib.rs`, `app/src/features/tools/ToolColumn.tsx` |
| 10 | **Báo giá** | Costing: m² ván theo vật liệu × dày, mét chỉ (có hệ số), phụ kiện theo loại/chiều dài, vít, đơn giá sửa được (undo), tổng. Danh sách cắt, Cabinet List. Xuất CSV/"Excel". | **CÓ\*** Không có hao hụt ván (nesting có `waste_ratio` nhưng costing không dùng). Đơn giá chỉ lưu theo dự án (`ProjectSettings.prices`), chưa có bảng giá thư viện. | Báo giá theo **mét dài** (bếp), **m² mặt đứng** (tủ áo), theo phòng/tầng. Công, lắp đặt, vận chuyển, lợi nhuận, VAT, chiết khấu. PDF báo giá có logo. | `crates/aic-api/src/costing.rs`, `crates/aic-project/src/document.rs:20–115`, `app/src/features/report/ReportWindow.tsx` |
| 11 | **UX/thao tác** | Chọn object/face/edge, multi, box. Gizmo move/rotate có snap từ core. Kéo khối trong 3D, gõ số khi kéo. Handle W/H/D. Phím tắt cấu hình được. Context menu tủ/tấm/vùng. Undo/redo. Tầng/phòng. Multi-edit "Nhiều giá trị". | **CÓ\*** Snap chỉ theo AABB (STATUS L4). Xoay chỉ qua ô rx/ry/rz. Dãy tủ chỉ nối sang phải (`CreateCabinet.after`). | Căn/phân bố nhiều tủ (STATUS "Chưa làm"). Xoay 90° nhanh. Đặt tủ theo tường của phòng (Room có `wall_thickness_mm` nhưng không có tường để bám). Dãy tủ chữ L/U. Đo hiện trường (nhập kích thước tường/trần). | `app/src/app/{Chrome,Shortcuts,actions}.ts(x)`, `app/src/viewport/Viewport.tsx`, `crates/aic-spatial/src/snap.rs`, `crates/aic-domain/src/object.rs:8–15` |
| 12 | **Template & thư viện** | Template tủ (lưu/chèn, giải lại theo W/H/D). Rule preset (2 builtin + lưu từ tủ). Mẫu từng tab kết cấu. Mẫu vùng. Thư viện dùng chung `~/.aic-cad/library.json`. | **CÓ\*** Chưa có **bộ template dựng sẵn** theo sản phẩm Việt Nam (chỉ có 2 rule preset, `templates.rs:19–40`). Template không có ảnh xem trước, nhóm, tham số hiển thị. | Thư viện nhóm/đám mây, phân quyền. Template có tham số người dùng (ví dụ "số cánh", "có ngăn kéo"). Template nhiều tủ (cả dãy bếp). | `crates/aic-api/src/{templates,library}.rs`, `crates/aic-project/src/document.rs:41–62` |
| 13 | **Loại nội thất khác** | Kệ mở (OpenShelf). Có thể ghép tay bằng tấm rời (`CreatePanel`). | — | Giường (đầu, vai, dát, chân, hộc kéo, giường tầng). Bàn (học, làm việc, trang điểm, ăn). Kệ TV treo. Vách ốp/lam. Bàn đảo. Tủ giày có ghế. Tủ thờ. Không có khái niệm "sản phẩm" khác "tủ". | `cabinet.rs:13–20` (enum đóng), `layout.rs::build` |
| 14 | **Bản vẽ/hiển thị** | View Trước/Trái/Phải/Trên/Mặt cắt dọc/ngang. Kích thước W/H + khoang + ngăn kéo sửa được. Bản vẽ trải phẳng tấm. Phối cảnh vật liệu (màu). Ẩn cánh. | **CÓ\*** Kích thước chỉ có W/H tủ và khoang. Không có chuỗi kích thước tổng dãy, cao độ, sâu. | Xuất bản vẽ in A3/A4 có khung tên. Bản vẽ lắp đặt theo phòng (mặt bằng + mặt đứng tường). Ký hiệu chiều mở cánh. Xuất PDF/DXF bản vẽ. Texture vân gỗ thật. | `app/src/features/drawing2d/{Drawing2D,EditLayer}.tsx`, `app/src/features/manufacturing/FlatPanelView.tsx` |
| 15 | **Parametric interaction** | Dimension sửa trực tiếp (W/H, khoang, ngăn kéo). LOCK/AUTO/%. Neo resize. Kéo vách/kệ (preview client, commit core). Kéo 4 cạnh tấm (giữ ràng buộc/tự do). Anchor cạnh → mặt. Offset 6 phía. Quan hệ phủ/lọt/bằng/khe. Multi-edit. Array tủ/tấm. Template. | **CÓ\*** Preview do UI tự tính (`EditLayer.tsx:449–460`). ChangeSet chưa có zone/dimension. 2D vẽ lại toàn view. Dimension chưa có cho D, offset, khe, cao chân. | Preview do core tính (dry-run). Ràng buộc kích thước tối thiểu theo phụ kiện (khoang ≥ rộng rổ kéo). Array tủ theo trục Y/Z trong UI (core đã có `axis`). | `docs/TONG-HOP-PARAMETRIC-EDITOR.md`, `crates/aic-project/src/document.rs:119–140`, `app/src/viewport/sceneSync.ts`, `app/src/features/cabinet/useZones.ts:26–40` |

---

# 3. Lỗi / điểm cứng phát hiện

## 3.1 Giá trị cứng nên chuyển thành tham số

Chúng nên gom vào `ShopStandard` (đề xuất D01). Cột "Tham số đề xuất" là tên field trong `ShopStandard` hoặc `StructureRules`.

| # | File:dòng | Giá trị cứng | Vấn đề | Tham số đề xuất (default · min–max) |
|---|---|---|---|---|
| H1 | `crates/aic-domain/src/cabinet.rs:74–75`, `material.rs:48–51` | Ván 17.2, hậu 8.6 | Đúng với MDF An Cường nhưng không đúng với MFC 18, HDF 5 | `std.board_t` 17.2 · 3–60; `std.back_t` 8.6 · 3–25 (lấy theo vật liệu slot nếu bật "dày theo vật liệu") |
| H2 | `cabinet.rs:239`, `document.rs:572` (lùi kệ 20) và `crates/aic-api/src/templates.rs:24` (lùi kệ 30) | 20 và 30 | **Mâu thuẫn**: tủ mới lùi 20, preset "Wardrobe Standard" lùi 30, tài liệu ghi 30 | `std.shelf_setback` 20 · 0–100, preset ghi rõ |
| H3 | `layout.rs:742` | Kệ di động `w − 1.0`, `x + 0.5` | Hở 0.5 mỗi bên cứng; xưởng thường dùng 1 mm mỗi bên | `std.shelf_side_clearance` 0.5 · 0–3 |
| H4 | `layout.rs:745–746` | Lỗ chốt tầng cách mép 37, dưới kệ 5, Ø5 × 10 | Chỉ khoan đúng vị trí kệ, không có hàng lỗ 32 | `std.pin_edge` 37 · 20–100; `std.pin_pitch` 32; `std.pin_d` 5; `std.pin_depth` 10; `std.pin_row` enum `AT_SHELF` / `ROW_32` |
| H5 | `layout.rs:769–777` | Số bản lề ≤900 → 2, ≤1600 → 3, còn lại 4 | Không theo rộng/khối lượng cánh; không có ngưỡng 2000/2400 → 5 | `std.hinge_table` [(900,2),(1600,3),(2000,4),(2400,5)]; `std.hinge_max_weight_kg` |
| H6 | `layout.rs:782–783, 793` | Tâm chén cách mép 22.5, cách đầu 100, Ø35 sâu 13 | Cố định theo một loại bản lề | Lấy từ `HingeType` trong catalog (D05): `cup_d` 35, `cup_depth` 13, `cup_edge` 22.5, `end_inset` 100 |
| H7 | `crates/aic-manufacturing/src/features.rs:64–69` và `layout.rs:769` | Luật bản lề **thứ hai** (2 chén, thêm 1 nếu cao > 1500) cho cánh không sinh từ generator | **Hai luật khác nhau** cho cùng một loại chi tiết | Dùng chung `std.hinge_table` |
| H8 | `layout.rs:847, 857–858` | Lùa chồng 30; ray trên 20, ray dưới 10; ray sâu `2t + 8` | Mỗi hệ ray lùa có thông số khác nhau | `SlidingSystem { overlap 30, top_h 20, bottom_h 10, track_depth, leaf_offset 4 }` |
| H9 | `layout.rs:889–897, 960` | Tay nắm 160; cách mép 40; lệch 6; giữa chiều cao cánh | Tủ bếp dưới cần tay nắm ở trên, tủ trên ở dưới. Bản cũ `cabinet.rs:417–424` có luật này, bản zone bị mất (**regression**) | `HandleSpec { type, length 160, hole_pitch 128, edge 40, v_pos AUTO / TOP / BOTTOM / CENTER / mm }` |
| H10 | `crates/aic-domain/src/zone.rs:469–481` | Ngăn kéo: khe hông 3, khe giữa 3, thành hộc 17.2, đáy 8.6, ray 13 | Tủ đổi sang ván 18 thì hộc vẫn 17.2 | `std.drawer_box_t` = `thickness`; `std.drawer_bottom_t` = `back_thickness`; `slide_clearance` lấy theo loại ray |
| H11 | `layout.rs:977–978` | Cao hộc `clamp(cell − 40, 60, 250)`, đáy hộc +15 | Ngăn kéo sâu (nồi) cao hơn 250 bị cắt | `std.drawer_box_top_gap` 40; `std.drawer_box_bottom_gap` 15; `drawer.box_height` AUTO / mm |
| H12 | `layout.rs:377, 948` | `STD_SLIDES` 250…600; chọn ray `≤ sâu − 10` | Ray âm cần sâu − 20…25; không có 650/700 | `SlideType { lengths[], depth_margin }` |
| H13 | `layout.rs:494, 501` | Dung sai rãnh 0.5; sâu rãnh `min(C, t−4)` | Xưởng dùng rãnh rộng 9 cho hậu 8.6 và bắt rãnh lệch | `rules.back.groove_tol` 0.5; `rules.back.groove_min_wall` 4 |
| H14 | `crates/aic-domain/src/structure.rs:94–96` | Len chân giật 50 | Đúng mặc định nhưng chỉ có mặt trước | Giữ, thêm `plinth_sides` (D08) |
| H15 | `zone.rs:412` | Thanh chặn 75/25/40/20.2 | 20.2 = 3 + 17.2, sai khi ván 18 | `setback = door_gap_front + board_t` (biểu thức) |
| H16 | `layout.rs:996–998`, `layout.rs:1139`, `app/src/features/cabinet/CreateTab.tsx:260` | Oval `w − 4`, OVAL-30×15, cách trên 60 | Không có loại thanh treo tròn Ø25 / thanh treo nâng hạ | `LinkKind::{OvalRail, RoundRail, PullDownRail}` + `end_gap` 2 |
| H17 | `crates/aic-manufacturing/src/features.rs:41–53`, `crates/aic-api/src/mfg.rs:76` | JointSettings Ø8, sâu 12/25, cách đầu 50, bước 300 | Không sửa được; không có cam | `ProjectSettings.joinery` (D06) |
| H18 | `crates/aic-domain/src/material.rs:38–39` | Khổ 2440×1220 cho mọi vật liệu | MFC/Acrylic có khổ 1830×2440 | Field sửa được trong MaterialBrowser |
| H19 | `crates/aic-domain/src/object.rs:98` | `skip_thicknesses: [8.6]` | Hậu 5/9 mm vẫn bị dán | Bỏ dán theo **vai trò** (Back/BackSub/DrawerBottom) thay vì theo độ dày |
| H20 | `crates/aic-api/src/zones.rs:237, 266`; `properties.rs:212` | Dùng 17.2/8.6 khi không đọc được tham số | Hiếm khi xảy ra, nhưng che lỗi | Trả `NOT_FOUND` thay vì đoán |
| H21 | `app/src/features/cabinet/CreateTab.tsx:71, 158, 220` | Form Tạo tấm mặc định 17.2 / 8.6 | **UI giữ giá trị sản xuất**; tủ ván 18 vẫn thêm kệ 17.2 | Để trống = "theo tủ"; core điền |
| H22 | `app/src/features/cabinet/FrameTab.tsx:59` | `skip {'17.2': false, '8.6': true}` | Luật dán cạnh viết trong UI | Core trả luật mặc định của chuẩn xưởng |
| H23 | `app/src/features/tools/ToolColumn.tsx:266, 700`, `app/src/app/Chrome.tsx:403` | Rãnh 13, khe 3 | Mặc định UI | Lấy từ `ShopStandard` qua `get_status` / `get_structure` |
| H24 | `crates/aic-domain/src/cabinet.rs:386, 429, 461, 479` | Generator cũ (plinth `depth − 50`, tay nắm, oval `inner_top − 80`) | Code cũ vẫn chạy song song với `layout.rs` (template `derived`); dễ lệch | Dọn phần panel/hardware của `cabinet::generate`, chỉ giữ `derived` và `constraints` |

## 3.2 Lỗi logic / điểm không nhất quán

| # | File:dòng | Mô tả | Hệ quả sản xuất | Sửa đề xuất |
|---|---|---|---|---|
| B1 | `layout.rs:728–766` (không đọc `tilt_deg`) | Kệ nghiêng lưu được nhưng không sinh hình và không có kích thước nghiêng | Tủ giày kệ nghiêng phải vẽ tay | D14 |
| B2 | `costing.rs:266–279` | "Cam & Dowel" đếm theo số lỗ chốt gỗ, nhưng core không sinh lỗ cam | Báo giá có cam mà file CNC không có lỗ cam | D06 |
| B3 | `ToolColumn.tsx:154–163, 171–181` | Khấu góc/khấu mặt/LED/Vbit: UI tính tọa độ (`pw − w`) rồi gửi feature tuyệt đối; lặp từng tấm, không `Batch` | Trái CLAUDE.md (UI tính CAD). Tấm đổi kích thước thì khấu góc nằm sai chỗ. Undo phải bấm N lần | D24 |
| B4 | `layout.rs:889–897` và `cabinet.rs:417–424` | Tay nắm luôn ở giữa cao cánh | Tủ bếp dưới/trên đặt sai vị trí tay nắm | D05 |
| B5 | `features.rs:57–84` và `layout.rs:769–797` | Hai luật bản lề | Số bản lề trên báo giá khác số lỗ | D05 |
| B6 | `cabinet.rs:239` và `templates.rs:24` | Lùi kệ 20 hay 30 | Kệ tủ áo không nhất quán giữa các tủ | D01 |
| B7 | `layout.rs:521` | Len chân chỉ sinh khi `p > 1` và kind ∈ {Base, Wardrobe, Drawer} | OpenShelf/Door đặt sàn không có chân; Wall có `plinth_height` thì đáy bị nâng mà không có chân | D08 |
| B8 | `layout.rs:977` | Cao hộc tối đa 250 | Ngăn nồi 300–350 không đúng | D04 |
| B9 | `app/src/features/drawing2d/Drawing2D.tsx:45–56` + `useZones.ts:26–40` | Mỗi `rev`: tính lại toàn bộ item của scope và gọi lại `get_zones` | Chậm với dự án lớn; chưa đạt DoD §35.13 ("Không redraw toàn bộ 2D") | D20 |
| B10 | `crates/aic-project/src/document.rs:119–133` | `ChangeSet` không có zones/dimensions/relations | UI phải tải lại toàn bộ để biết khoang đổi | D20 |

---

# 4. Danh sách đề xuất chi tiết

Quy ước chung, **áp dụng cho mọi đề xuất**:
- Mọi thay đổi đi qua `Command::SetCabinet` / `SetParameter` / `Batch` (có lệnh nghịch đảo), hoặc qua Command mới ghi rõ.
- Kích thước là tham số. Không scale mesh. Feature lưu ở tọa độ local của tấm. UI không tính CAD.
- Thêm request theo thứ tự: `protocol.rs` → `lib.rs::handle` → `app/src/core-api/{types,commands,queries}.ts` →
  `docs/api-mapping.md`. Mã lỗi mới thêm vào `app/src/core-api/errors.ts`.
- Event flow mặc định cho sửa tủ: *Action UI → Request → `edit_cabinet(_checked)` → `Command::SetCabinet` →
  `regenerate_cabinet` (so khớp theo part key) → `ChangeSet{changed, geometry, created, deleted}` → CoreEvents →
  `sceneSync.syncIds` + `useZones` → 3D/2D patch*. Mục nào khác mặc định thì ghi rõ.

---

## Nhóm A: Nền tảng chuẩn xưởng & sản xuất

### [P0][M] D01 · Chuẩn xưởng (ShopStandard) có tên, áp theo dự án / tủ
- **Mô tả**: gom các giá trị cứng ở §3.1 vào một cấu trúc có tên, ví dụ "AIC MDF 17", "Xưởng MFC 18", "An Cường
  Acrylic". Dự án chọn một chuẩn mặc định; tủ có thể ghi đè từng field. Rule preset hiện có (`RulePreset`) thành một
  phần của chuẩn.
- **Tùy chọn**:
  - `board_t` (số, mm, 17.2, 3–60) · `back_t` (số, mm, 8.6, 3–25) · `door_t` (số, mm, = board_t).
  - `door_gap` (số, mm, 2, 0–10) · `shelf_setback` (số, mm, 20, 0–100) · `shelf_side_clearance` (số, mm, 0.5, 0–3).
  - `back_groove` (số, mm, 0, 0–20) · `back_offset` (số, mm, 10, 0–50) · `groove_tol` (0.5) · `groove_min_wall` (4).
  - `pin_edge` (37) · `pin_pitch` (32) · `pin_d` (5) · `pin_depth` (10) · `pin_row` (chọn `AT_SHELF` / `ROW_32`, mặc định `ROW_32`).
  - `hinge_table` (bảng cao → số lượng) · `default_hinge` / `default_slide` / `default_handle` (mã catalog, D05).
  - `drawer_box_t` (= board_t) · `drawer_bottom_t` (= back_t) · `drawer_box_gaps` [trên 40, dưới 15].
  - `plinth_setback` (50) · `thickness_from_material` (bật/tắt, mặc định tắt): dày tấm lấy theo vật liệu của slot.
- **UI**: Cài đặt → tab "Chuẩn xưởng" (bảng field do core trả, giống `get_structure`). Tab Khung → ô chọn "Chuẩn xưởng".
  Bảng Thuộc tính kết cấu → nút "Theo chuẩn" / "Ghi đè" cho từng field (chấm đỏ = ghi đè).
- **Core**: `aic_domain::standard::ShopStandard` (serde default = giá trị hiện tại, để file cũ không đổi hình).
  `ProjectSettings.standards: Vec<ShopStandard>` + `default_standard: String`. `Cabinet.standard: Option<String>` +
  `Cabinet.std_overrides: BTreeMap<String, f64>`. `CabinetValues` nhận thêm `&ShopStandard` (hoặc struct đã trộn).
  Request mới: `get_standards`, `save_standard {name, from_cabinet?}`, `apply_standard {ids, name}`,
  `set_standard_field {name, key, value}`. Command mới `SetStandard { name, standard: Option<ShopStandard> }` (giống
  `SetRulePreset`). Migration `ProjectFile` v1 → v2: tạo chuẩn "Mặc định" từ giá trị cứng; `RulePreset` cũ đổi thành
  chuẩn.
- **Event flow**: Cài đặt sửa `pin_edge` → `set_standard_field` → `Command::SetStandard` → mọi tủ dùng chuẩn đó được
  `regenerate_cabinet` → ChangeSet.geometry = các hồi/vách có lỗ chốt → `GeometryChanged` → `get_render_objects(ids)`
  → 2D/FlatPanelView tải lại tấm đang xem.
- **Production impact**: cắt đúng dày, khoan đúng, hộc ngăn kéo theo ván. Đổi chuẩn là tính lại toàn bộ báo giá.
- **Nghiệm thu**:
  1. Tạo chuẩn "MFC 18" (board 18, back 9). Áp cho tủ áo 1600 → danh sách cắt: hồi dày 18, hậu 9, hộc ngăn kéo 18. Ctrl+Z trả lại 17.2.
  2. Tủ mới không chọn chuẩn → lùi kệ = 20 ở cả form, 2D và preset (hết mâu thuẫn 20/30).
  3. Mở file dự án cũ → hình và danh sách cắt giống hệt trước khi nâng cấp (test `save_load_identical` + so sánh layout).

### [P0][L] D06 · Liên kết thùng: cam + chốt (minifix), vít, ke — cấu hình được và sinh khoan thật
- **Mô tả**: thay `JointSettings::default()` bằng luật liên kết của dự án/tủ. Mỗi mối nối hồi–nóc/đáy/kệ cố định sinh
  đúng lỗ theo kiểu: *chốt gỗ + cam* (lỗ cam Ø15 mặt trong + lỗ Ø8 cạnh + chốt), *chỉ chốt*, *vít xuyên (Ø5 mặt ngoài
  + Ø3 cạnh)*, *ke góc*. Báo giá đếm đúng từng loại.
- **Tùy chọn**: `joint_type` (chọn `CAM_DOWEL` / `DOWEL` / `SCREW` / `BRACKET`, mặc định `CAM_DOWEL`) ·
  `cam_d` (15, 10–25) · `cam_depth` (12.5) · `cam_offset_from_edge` (34 — khoảng cách tâm cam tới mép, theo loại
  minifix) · `bolt_d` (8) · `dowel_d` (8) · `dowel_face_depth` (12) · `dowel_edge_depth` (25) · `end_inset` (50, 20–150) ·
  `max_pitch` (300, 100–600) · `pattern` (chọn `CAM_DOWEL_CAM` / `DOWEL_CAM_DOWEL`) · `min_count` (2).
- **UI**: Thuộc tính kết cấu → tab mới "Liên kết" (bảng field + Lưu mẫu tab). Chỉnh tấm → nhóm "Liên kết" hiện danh sách
  mối nối của tấm (đọc từ core). FlatPanelView: tô màu lỗ cam/chốt/vít khác nhau.
- **Core**: `aic_manufacturing::JointSettings` thêm `joint_type`, cam fields. Lưu ở `ShopStandard.joinery` và
  `StructureRules.joinery: Option<...>`. `derive_joint_features` sinh `DrillPurpose::CamLock` (mặt) + `EdgeDrill`
  bolt/dowel. `mfg.rs:76` đọc settings theo tủ của từng tấm (cache theo revision như hiện tại). Costing đếm
  `fit:cam`, `fit:dowel`, `fit:screw_joint` riêng thay cho `cam_dowel`. Không cần Command mới (đi qua
  `set_parameter`/`apply_group_preset`).
- **Event flow**: đổi `joint_type` → `set_parameter {id: cab, name: "joint_type"}` → `SetCabinet` → ChangeSet.changed
  (tủ) → `ObjectChanged` → mfg cache invalidate theo revision → FlatPanelView/Costing tải lại khi mở.
- **Production impact**: CNC/khoan có lỗ cam đúng. Báo giá có số cam, chốt, vít đúng. Khoan cạnh vẫn là cảnh báo cho
  tới khi có D26.
- **Nghiệm thu**:
  1. Tủ bếp dưới 800, nóc giằng, kiểu CAM_DOWEL → mỗi mối hồi–đáy (sâu 560) có 2 cam + 3 chốt. Costing: cam = số lỗ cam trên toàn tủ.
  2. Đổi thành SCREW → lỗ cam biến mất, xuất hiện lỗ vít Ø5 mặt ngoài hồi; costing không còn cam.
  3. Tủ cánh lùa không hậu: ke góc được đếm, không có lỗ.

### [P0][M] D02 · Hàng lỗ chốt tầng hệ 32 + snap kéo kệ theo lưới 32
- **Mô tả**: kệ di động dùng hàng lỗ Ø5 bước 32 trên hồi/vách, cách mép trước `pin_edge` và mép sau `pin_edge_back`,
  chạy trong khoảng `[zone_bottom + start, zone_top − end]`. Cao độ kệ bắt vào lỗ gần nhất. Khi kéo kệ trên 2D, snap
  vào lỗ 32 (vạch mờ hiện trên hồi).
- **Tùy chọn**: `pin_row` (`AT_SHELF` / `ROW_32`, mặc định `ROW_32`) · `pin_row_start` (mm từ đáy khoang, 64, 0–300) ·
  `pin_row_end` (64) · `pin_edge_back` (37) · `pin_rows` (2, 2–3: thêm hàng giữa khi sâu > 500) · `snap_to_pins` (bật).
- **UI**: Thuộc tính kết cấu → tab "Lùi đợt" đổi thành "Kệ & chốt tầng". 2D: khi kéo kệ di động, vạch mờ mỗi 32 mm;
  nhãn "Lỗ #12 · 416". Shift = bỏ snap. Chỉnh tấm → Vị trí: nút "Bắt lỗ gần nhất".
- **Core**: `layout.rs::split_panel` (ShelfAdjustable): lưu dải lỗ theo zone (không theo kệ). Thêm
  `Layout.pin_rows: Vec<PinRow{part, ys}>`; sinh `DrillFeature` ShelfPin cho cả hàng (loại trùng khi nhiều kệ cùng
  vách). `solve_bays` giữ nguyên; thêm hàm thuần `snap_to_pins(y, rows) -> y` trong domain. `get_zones` trả `pin_grid`
  cho UI vẽ vạch (UI không tự tính). Request `move_split_panel` thêm `snap: "PINS" | "NONE"`: core làm tròn.
- **Event flow**: kéo kệ (preview client dùng `pin_grid` từ core, chỉ để hiển thị) → thả → `move_split_panel {id, before, snap: PINS}`
  → core làm tròn `before` về lỗ gần nhất → `SetCabinet` → ChangeSet: kệ + 2 hồi (feature không đổi nếu `ROW_32`) → patch.
- **Production impact**: số chốt tầng = 4 × số kệ di động (giữ nguyên). Khoan: hàng lỗ Ø5 (CNC chạy bằng mũi 5 mm đã có
  trong `default_tools`). Không đổi danh sách cắt.
- **Nghiệm thu**:
  1. Tủ áo cao 2400, khoang trái 3 kệ di động, `ROW_32` → hồi trái có 2 hàng lỗ cách nhau 32, cách mép trước 37; FlatPanelView đếm đúng số lỗ.
  2. Kéo kệ 2 tới 1133 → thả → kệ về 1136 (bội 32 tính từ lỗ đầu); undo trả lại vị trí cũ.
  3. `pin_row = AT_SHELF` → hành vi như hiện tại (test hồi quy).

### [P0][M] D05 · Catalog phụ kiện có loại: bản lề, ray, tay nắm, push-open
- **Mô tả**: thay mã cứng (`HDL-BAR-160`, `RAYBI-*`, chén Ø35) bằng catalog phụ kiện có thông số hình học + luật khoan
  + giá. Mỗi `DoorSpec` / `DrawerSpec` tham chiếu mã catalog (None = mặc định của chuẩn xưởng).
- **Tùy chọn**:
  - Bản lề: `hinge_type` (`STRAIGHT` phủ toàn / `HALF` trùm nửa / `INSET` lọt / `WIDE_165` / `PUSH`) · `cup_d` 35 ·
    `cup_depth` 13 · `cup_edge` 22.5 (3–7 K) · `end_inset` 100 (60–200) · `plate` (bật: khoan đế trên hồi, 2 lỗ Ø5 cách
    mép trước 37, bước 32) · `soft_close` (bật).
  - Tay nắm: `handle_type` (`BAR` / `KNOB` / `PROFILE_GOLA` / `EDGE_PULL` / `PUSH_OPEN` / `NONE`) · `length` (160) ·
    `hole_pitch` (128; 96/128/160/192/224/320) · `edge` (40) · `v_pos` (chọn `AUTO` / `TOP` / `BOTTOM` / `CENTER` / mm;
    AUTO: bếp dưới = trên 60, bếp trên = dưới 60, tủ áo = cao 1000 tính từ sàn) · `drill` (bật: khoan xuyên Ø5 theo pitch).
  - Push-open: số bộ theo cao (1 bộ/cánh ≤ 1200, 2 bộ > 1200), bỏ tay nắm.
- **UI**: Thư viện → tab "Phụ kiện" (danh sách, sửa thông số, giá). Chỉnh tấm (cánh/ngăn kéo): ô chọn "Bản lề", "Tay
  nắm", "Vị trí tay nắm"; 2D: kéo tay nắm lên/xuống (direct edit `v_pos`, gõ số). Chuột phải cánh → "Đổi tay nắm…".
- **Core**: `aic_domain::catalog::{HingeType, HandleSpec, SlideType}`; `ProjectSettings.catalog` + thư viện chung.
  `DoorSpec.hinge_code/handle`, `DrawerSpec.slide_code/handle` (serde default). `layout.rs::doors/drawers` đọc spec;
  bỏ `rule_features` hinge cho tấm sinh (hợp nhất luật: B5). Sinh `DrillPurpose::HingeScrew` trên hồi/vách kề phía lề
  (Neighbor.part đã có). `Fittings` đếm theo mã: `BTreeMap<String,u32>`. Costing giá theo `fit:<code>`.
  Request: `get_catalog`, `save_catalog_item`, `delete_catalog_item` (Command `SetCatalogItem`).
- **Event flow**: chọn tay nắm Gola cho cánh → `set_parameter {id: door, name: "handle_type", value: "PROFILE_GOLA"}` →
  zones.rs sửa `DoorSpec.handle` → `SetCabinet` → ChangeSet: xóa hardware tay nắm (deleted), cánh (changed) → patch.
- **Production impact**: khoan đế bản lề trên hồi; khoan lỗ tay nắm trên cánh; báo giá đúng mã phụ kiện; Gola sinh
  thêm thanh nhôm theo mét dài (D09 cho rãnh trên hồi).
- **Nghiệm thu**:
  1. Tủ bếp dưới 2 cánh, tay nắm AUTO → tay nắm cách mép trên cánh 60; tủ bếp trên → cách mép dưới 60 (sửa B4).
  2. Cánh cao 2100 → 4 bản lề (theo `hinge_table`); số chén trên FlatPanelView = số bản lề trong Costing (sửa B5).
  3. Đổi sang PUSH_OPEN → tay nắm biến mất, Costing có "Push-open × 2"; undo khôi phục.

### [P0][M] D04 · Ngăn kéo theo loại ray (bi / âm giảm chấn / tandem box) + ngăn kéo trong
- **Mô tả**: hộc ngăn kéo tính theo `SlideType`: ray bi 3 tầng (hở hông 12.7–13), ray âm (hở hông 5, hộc ngắn hơn
  ray 10, đáy lùi lên 12, khoét rãnh sau), tandem/box kim loại (chỉ cắt đáy + hậu hộc). Có ngăn kéo **trong** (nằm sau
  cánh, mặt lùi) và **mặt ngăn giả**.
- **Tùy chọn**: `slide_code` (catalog; mặc định theo chuẩn) · `box_height` (`AUTO` hoặc mm, 60–400) · `box_t` (= board_t) ·
  `bottom_t` (= back_t) · `bottom_mode` (`UNDER` / `GROOVE` rãnh 10 / `INSET`) · `inner` (bật: ngăn kéo trong,
  `setback` 25) · `false_front` (bật: chỉ mặt, không hộc/ray) · `dividers` (0–4 vách chia hộc).
- **UI**: Tạo tấm → Ngăn kéo: thêm "Loại ray", "Cao hộc", "Ngăn kéo trong", "Mặt giả". Chỉnh tấm (mặt ngăn): cùng các
  field. Dựng nhanh: "Ngăn kéo trong × 2", "Mặt giả (chậu rửa)".
- **Core**: `DrawerSpec` thêm `slide_code`, `box_height: Option<Bay>`, `bottom_mode`, `inner`, `false_front`, `dividers`.
  `layout.rs::drawers` tách `drawer_box(slide, cell)`, thay cứng `sc`, `clamp(60,250)`, `+15`, `−10` bằng thông số
  `SlideType { side_clearance, length_margin, lengths[], box_height_max, bottom_raise, drill_pattern }`. Ray âm sinh
  khoét hậu hộc (pocket) và lỗ khóa ray.
- **Event flow**: mặc định (SetCabinet).
- **Production impact**: kích thước hộc đúng theo ray (đỡ phải cắt lại). Báo giá ray theo mã/chiều dài; tandem không
  có thành gỗ → ít m² ván hơn.
- **Nghiệm thu**:
  1. Khoang 564 rộng, sâu 540, ray âm → hộc rộng = 564 − 2×5 − …, ray 500, hộc sâu 490; Costing "Ray âm 500 × 1".
  2. Ngăn kéo trong 2 tầng sau cánh đôi → mặt ngăn lùi 25 so với mặt thùng, cánh không va (kiểm tra `get_relations` không có PENETRATE).
  3. Cao hộc 300 (ngăn nồi) → hộc cao 300, không bị cắt ở 250 (sửa B8).

### [P0][M] D10 · Dán cạnh theo nhóm tấm + vật liệu Việt Nam + bộ vật liệu
- **Mô tả**: `EdgeRule` thành bảng theo **vai trò tấm**, ví dụ cánh/mặt ngăn: ABS 2 mm 4 cạnh; hồi: PVC 1 mm cạnh trước
  (+ cạnh dưới nếu tủ treo); kệ: 1 cạnh trước; hậu, đáy hộc: không dán. Thêm vật liệu VN và **bộ vật liệu** gắn với
  tủ/phòng.
- **Tùy chọn**: theo từng nhóm `{Thùng, Kệ, Cánh, Mặt ngăn, Hậu, Hộc, Nẹp}`: `mode` (hở bỏ khuất / toàn bộ / trước /
  không) · `band_code` · `band_t` (0.4–3) · `premill` (0/0.5/1 mm, cộng vào trừ kích thước cắt). Vật liệu:
  `kind` thêm `MFC`, `MFC_MR` (lõi xanh chống ẩm), `HDF`, `ACRYLIC`, `LAMINATE`, `VENEER`, `PLYWOOD`; `sheet_w/h` sửa
  được (1220×2440 / 1830×2440); `face_a`/`face_b` (mặt khác nhau). Bộ vật liệu: `{carcass, front, back, drawer_box, edge_front, edge_carcass}`.
- **UI**: Khung → Luật dán cạnh: bảng 7 dòng (nhóm) × cột (mode, chỉ, dày). Thư viện → Vật liệu: thêm field loại, khổ,
  chống ẩm. Thư viện → "Bộ vật liệu" (áp cho tủ/phòng; chuột phải tab phòng → "Áp bộ vật liệu").
- **Core**: `EdgeRule` → `EdgeRuleSet { groups: BTreeMap<RoleGroup, EdgeRule> }` (migrate: mọi nhóm = rule cũ; hậu/hộc
  = không dán thay cho `skip_thicknesses`). `effective_edges` (`costing.rs:65`) chọn rule theo `PanelRole`. `MaterialKind`
  thêm biến thể; `Material.moisture_resistant: bool`. `MaterialSet` trong thư viện; request `apply_material_set {ids | room, name}`
  (Batch `SetCabinetSlot`).
- **Event flow**: đổi chỉ cánh sang ABS 2 → `set_parameter {cab, name: "edge_front_code", value}` → SetCabinet →
  ChangeSet.geometry các cánh (kích thước cắt đổi, kích thước tinh không đổi) → Costing tính lại khi mở.
- **Production impact**: kích thước cắt trừ đúng từng cạnh; mét chỉ tách theo mã; vật liệu chống ẩm cho WC/bếp chậu.
- **Nghiệm thu**:
  1. Tủ bếp dưới: cánh 2 mm 4 cạnh, hồi 1 mm cạnh trước → danh sách cắt: cánh 396×716 → cắt 392×712.
  2. Hậu 9 mm không bị dán dù không có trong `skip_thicknesses` (sửa H19).
  3. Áp bộ vật liệu "Bếp chống ẩm" cho phòng Bếp → mọi thùng đổi sang MFC lõi xanh trong 1 bước undo.

### [P0][M] D11 · Nhãn tấm + danh sách cắt gộp + xuất cho máy cắt
- **Mô tả**: mỗi tấm có **mã tấm** ổn định (`P-<phòng>-<tủ>-<số>`), nhãn in (tên, kích thước cắt, vật liệu, dán cạnh
  vẽ ký hiệu, hướng vân, QR chứa mã). Danh sách cắt gộp chi tiết giống nhau (cùng vật liệu, kích thước, dán cạnh, gia
  công). Xuất CSV theo cột cấu hình được cho máy cắt/phần mềm nesting ngoài.
- **Tùy chọn**: `group_identical` (bật) · `label_size` (`60x40` / `80x50` / `100x70`) · `label_fields` (checklist) ·
  `csv_profile` (tên, dấu phân cách, thứ tự cột, đơn vị, có header) · `code_pattern` (`{room}-{cab}-{n:03}`).
- **UI**: Báo cáo → tab "Danh sách cắt": nút "Gộp giống nhau", "In nhãn", "Xuất CSV (profile)". Cài đặt → "Profile xuất".
- **Core**: `CutRow` thêm `code`, `qty` > 1 khi gộp (khóa gộp = vật liệu + dày + kích thước cắt + mã chỉ 4 cạnh + chữ ký
  gia công). Request `get_cut_list {group: bool, profile?}` trả rows đã gộp; `get_labels {ids?}` trả dữ liệu nhãn (vector
  ký hiệu dán cạnh do core quyết định cạnh nào). Mã tấm lưu trong `Panel.code: Option<String>` (sinh lúc regenerate,
  ổn định theo part key). CSV do core sinh theo profile (UI chỉ tải file).
- **Event flow**: truy vấn thuần (không Command); đổi profile là `SetSetting` (Command `SetCsvProfile`).
- **Production impact**: dán nhãn tại máy cắt, lắp theo mã; giảm dòng danh sách cắt 30–60%.
- **Nghiệm thu**:
  1. 4 tủ bếp dưới 800 giống nhau → "Hồi 560×833" gộp qty 8.
  2. In nhãn A4 (8 nhãn 100×70) → mỗi nhãn có QR giải ra đúng mã tấm.
  3. Xuất CSV profile "Máy cắt KDT" → cột `Mã;Tên;Dài;Rộng;Dày;SL;Vật liệu;Chỉ L1;L2;W1;W2;Vân`, mở được bằng Excel.

### [P0][M] D12 · Báo giá theo mét dài / m² mặt đứng / chi tiết + hao hụt + công, theo phòng
- **Mô tả**: ba cách báo giá hay dùng tại Việt Nam, chọn theo từng tủ hoặc phòng: (a) **mét dài** cho tủ bếp (đơn giá
  riêng tủ dưới/trên/kịch trần); (b) **m² mặt đứng** cho tủ áo, vách, kệ (W × H); (c) **bóc chi tiết** (như hiện tại) +
  hao hụt + công + lợi nhuận. Nhóm theo tầng → phòng → tủ.
- **Tùy chọn**: `pricing_mode` theo tủ (`DETAIL` / `LINEAR_M` / `FACADE_M2` / `PIECE`) · `price_linear` (VND/m) ·
  `price_facade` (VND/m²) · `waste_pct` (10 %, 0–40; hoặc "theo nesting" dùng `waste_ratio` thật) · `labor` (VND/m² ván
  hoặc % vật tư) · `install` (VND/m dài hoặc /tủ) · `transport` · `margin_pct` · `vat_pct` (0/8/10) · `discount`.
- **UI**: Báo cáo → Costing: bộ chọn "Theo phòng / Theo tủ / Toàn dự án"; cột "Cách tính". Chỉnh tấm (tủ) → "Cách báo
  giá". Xuất PDF báo giá (logo, thông tin khách, phòng, tổng bằng chữ) và Excel.
- **Core**: `ProjectSettings.quote: QuoteSettings`; `Cabinet.pricing: Option<PricingMode>`. `costing.rs` thêm tầng
  `quote_for(scope)`; mét dài = W của tủ (tủ góc tính theo cạnh ngoài, D07). Command `SetQuoteSetting` (giống `SetPrice`).
  Request `get_quote {scope: PROJECT | FLOOR | ROOM | CABINET, id?}`; `export_quote {format: PDF | XLSX}` (core trả dữ
  liệu bảng, UI dựng PDF bằng thư viện in; không tính số ở UI).
- **Event flow**: sửa hao hụt → `set_quote_setting` → Command → ChangeSet.settings → `SettingsChanged` → ReportWindow tải lại.
- **Production impact**: báo giá khách đúng thói quen thị trường; vẫn giữ bóc chi tiết để tính giá vốn.
- **Nghiệm thu**:
  1. Bếp: 3 tủ dưới 800+600+900 (LINEAR_M, 4.5 tr/m) → dòng "Tủ bếp dưới 2.3 m = 10.35 tr".
  2. Tủ áo 1800×2400 (FACADE_M2, 3.2 tr/m²) → 4.32 m² = 13.82 tr.
  3. DETAIL + hao hụt "theo nesting" → m² ván = m² tấm / (1 − waste_ratio) của lần nesting gần nhất.

---

## Nhóm B: Khung, kết cấu phụ, sản phẩm theo phòng

### [P0][M] D08 · Chân tủ: len chân 3 mặt, chân nhựa tăng chỉnh, tủ treo
- **Mô tả**: `plinth` hiện chỉ có tấm trước (`layout.rs:521`). Thêm: len chân 3 mặt (trước + 2 hông, cho tủ đầu dãy),
  len chân rời theo dãy (D09), chân nhựa tăng chỉnh (4–6 chân + kẹp len), chân inox/chân gỗ cho kệ TV, tủ treo (ke treo
  + thanh treo tường).
- **Tùy chọn**: `base_type` (`NONE` / `PLINTH` / `PLINTH_3` / `LEGS` / `LEGS_PLINTH` / `HANGING`) · `plinth_height` (100,
  0–200) · `plinth_setback` (50) · `plinth_side_setback` (0–50) · `leg_count` (AUTO: W ≤ 600 → 4, ≤ 1200 → 6, còn lại 8) ·
  `leg_code` (catalog) · `hang_rail` (bật: thanh treo 17 × 60 sau hậu, lùi hậu tương ứng) · `hanger_code`.
- **UI**: Thuộc tính kết cấu → tab "Len chân" đổi thành "Chân / treo". 2D view Trước/Bên: số cao chân sửa trực tiếp
  (direct edit `plinth_height`), kéo len chân vào/ra (`plinth_setback`).
- **Core**: `StructureRules.base: BaseRule`. `layout.rs::carcass` sinh `c:plinth`, `c:plinth_l`, `c:plinth_r`, hardware
  `c:leg:n` (`HardwareKind::Leg` đã có hình trong `aic-geometry/src/panel.rs:186`), `c:hang_rail`. Bỏ ràng buộc kind
  (sửa B7): `base_type` quyết định.
- **Event flow**: mặc định.
- **Production impact**: thêm tấm len hông vào danh sách cắt; chân nhựa, ke treo vào báo giá.
- **Nghiệm thu**:
  1. Tủ áo đầu dãy, `PLINTH_3` → danh sách cắt có ChânTủ + 2 ChânHông dài = sâu − 50.
  2. Tủ bếp dưới 900, `LEGS_PLINTH` → 6 chân nhựa, len chân kẹp.
  3. Tủ bếp trên `HANGING` → không có chân, có 2 ke treo + thanh treo 17×60 dài = rộng trong.

### [P0][M] D09 · Dãy tủ (Run): mặt đá, chỉ chân chạy suốt, tấm lấp, che trần
- **Mô tả**: đối tượng mới **Dãy** gom các tủ liền nhau trong một phòng (dãy dưới, dãy trên). Dãy sinh các phần *chạy
  qua nhiều tủ*: mặt đá (outline + khoét chậu/bếp), len chân liền, tấm lấp đầu dãy (sát tường), tấm che hở trần (tủ trên
  kịch trần), nẹp/đèn hắt. Tủ trong dãy tự bỏ len chân riêng.
- **Tùy chọn**: `run_kind` (`BASE` / `WALL` / `TALL`) · `countertop` (bật, dày 20/30, nhô trước 20, nhô hông 0/20, vật
  liệu đá/gỗ, khoét: chậu `{x, w, d, r}`, bếp `{x, w, d}`) · `continuous_plinth` (bật) · `filler_left/right` (mm, 0–150,
  kiểu `FLAT` / `L` 2 tấm) · `ceiling_filler` (bật, cao = trần − đỉnh tủ, lùi 0) · `wall_gap` (khe bù tường 3–10).
- **UI**: chọn nhiều tủ → chuột phải "Tạo dãy". Scene tree: node "Dãy" chứa tủ. Thuộc tính dãy: tab "Mặt đá", "Len chân",
  "Tấm lấp". 2D: kéo tấm lấp (direct edit khe), bấm số tổng dài dãy.
- **Core**: `DomainObject::Run { id, room, kind, cabinets: Vec<ObjectId>, rules: RunRules }` trong `aic-domain`. Generator
  `run::build(run, cabinets' world AABB) -> Vec<Part>` (thuần, như `layout::build`). Command mới: `CreateRun`,
  `SetRun { id, run }` (inverse = run cũ), xóa run qua `DeleteObject`. Dependency: `resize_cabinet` / `move_objects` của tủ
  thuộc dãy đánh dấu dãy dirty → regenerate run trong cùng Batch. Mặt đá là tấm có `ContourFeature` (khoét) nên đi
  được vào báo giá m².
- **Event flow**: đổi rộng tủ giữa dãy → `resize_cabinet` → Batch{SetParameter, dịch tủ liền kề (đã có), `SetRun` regen}
  → ChangeSet: tủ + tủ kề (transform) + mặt đá/len chân (geometry) → patch 3D/2D.
- **Production impact**: mặt đá tính m² và mét dài (báo giá D12). Tấm lấp và che trần có trong danh sách cắt, có dán cạnh.
  Len chân liền giảm mối nối.
- **Nghiệm thu**:
  1. Dãy dưới 800+600+900 + mặt đá nhô 20 → mặt đá 2300 × 620 × 20 (không có tấm đá rời từng tủ).
  2. Đổi tủ 600 → 700 → mặt đá tự thành 2400, len chân liền 2400 − 2×50 (nếu có tấm lấp thì trừ), một bước undo.
  3. Tủ trên kịch trần, trần 2700, đỉnh tủ 2550 → tấm che trần cao 150 dài = dãy.

### [P0][L] D07 · Tủ góc bếp: góc L (blind corner), góc chéo, mâm xoay
- **Mô tả**: generator mới cho 3 loại tủ góc. **Góc L mù** (tủ 1000–1200 có tấm che mù 100–150 vuông góc với dãy kia).
  **Góc chéo** (mặt cánh xiên 45°, đáy/nóc 5 cạnh, 2 hồi 90°). **Tủ góc cho mâm xoay/Magic corner** (khoang rỗng có kích
  thước tối thiểu của phụ kiện).
- **Tùy chọn**: `corner_type` (`BLIND` / `DIAGONAL` / `LAZY_SUSAN`) · `hand` (`LEFT` / `RIGHT`) · `w1`, `w2` (cạnh dọc
  hai tường, 800–1200) · `d1`, `d2` (sâu 2 dãy, 560–600) · `blind_width` (150) · `door_width` (450) · `filler` (50,
  đoạn bù chống va tay nắm) · `accessory_code` (mâm xoay/Magic corner kiểm tra khoang tối thiểu).
- **UI**: Khung → Kiểu khung "07. BếpGóc" + nhóm tham số riêng. 2D view Trên: kéo w1/w2. Chuột phải tủ cuối dãy →
  "Thêm tủ góc (trái/phải)" tạo góc + xoay 90° dãy kế tiếp.
- **Core**: `CabinetKind::Corner` + `CornerSpec` trong `Cabinet`. `layout::build` phân nhánh `carcass_corner(...)` (vẫn
  xuất `ZBox` gốc cho zone tree bên trong, để kệ/cánh dùng lại). Đáy/nóc chéo dùng `ContourFeature` ngoài (polygon 5
  cạnh) — feature local của tấm. Cánh chéo dùng rotation 45° (transform không scale). Constraint:
  `door_width ≥ 250`, khoang ≥ kích thước phụ kiện → `CONSTRAINT_VIOLATED {constraint: "ACCESSORY_TOO_BIG"}`.
- **Event flow**: mặc định (SetCabinet/SetParameter); tạo tủ góc cùng dãy = Batch{CreateCabinet, SetRun}.
- **Production impact**: đáy/nóc 5 cạnh có contour trên CNC (đã hỗ trợ contour). Báo giá mét dài tủ góc = w1 + w2 − d.
- **Nghiệm thu**:
  1. Góc L mù 1100 trái → tủ 1100 + tấm che mù 150 + tấm lấp 50; tủ dãy vuông góc bắt đầu sát tấm lấp.
  2. Góc chéo 900×900 sâu 580 → đáy polygon 5 đỉnh, cánh rộng = √2 × (900 − 580) ≈ 452 (trừ khe).
  3. Mâm xoay 3/4 Ø700 vào khoang 800×800 → OK; khoang 700 → báo lỗi tiếng Việt "Khoang nhỏ hơn phụ kiện".

### [P1][M] D13 · Kết cấu phụ: phào nóc, phào chân, ốp hông, tấm che, nẹp
- **Mô tả**: sinh theo luật trên tủ (hoặc theo dãy khi có D09): phào nóc (1–3 mặt), phào chân, ốp hông (tấm dày phủ hồi
  ngoài, có thể chạm sàn), nẹp che khe giữa 2 tủ.
- **Tùy chọn**: `cornice` {bật, cao 60 (20–200), nhô 20, mặt `FRONT` / `FRONT_LEFT` / `FRONT_RIGHT` / `3_SIDES`, vật liệu,
  góc nối `MITER_45` / `BUTT`} · `end_panel` {trái/phải, dày = board_t hoặc 25, nhô trước 0–20, chạm sàn bật/tắt} ·
  `scribe_strip` (nẹp, rộng 30–60).
- **UI**: Thuộc tính kết cấu → tab "Phào & ốp". Lưu mẫu tab.
- **Core**: `StructureRules.cornice/end_panels`; sinh `c:cornice_f/l/r`, `c:end_l/r` trong `layout::carcass`; cắt vát
  45° = `CutLine` lưu thành contour local.
- **Event flow**: mặc định.
- **Production impact**: 1–3 thanh phào có góc vát trên CNC, báo giá theo mét dài; ốp hông dán cạnh 4 cạnh.
- **Nghiệm thu**: tủ 1600 → 1800 thì phào nóc trước tự dài 1800 + 2×20, phào hông không đổi; danh sách cắt có 3 thanh với ghi chú "vát 45°".

### [P1][M] D14 · Kệ nghiêng thật, vách lửng, kệ góc
- **Mô tả**: dùng `tilt_deg` (đang bị bỏ qua, B1) để sinh kệ nghiêng cho tủ giày (nghiêng 15–20°, có thanh chặn gót).
  Thêm vách lửng (không chạm nóc/đáy, cao theo mm) và kệ góc L (kệ góc trong tủ góc).
- **Tùy chọn**: `tilt_fb` (0–30°) · `heel_stop` (bật, cao 20) · `partial_divider` {`height` mm hoặc %, `from` `BOTTOM` / `TOP`} ·
  `corner_shelf` (polygon theo khoang).
- **UI**: Tạo tấm → Tấm ngang: "Nghiêng" (đã có ô) có hiệu lực; Dựng nhanh "Kệ giày nghiêng × n". 2D view Bên: kéo góc.
- **Core**: `layout.rs::split_panel` áp rotation `[−90 + tilt, 0, 0]` quanh cạnh sau; tính lại chiều sâu kệ để không vượt
  khoang (`d / cos`); `SplitKind::DividerPartial` + field `extent`.
- **Event flow**: mặc định.
- **Production impact**: chốt đỡ kệ nghiêng (hoặc ke), kích thước cắt theo sâu nghiêng; thanh chặn gót vào danh sách cắt.
- **Nghiệm thu**: tủ giày 1200 × 1000 × 350, 4 kệ nghiêng 15° → 3D thấy kệ nghiêng; kệ sâu 300 (không chạm cánh); mỗi kệ có 1 thanh chặn gót.

### [P1][M] D15 · Hậu: khoét hậu, hậu ốp, hậu dừng tại kệ cố định
- **Mô tả**: khoét lỗ trên hậu cho ổ điện, ống nước, thoát nhiệt tủ lò/tủ lạnh; kiểu hậu ốp (bắt vít sau hồi); hậu chia
  theo kệ cố định (mỗi khoang một tấm hậu).
- **Tùy chọn**: `back_mode` (`GROOVE` / `INSET` / `OVERLAY_SCREW`) · `cutouts: [{x, y, w, h, r, kind: SOCKET | PIPE | VENT}]`
  (tọa độ theo khoang để khi tủ đổi cỡ vẫn đúng chỗ) · `split_at_fixed` (bật).
- **UI**: Chỉnh tấm (hậu) → "Khoét hậu" + danh sách; 2D view Trước khi ẩn cánh: vẽ khoét, kéo được (direct edit).
- **Core**: `BackRule.cutouts` (neo theo zone uid + offset), sinh `ContourFeature { inner: true }` local trên tấm hậu.
- **Event flow**: mặc định.
- **Production impact**: CNC cắt lỗ trong; không phải khoan tay tại công trình.
- **Nghiệm thu**: tủ lavabo 800, khoét ống Ø60 cách đáy 250, giữa → đổi rộng 800 → 900 thì lỗ vẫn ở giữa.

### [P1][L] D16 · Tủ áo cánh lùa theo hệ ray + cánh kính / khung nhôm
- **Mô tả**: `DoorKind::Sliding` hiện chia đều, chồng 30 cứng. Thêm hệ ray (catalog): số ray 2/3, số cánh 2/3/4,
  độ chồng theo hệ, trừ kích thước cánh theo bánh xe (trên/dưới), khung nhôm (profile + kính/gỗ nhét), vách chia cánh
  lùa (nẹp ngang).
- **Tùy chọn**: `system_code` · `leaves` (2–6) · `tracks` (2/3) · `overlap` (theo hệ, 20–50) · `height_deduct_top/bottom` ·
  `frame` (`NONE` / `ALU_THIN` / `ALU_WIDE`) · `infill` (`BOARD` / `GLASS` / `MIRROR`) · `rails_h` (0–3 nẹp ngang).
- **UI**: Tạo tấm → Cánh → Lùa: nhóm "Hệ ray". Chỉnh tấm: chọn cánh lùa → thứ tự ray.
- **Core**: `DoorSpec.sliding: Option<SlidingSpec>`; `layout.rs::doors` (nhánh Sliding) đọc spec thay `overlap = 30.0`.
  Khung nhôm là hardware có chiều dài (báo giá theo m), kính là part "Glass" (không vào nesting ván).
- **Event flow**: mặc định.
- **Production impact**: đúng kích thước cánh; profile nhôm theo mét; kính theo m².
- **Nghiệm thu**: tủ 2400 × 2600, 3 cánh 2 ray, chồng 35 → 3 cánh rộng (2400 + 2×35)/3; ray 2400; kính 3 tấm.

### [P1][M] D17 · Cánh lật / cánh gập / cánh kính khung nhôm
- **Mô tả**: cánh lật lên (tay nâng Aventos HK/HF/HL), lật xuống, cánh gập 2 lá; cánh khung nhôm kính cho tủ trên.
- **Tùy chọn**: `lift_type` (`HK` / `HF` / `HL` / `STRUT`) · `lift_power` (auto theo khối lượng cánh = dày × diện tích × tỷ trọng) ·
  `fold_leaves` (2) · `glass_frame` (profile, rộng 20/45).
- **UI**: Dựng nhanh "Cánh lật (tay nâng)"; Chỉnh tấm cánh → "Loại mở".
- **Core**: `DoorKind::{LiftUp, Fold}`; catalog tay nâng có vùng lắp tối thiểu (khoang cao 350–800) → ràng buộc.
- **Event flow**: mặc định.
- **Production impact**: khoan lắp tay nâng trên hồi; báo giá theo mã.
- **Nghiệm thu**: tủ trên 800 × 400, cánh lật HK → 1 bộ HK, 2 lỗ chén trên cánh; khoang cao 300 → báo lỗi "Khoang quá thấp cho tay nâng".

### [P1][M] D18 · Phụ kiện trong khoang: giá bát, rổ gia vị, giá giày, giá kéo, đèn LED
- **Mô tả**: `LinkKind` mở rộng thành phụ kiện catalog gắn vào zone: kiểm tra kích thước khoang (rộng/sâu/cao tối thiểu),
  sinh hardware + khoan lắp, đếm vào báo giá. Đèn LED: rãnh (tool 16 đã có) + nguồn + dây theo mét.
- **Tùy chọn**: `accessory_code` · `fit` (`EXACT_WIDTH` / `MIN_WIDTH`) · `offset_top` · `led` {`profile`, `watt_per_m`, `driver_code`, `switch` (cảm biến cửa / chạm)}.
- **UI**: Tạo tấm → nút tròn "Phụ kiện" (thay "Liên kết"), lọc theo khoang vừa. Dựng nhanh: "Giá bát 800", "Rổ gia vị 200".
- **Core**: `LinkKind::Accessory { code }`; `Layout.fittings.by_code`; ràng buộc khi resize tủ: nếu khoang < min →
  `problems` + cảnh báo trong `get_zones` (không chặn resize, đánh dấu đỏ).
- **Event flow**: mặc định; resize làm phụ kiện không vừa → ChangeSet + `problems` → 2D tô đỏ khoang.
- **Production impact**: báo giá phụ kiện theo mã; khoan lắp giá kéo trên hồi.
- **Nghiệm thu**: khoang 764 → giá bát 800 báo "không vừa"; đổi tủ rộng để khoang 800 → hết cảnh báo.

---

## Nhóm C: Sản phẩm ngoài tủ hộp

### [P1][L] D19 · Khung sản phẩm tổng quát (Product) + generator Giường
- **Mô tả**: tách khái niệm **Sản phẩm** khỏi `CabinetKind` đóng. `ProductKind::{Cabinet, Bed, Desk, TvUnit, WallCladding, Island}`
  dùng chung hạ tầng (tham số, part key, PartMod, anchor, template). Generator đầu tiên là **Giường**: đầu giường, 2 vai
  (thành), đuôi, dát (nan hoặc tấm), đà giữa, chân; tùy chọn hộc kéo 2 bên/đuôi hoặc nâng hơi.
- **Tùy chọn**: `mattress_w` (1200/1400/1600/1800), `mattress_l` (2000) · `frame_h` (mặt dát cao 350–450) ·
  `headboard_h` (900–1200), `headboard_t`, `headboard_style` (`FLAT` / `UPHOLSTERED` / `SLATTED`) · `side_rail_h` (200–300) ·
  `slats` (`BOARD` / `SLATS` n = 12–18) · `center_beam` (bật khi W ≥ 1400) · `legs` (4/6, cao) · `storage` (`NONE` /
  `DRAWERS_2_SIDES` / `DRAWERS_FOOT` / `GAS_LIFT`) · `drawer_count` (2–4 mỗi bên).
- **UI**: Khung → Kiểu "Giường" với tab thuộc tính riêng ("Kích thước nệm", "Đầu giường", "Thành & dát", "Hộc kéo").
  2D view Trên/Trước: sửa W/L nệm, cao dát.
- **Core**: `aic-domain::product` + `bed::build(spec, values) -> Layout` (thuần). `Cabinet` đổi tên nội bộ không cần:
  thêm `kind: CabinetKind::Bed` và `bed: Option<BedSpec>` (serde default) để giữ file cũ; về sau mới tách enum.
  Hộc kéo tái dùng `drawers()` (D04). Constraint: `mattress_w ∈ [800, 2200]`.
- **Event flow**: mặc định.
- **Production impact**: danh sách cắt giường; ray hộc kéo; ben hơi; báo giá theo chiếc (`PIECE`).
- **Nghiệm thu**:
  1. Giường 1600×2000 cao dát 400 → lọt nệm 1600×2000, vai dài 2000 + 2t, có đà giữa.
  2. Hộc kéo 2 bên × 2 → 4 ngăn kéo, ray theo sâu hộc, không va chân giường.
  3. Lưu template "Giường 1m6 hộc kéo" → chèn với W = 1800 → mọi chi tiết giải lại.

### [P1][M] D21 · Generator Bàn (học / làm việc / trang điểm)
- **Mô tả**: mặt bàn, 2 chân tấm hoặc hộc tủ + chân, yếm (modesty panel), hộc bàn phím, kệ trên (bàn học), gương + ngăn
  kéo nông (bàn trang điểm), khoét luồn dây.
- **Tùy chọn**: `W` 1000–1800, `D` 500–700, `H` 750 · `top_t` 25 (17–40) · `top_overhang` 0–50 · `support_left/right`
  (`PANEL` / `DRAWER_UNIT` / `LEG`) · `modesty` (bật, cao 300, lùi 50) · `keyboard_tray` · `hutch` {cao, số kệ} ·
  `cable_hole` (Ø60, vị trí) · `mirror` (trang điểm).
- **UI**: Kiểu khung "Bàn" + tab thuộc tính. Direct edit 2D: W/D/H, vị trí khoét dây.
- **Core**: `ProductKind::Desk`, `desk::build`, dùng lại `drawers()` cho hộc.
- **Event flow**: mặc định.
- **Production impact**: mặt bàn dày ghép (tool 14 đã có); khoét dây là contour trong.
- **Nghiệm thu**: bàn học 1200 × 600, hộc phải 3 ngăn, kệ trên 2 tầng → danh sách cắt đủ, hộc rộng 400 theo LOCK.

### [P1][M] D22 · Kệ TV treo / đặt sàn + vách ốp TV / lam gỗ
- **Mô tả**: kệ TV = dãy tủ thấp (tái dùng Base, sâu 350–450, cao 300–450) + tùy chọn treo (ke ẩn). Vách ốp = sản phẩm
  mới: lưới tấm ốp (chia theo mô-đun, khe V 3–5 mm), lam gỗ (lam rộng 40–60, khe 20–40, dày 17–25), khoét hộp điện.
- **Tùy chọn** (vách): `W`, `H`, `module_w` (600 / AUTO) · `joint_gap` 3 · `joint_type` (`V_GROOVE` / `SHADOW_GAP` / `NONE`) ·
  `batten` {rộng 40, khe 25, dày 20, hướng dọc/ngang} · `backing` (khung xương/ván lót) · `cutouts` (TV bracket, ổ điện).
- **UI**: Kiểu khung "Vách ốp" / "Lam". 2D Trước: kéo đường chia mô-đun (dùng lại zone split + `parse_split_formula`).
- **Core**: `ProductKind::WallCladding` dùng **ZoneTree 2D** (axis 0/1, `VirtualV/VirtualH`) — mỗi ô lá sinh 1 tấm ốp;
  không cần hồi/nóc. Lam: array tấm theo công thức `split_formula`.
- **Event flow**: mặc định.
- **Production impact**: danh sách cắt tấm ốp/lam; mét dài lam; báo giá m² mặt đứng.
- **Nghiệm thu**: vách 3000 × 2700, công thức `/5` dọc → 5 tấm 596 × 2700 (khe 3); đổi W thành 3200 → tấm 636.

### [P2][M] D23 · Bàn đảo / quầy bar (tủ mở 2 mặt)
- **Mô tả**: tủ sâu 900–1200 mở 2 mặt (cánh cả trước và sau), mặt đá nhô 250–300 phía ghế ngồi, ốp lưng/ốp hông.
- **Tùy chọn**: `double_sided` (bật) · `back_zone_depth` · `seat_overhang` 250 · `end_panels` (D13).
- **UI**: Kiểu khung "Bàn đảo". Tạo tấm: vùng mặt sau có tab riêng ("Mặt trước / Mặt sau").
- **Core**: `Cabinet.rear_zones: Option<ZoneTree>`; `layout::build` chạy `zone()` lần hai với ZBox mặt sau (Z ngược).
- **Event flow**: mặc định.
- **Production impact**: không có hậu; thêm vách giữa; mặt đá (D09) một khối.
- **Nghiệm thu**: bàn đảo 1800 × 900, mặt trước 3 ngăn kéo, mặt sau kệ mở → không có tấm hậu, vách giữa sâu 900.

---

## Nhóm D: Tương tác tham số & hiệu năng

### [P1][M] D20 · ChangeSet cấp zone/dimension + 2D incremental patch
- **Mô tả**: đạt DoD §35.13. Core trả thêm `zonesUpdated`, `dimensionsUpdated`, `relationsUpdated`; 2D chỉ vẽ lại phần
  bị ảnh hưởng.
- **Tùy chọn**: không (hạ tầng).
- **UI**: `useZones` nhận patch thay cho tải lại toàn bộ; Drawing2D tách `items` theo `ObjectId` (Map) và chỉ tính lại id
  có trong event; SVG dùng `key={id}` và `React.memo` theo item.
- **Core**: `ChangeSet` thêm `zones: BTreeMap<ObjectId /*cabinet*/, BTreeSet<Uid>>`, `dims: BTreeSet<ObjectId>`. Trong
  `regenerate_inner` so sánh `Layout.zones/bays/front_bays` cũ và mới (layout cũ cache theo cabinet) → diff. CoreEvent
  mới `ZonesChanged { cabinet, zones: [ZoneBox], bays: [BayInfo], removed: [Uid] }`. `get_zones` giữ nguyên cho lần đầu.
  Build layout vẫn là toàn tủ (hàm thuần, rẻ), chỉ phần **phát sự kiện và vẽ** là incremental.
- **Event flow**: kéo vách → `move_split_panel` → SetCabinet → regenerate (diff part + diff zone) → events
  `GeometryChanged{2 kệ}` + `ZonesChanged{cabinet, 2 khoang}` → sceneSync lấy mesh 2 tấm; useZones vá 2 khoang; Drawing2D
  tính lại 2 item.
- **Production impact**: không; hiệu năng cho dự án cả căn hộ (≥ 1000 tấm).
- **Nghiệm thu**:
  1. Kéo vách trong tủ có 40 tấm → chỉ 2 tấm + 2 khoang trong events (test API).
  2. Đo: dự án 60 tủ, sửa 1 khoang → thời gian vẽ lại 2D < 16 ms (Performance panel).
  3. Undo trả về đúng events đối xứng.

### [P1][M] D25 · Preview do core tính (dry-run) cho kéo vách/kệ/tủ/cạnh
- **Mô tả**: hiện preview kéo vách tính ở UI (`EditLayer.tsx:449–460`). Với LOCK/AUTO/% nhiều khoang, ràng buộc anchor,
  phụ kiện, UI khó đoán đúng. Thêm `preview: true` cho các request kéo: core chạy trên bản sao, trả layout tạm (chỉ các
  tấm/khoang đổi), **không** vào history, không tăng revision.
- **Tùy chọn**: `throttle_ms` (UI, 30) · `preview_detail` (`BOXES` / `MESH`).
- **UI**: EditLayer gọi `move_split_panel {…, preview: true}` với throttle; vẽ khung tạm từ kết quả; thả chuột → gọi thật.
- **Core**: `Engine::dry_run(req)` = clone `Cabinet` def + `build_cabinet` (hàm thuần, không đụng Document) → trả
  `{parts: [{key, box}], bays, problems}`. Không cần Command mới.
- **Event flow**: Preview: Action kéo → request preview → core build tạm → response (không events) → UI vẽ overlay.
  Commit: như hiện tại.
- **Production impact**: không.
- **Nghiệm thu**: tủ `600 KHÓA · AUTO · 400 KHÓA`, kéo vách 1 → preview hiện khoang giữa co lại, khoang KHÓA 400 không đổi; Esc → không có undo entry.

### [P1][M] D03 · Direct edit 2D mở rộng
- **Mô tả**: thêm các số sửa trực tiếp trên 2D: D (view Bên), lùi kệ/offset trước, khe cánh từng phía, cao chân, cao độ
  tay nắm, cách trên thanh treo, khe tấm lấp, dày tấm; chuỗi kích thước tổng dãy.
- **Tùy chọn**: `show_dims` (nhóm: Tủ / Khoang / Cánh / Offset / Phụ kiện) · `dim_style` (chuỗi / tổng).
- **UI**: Drawing2D: số màu cam = sửa được; bấm → ô nhập tại chỗ (đã có cơ chế). View Bên: số D, lùi kệ. Chuột phải số →
  "Khóa" / "AUTO".
- **Core**: request hiện có (`set_parameter`, `set_part_mod` offsets, `door_gap_*`). Thêm vào `get_zones` danh sách
  `dims: [{kind, target, value, editable, request}]` do core tính vị trí (UI không tự suy ra cạnh nào là offset).
- **Event flow**: bấm số lùi kệ 20 → nhập 30 → `set_parameter {id: shelf, name: "off_front", 30}` → SetCabinet →
  GeometryChanged kệ → patch (D20).
- **Production impact**: không (nhanh hơn, ít lỗi nhập).
- **Nghiệm thu**: view Bên tủ áo: bấm "600" (D) nhập 580 → tủ sâu 580, kệ tự ngắn 20, ray treo vẫn giữa; một bước undo.

### [P1][M] D27 · Căn / phân bố / xoay 90° / đặt theo tường
- **Mô tả**: căn trái/phải/trên/dưới/giữa, phân bố đều, xoay 90° quanh góc tủ, đặt sát tường phòng (Room đã có kích
  thước + tường).
- **Tùy chọn**: `align` (`LEFT` / `RIGHT` / `TOP` / `BOTTOM` / `CENTER_X` / `CENTER_Y` / `BACK`) · `distribute` (`GAP` / `CENTER`) ·
  `rotate` (±90, pivot `LEFT_BACK` / `RIGHT_BACK` / `CENTER`) · `to_wall` (tường gần nhất, khe 0–10).
- **UI**: Ribbon → nhóm "Căn chỉnh"; phím tắt `R` xoay 90°, `Shift+R` −90°; chuột phải nhiều tủ.
- **Core**: request `align_objects {ids, mode}`, `distribute_objects {ids, mode}`, `rotate_objects {ids, deg, pivot}`,
  `snap_to_wall {ids, gap}` → Batch `SetTransform` (một bước undo). Tính trong `aic-spatial` (AABB đã có).
- **Event flow**: TransformChanged cho các tủ → UI cập nhật ma trận.
- **Production impact**: không; giảm lỗi đặt tủ.
- **Nghiệm thu**: 3 tủ trên cao độ khác nhau → Căn trên → cùng đỉnh; undo 1 lần trả cả 3.

### [P2][S] D28 · Array trong UI theo mọi trục + array theo công thức
- **Mô tả**: core `array_cabinet` đã có `axis` và `gap`, UI chỉ gọi axis 0 gap 0 (`Chrome.tsx:433–434`). Mở hộp thoại:
  số lượng, trục (X/Y/Z), khe, hoặc công thức `3*600` (dùng lại `parse_split_formula`).
- **UI**: chuột phải tủ → "Nhân dãy tủ…" (hộp thoại). **Core**: không đổi, thêm `widths: Option<Vec<f64>>`.
- **Event flow**: ObjectCreated. **Production**: không.
- **Nghiệm thu**: tủ trên 600, nhân 3 trục X khe 0 với công thức `400,600,800` → 3 tủ rộng đúng thứ tự.

### [P1][S] D24 · Chuyển tính feature của tool 03/08/16/18 về core + một bước undo
- **Mô tả**: sửa B3. Tool gửi ý định (góc, kích thước, cạnh neo) thay vì tọa độ tuyệt đối; core tính theo kích thước tấm
  **tại thời điểm dựng**, nên tấm đổi cỡ thì khấu góc vẫn đúng góc.
- **Tùy chọn**: giữ UI; thêm `anchor` (`TL` / `TR` / `BL` / `BR` / `CENTER`) cho pocket.
- **Core**: `ShapeOp::Notch { corner, w, d }`, `ShapeOp::Pocket { anchor, x, y, w, h, depth, side }`, `ShapeOp::LedGroove {…}`
  lưu trong `PartMod` dạng **tham số** (không phải `MachiningFeature` tuyệt đối), giải trong `apply_mods`. `shape_tool {ids}`
  đã là một request cho nhiều id → một Batch.
- **Event flow**: GeometryChanged các tấm.
- **Production impact**: khấu góc đúng sau khi đổi kích thước; CNC đúng.
- **Nghiệm thu**: khấu góc TR 100×100 trên hồi 560 → đổi sâu tủ 600 → lỗ khấu vẫn ở góc TR (x = 500); 3 tấm = 1 undo.

---

## Nhóm E: Bản vẽ, thư viện, xuất máy

### [P1][M] D29 · Bản vẽ in A3/A4 có khung tên + bản vẽ lắp đặt theo phòng
- **Mô tả**: layout in gồm mặt đứng các tủ trong phòng, mặt bằng, chi tiết tủ (Trước/Bên/Mặt cắt), kích thước tự động,
  ký hiệu chiều mở cánh, khung tên (dự án, phòng, người vẽ, ngày, tỷ lệ).
- **Tùy chọn**: `paper` (A4/A3, dọc/ngang) · `scale` (1:10/1:20/1:25/auto) · `title_block` (mẫu) · `views` (checklist) · `hide_fronts`.
- **UI**: nút "Xuất bản vẽ" trên thanh 2D → xem trước trang → PDF.
- **Core**: request `get_drawing_sheet {scope, views}` trả **primitive vector** (đường, text, dim) đã chiếu (core chiếu từ
  layout; UI chỉ vẽ). Ký hiệu mở cánh dựa trên `Panel.hinge`.
- **Event flow**: truy vấn.
- **Production impact**: bản vẽ cho thợ lắp và khách ký duyệt.
- **Nghiệm thu**: phòng Bếp 5 tủ → 1 trang A3: mặt đứng dãy dưới + trên, chuỗi kích thước tổng khớp tổng W các tủ.

### [P2][L] D26 · Xuất DXF / MPR (Homag) / CIX (Biesse) + khoan mặt B và cạnh
- **Mô tả**: STATUS L7 nói chỉ gia công mặt A. Xuất file theo tấm cho máy khoan CNC/khoan ngang; DXF theo layer
  (`CUT`, `DRILL_5`, `DRILL_8`, `POCKET_<depth>`) cho phần mềm CAM ngoài.
- **Tùy chọn**: `format` (`DXF` / `MPR` / `CIX`) · `layer_map` · `flip_for_b` (bật) · `origin` (góc trái dưới / trên).
- **UI**: Gia công → "Xuất file máy" (theo tấm / theo sheet nesting).
- **Core**: `aic-manufacturing::export::{dxf, mpr, cix}` từ `FlatPanel` (đã có features local).
- **Event flow**: truy vấn.
- **Production impact**: dùng được máy khoan ngang / máy có khoan cạnh.
- **Nghiệm thu**: hồi có cam + chốt → DXF có layer `DRILL_15_12.5` và `HDRILL_8_25` đúng tọa độ.

### [P1][M] D30 · Bộ template dựng sẵn theo sản phẩm Việt Nam (§7) + template có tham số
- **Mô tả**: nạp sẵn ≥ 13 template (§7) vào thư viện builtin; template có **tham số hiển thị** (số cánh, có ngăn kéo,
  kịch trần) để form chèn đơn giản.
- **Tùy chọn**: `template.params: [{key, label, kind, default, options}]` · `category` (Bếp / PN / Khách / WC) · `thumbnail`.
- **UI**: Khung → "Template" dạng lưới ảnh theo phòng; chọn → form W/H/D + tham số → [TAB].
- **Core**: `builtin_templates()` như `builtin_presets()` (`templates.rs:19`); `CabinetTemplate.params` + hàm `apply_params`
  sửa def trước khi `insert_template`. Ảnh thumbnail: core trả layout, UI render offscreen một lần và cache.
- **Event flow**: ObjectCreated.
- **Production impact**: tạo nhanh sản phẩm đúng chuẩn; ít sai.
- **Nghiệm thu**: chèn "Tủ áo 4 cánh 1800" với W = 2000, kịch trần = bật, trần 2700 → tủ cao 2700 − che trần, 4 cánh rộng đều, 2 khoang treo + 1 khoang kệ.

### [P2][M] D31 · Thư viện nhóm / đồng bộ
- **Mô tả**: thư viện hiện là file máy (`~/.aic-cad/library.json`). Thêm nguồn thư viện "Công ty" (thư mục mạng hoặc
  URL), chỉ đọc cho thợ, ghi cho trưởng phòng; phiên bản; so sánh khi trùng tên.
- **Tùy chọn**: `library_sources: [{name, path | url, readonly}]` · `conflict` (`KEEP_LOCAL` / `USE_REMOTE`).
- **UI**: Cài đặt → "Thư viện". **Core**: `aic-api/src/library.rs` nhiều nguồn; không Command (ngoài dự án).
- **Event flow**: SettingsChanged. **Production**: đồng bộ chuẩn xưởng giữa các máy.
- **Nghiệm thu**: 2 máy cùng trỏ thư mục chung → chuẩn "MFC 18" lưu ở máy A hiện ở máy B.

### [P2][S] D32 · Nối vân nhiều cánh + hướng vân theo tấm trong nesting
- **Mô tả**: các cánh cạnh nhau (hoặc cánh + mặt ngăn kéo) lấy từ **cùng một tấm ván**, vân chạy liên tục; nesting
  đặt chúng liền nhau theo thứ tự.
- **Tùy chọn**: `grain_group` (tên nhóm) · `grain_continuous` (`H` / `V`).
- **UI**: chọn nhiều cánh → chuột phải "Nối vân". **Core**: `PartMod.grain_group`; `aic-nesting` đặt nhóm như 1 khối.
- **Event flow**: ObjectChanged. **Production**: đẹp mặt tủ veneer/vân gỗ.
- **Nghiệm thu**: 4 cánh tủ áo nối vân → nesting đặt 4 cánh liền nhau trên cùng sheet, cùng hướng.

### [P2][M] D33 · Tủ lavabo / tủ chống ẩm / tủ thờ / tủ máy giặt (preset + luật)
- **Mô tả**: không cần generator mới; cần **luật theo phòng**: phòng WC → vật liệu chống ẩm, chân nhựa, khoét ống
  (D15), không đáy phía sau ống. Tủ máy giặt: khoang trống theo máy (600 × 850 × 600 + khe 20). Tủ thờ: hộc kéo + khoang
  trang trí + đèn.
- **Tùy chọn**: `room_rules: {room_type → material_set, base_type, edge_rule}` · `appliance_bays: [{code, w, h, d, clearance}]`.
- **UI**: tab phòng → "Loại phòng" (Bếp/WC/PN/Khách). Dựng nhanh "Khoang máy giặt".
- **Core**: `RoomRules` trong `ProjectSettings`; áp khi `create_cabinet` với `room` tương ứng. `LinkKind::ApplianceBay`
  (khoang trống có kiểm tra kích thước).
- **Event flow**: ObjectCreated. **Production**: vật liệu đúng môi trường ẩm.
- **Nghiệm thu**: tạo tủ trong phòng "WC1" (loại WC) → thùng MFC lõi xanh, chân nhựa, hậu có khoét ống mặc định.

### [P1][S] D34 · Khoang thiết bị: tủ lò / lò vi sóng / tủ lạnh âm / máy rửa bát
- **Mô tả**: khoang trống kích thước theo thiết bị (catalog), có thanh đỡ lò, khe thoát nhiệt (khoét hậu D15/khe trên),
  tấm ốp tủ lạnh.
- **Tùy chọn**: `appliance_code` · `vent_gap_top` (50) · `support_shelf` (bật) · `side_clearance` (2–5).
- **UI**: Dựng nhanh "Khoang lò 600", "Khoang vi sóng 380", "Tủ lạnh âm". **Core**: `LinkKind::ApplianceBay` (dùng chung D33).
- **Event flow**: mặc định. **Production**: khoét hậu thông gió; thanh đỡ vào danh sách cắt.
- **Nghiệm thu**: tủ kịch trần 600 có khoang lò 600 × 595 → khoang ≥ 560 × 590 × 550; nếu nhỏ hơn thì báo lỗi.

---

**Tổng: 34 đề xuất** (D01–D34). P0: D01, D02, D04, D05, D06, D07, D08, D09, D10, D11, D12 (11 mục). P1: D03, D13,
D14, D15, D16, D17, D18, D19, D20, D21, D22, D24, D25, D27, D29, D30, D34 (17 mục). P2: D23, D26, D28, D31, D32, D33 (6 mục).

---

# 5. Parametric Interaction Gap Analysis

Kiểm tra hạ tầng incremental trong code:
- **Core**: `Command::SetCabinet` thay cả định nghĩa tủ, lệnh nghịch đảo là định nghĩa cũ của **một tủ** (không snapshot
  dự án). `regenerate_inner` (`document.rs:610`) chạy lại `build_cabinet` cho toàn tủ (hàm thuần), rồi so khớp part key
  để giữ `ObjectId`. `ChangeSet` (`document.rs:119–133`) chỉ có `created/deleted/changed/geometry/transform/tree/settings`.
- **UI 3D**: `sceneSync.ts` chỉ tải lại object có trong event, mesh cache theo geometry key. Đã incremental ở mức object.
- **UI 2D**: `Drawing2D.tsx` tính lại toàn bộ `items` của scope mỗi lần `rev` đổi. `useZones.ts:26–40` gọi lại toàn bộ
  `get_zones` mỗi `revision`. **Chưa incremental.**
- **Preview**: 2D kéo vách/kệ/cạnh do client tính và chỉ vẽ nhãn số (`EditLayer.tsx:449–460`). Gizmo 3D gọi `snap` khi kéo.

| Mục | Feature | Existing | Missing | Core work | UI work | Priority |
|---|---|---|---|---|---|---|
| A | Editable Dimension | W/H tủ (2D, 3D), kích thước khoang, cao ngăn kéo, gõ số khi kéo | D (view Bên), offset/lùi kệ, khe cánh, cao chân, tay nắm, thanh treo, tấm lấp; chuỗi dim dãy | `get_zones.dims[]` mô tả dim + request đích (D03) | Vẽ và nhập các dim mới; view Bên sửa được | P1 |
| B | LOCK/AUTO/PERCENT | `Split.bays`, `DrawerSpec.heights`, công thức chia (mới) | Cho cánh nhiều cột (rộng từng cánh), cho mô-đun vách ốp, cho khoang ngăn kéo nhiều cột | Dùng `solve_bays` cho `DoorSpec.col_widths`, `DrawerSpec.col_widths` | Nhãn KHÓA/%/AUTO trên cánh | P1 |
| C | Resize cabinet + anchor | Neo W/H/D; KEEP/PROPORTIONAL/EDGE; tủ liền kề tự dịch | Phụ kiện/tấm ngoài thùng (phào, mặt đá) theo; ràng buộc tối thiểu theo phụ kiện | Run (D09), accessory constraint (D18) | Hiển thị cảnh báo khoang đỏ | P0 (qua D09) |
| D | Drag divider | Kéo vách, preview client, commit `move_split_panel`, snap chia đều, Shift 10 | Preview core; snap mặt tấm tủ khác/cao độ lưu sẵn | Dry-run (D25); danh sách điểm snap trong `get_zones` | Throttle preview; vạch snap | P1 |
| E | Drag shelf | Như D | Snap lỗ 32 | `pin_grid`, `snap: PINS` (D02) | Vạch lỗ 32 | P0 |
| F | Add shelf/divider trong zone | Dựng nhanh, form, `split_zone` công thức (mới), nhân tấm | Thêm kệ nghiêng có hiệu lực; vách lửng | D14 | Mục Dựng nhanh mới | P1 |
| G | Dynamic constraints | `PartMod.anchors` (cạnh → mặt + offset), `set_relation` | Ràng buộc kích thước (≥, ≤), ràng buộc theo phụ kiện, danh sách ràng buộc toàn tủ | `ConstraintTemplate` theo zone/phụ kiện | Tab "Ràng buộc" cấp tủ | P1 |
| H | Offset | `PartMod.offsets` 6 phía, UI Chỉnh tấm | Offset sửa trên 2D; offset theo nhóm (mọi kệ di động) | `get_zones.dims` loại OFFSET | Direct edit | P1 |
| I | Overlay/Inset/Flush/Gap | Nóc/đáy phủ/lọt/giằng; cánh phủ/lọt (nửa phủ tự động ở vách); `set_relation` 2 tấm | Loại bản lề theo phủ/nửa/lọt (khoan, K) | Catalog bản lề (D05) | Chọn loại bản lề | P0 (qua D05) |
| J | Multi-select/edit | `get_properties_multi`, `set_parameter_multi` (1 undo) | Multi-edit cho cánh/ngăn kéo spec (khe, tay nắm) nhiều tủ; áp chuẩn xưởng nhiều tủ | Mở rộng `set_parameter_multi` sang field spec | "Nhiều giá trị" cho các field mới | P1 |
| K | Copy/Array | Duplicate, `array_cabinet {axis, gap}`, `array_split_panel` | UI chỉ axis 0, gap 0; array theo công thức | `widths` (D28) | Hộp thoại | P2 |
| L | Preview/Commit | Commit là 1 Command; preview client (2D), `snap` (gizmo) | Preview tính bởi core cho zone/anchor | `dry_run` (D25) | Throttle, Esc hủy | P1 |
| M | Incremental patch | 3D theo object + geometry key | Event zone/dimension; 2D theo item | `ChangeSet.zones/dims`, `ZonesChanged` (D20) | Map item theo id; `React.memo` | P1 |

---

# 6. Bảng theo phòng

Cột "Dựng được?": **Có** = dựng được bằng khung + Tạo tấm hiện tại · **Có\*** = dựng được nhưng phải chỉnh tay/thiếu
phụ kiện · **Chưa** = không dựng được.

## 6.1 Bếp

| Sản phẩm | Dựng được? | Thiếu gì | Generator cần | Tab thuộc tính | Preset mặc định |
|---|---|---|---|---|---|
| Tủ bếp dưới | Có\* | Chân nhựa, tay nắm vị trí đúng (B4), cam, dán cạnh theo nhóm, báo giá mét dài | Có (Base) + D05, D06, D08 | Chân/treo, Liên kết, Hậu | Tủ bếp dưới 2 cánh 800 |
| Tủ bếp trên | Có\* | Thanh treo/ke treo, tay nắm dưới, cánh lật tay nâng | Wall + D08(HANGING), D17 | Chân/treo | Tủ bếp trên 800 |
| Tủ kịch trần | Có\* | Che trần, 2 tầng (tủ trên chồng tủ dưới) | Base cao + D09 ceiling_filler | Dãy | Tủ kịch trần 600 × 2400 |
| Tủ đồ khô | Có\* | Giá kéo đồ khô (phụ kiện) | D18 | Phụ kiện | Tủ đồ khô 450 kịch trần |
| Tủ lò / lò vi sóng âm | Có\* | Khoang thiết bị, thanh đỡ, thông gió | D34, D15 | Khoang thiết bị | Tủ lò 600 kịch trần |
| Tủ lạnh âm | Chưa | Khoang + ốp cánh tủ lạnh | D34 | Khoang thiết bị | Tủ lạnh âm 600 |
| Tủ góc L / U / chéo / mâm xoay | Chưa | Tủ góc | D07 | Tủ góc | Tủ bếp góc L 1100 |
| Bàn đảo | Chưa | Mở 2 mặt, mặt đá nhô | D23, D09 | Bàn đảo, Mặt đá | Bàn đảo 1800 |
| Quầy bar | Chưa | Như bàn đảo + chân bar | D23 | Bàn đảo | Quầy bar 1500 × 1050 |
| Chậu rửa (khoét mặt đá) | Chưa | Mặt đá + khoét, mặt ngăn giả, khoét hậu ống | D09, D04(false_front), D15 | Mặt đá, Hậu | Tủ chậu 900 |
| Hút mùi (khoang trống) | Có\* | Khoang trống có hậu khoét ống thoát | D15, D34 | Hậu | Tủ hút mùi 900 |

## 6.2 Phòng ngủ

| Sản phẩm | Dựng được? | Thiếu gì | Generator cần | Tab thuộc tính | Preset mặc định |
|---|---|---|---|---|---|
| Tủ áo cánh mở | Có | Hàng lỗ 32, cam, số bản lề theo chuẩn, tay nắm cao 1000 | Wardrobe + D02, D05, D06 | Kệ & chốt tầng, Liên kết | Tủ áo 2 cánh 1000; Tủ áo 4 cánh 1800 |
| Tủ áo cánh lùa | Có\* | Hệ ray, khung nhôm, trừ kích thước | D16 | Cánh lùa | Tủ áo cánh lùa 2000 |
| Tủ áo kịch trần | Có\* | Che trần, tủ trên (2 tầng cánh) | D09, split VirtualH (mới) | Dãy | Tủ áo 3 khoang kịch trần |
| Tủ áo góc L | Chưa | Tủ góc | D07 (dùng chung) | Tủ góc | Tủ áo góc L 1000 × 1000 |
| Giường | Chưa | Toàn bộ | D19 | Nệm, Đầu giường, Thành & dát | Giường 1600×2000 |
| Giường hộc kéo | Chưa | Toàn bộ | D19 + D04 | Hộc kéo | Giường hộc kéo 1600 |
| Tab đầu giường | Có | Chân/treo, tay nắm | Drawer + D08 | Chân/treo | Tab 450 × 450 × 400 2 ngăn |
| Bàn trang điểm | Chưa | Bàn + gương | D21 | Bàn, Gương | Bàn trang điểm 1000 |
| Bàn học | Chưa | Bàn + kệ trên | D21 | Bàn, Kệ trên | Bàn học 1200 |
| Kệ sách | Có | Hàng lỗ 32, kệ lùi, chân | OpenShelf + D02 | Kệ & chốt tầng | Kệ sách 800 × 2000 × 300 |

## 6.3 Phòng khách

| Sản phẩm | Dựng được? | Thiếu gì | Generator cần | Tab thuộc tính | Preset mặc định |
|---|---|---|---|---|---|
| Kệ TV (đặt sàn) | Có\* | Chân inox, push-open, luồn dây | Base thấp + D08, D05, D15 | Chân/treo | Kệ TV 1800 đặt sàn |
| Kệ TV treo | Chưa (thiếu treo) | Ke treo ẩn, lỗ luồn dây | D08(HANGING) | Chân/treo | Kệ TV treo 1800 |
| Vách TV | Chưa | Vách ốp mô-đun, khoét | D22 | Vách ốp | Vách TV 3000 × 2700 |
| Tủ trang trí | Có\* | Cánh kính khung nhôm, LED | D17, D18 | Cánh, Phụ kiện | Tủ trang trí 800 × 2000 |
| Tủ rượu | Có\* | Kệ ô chéo/ô chai | D14 + phụ kiện | Phụ kiện | Tủ rượu 600 |
| Tủ giày | Có\* | Kệ nghiêng thật (B1), chân, ghế ngồi | D14, D08 | Kệ & chốt tầng | Tủ giày 1200 |
| Vách ngăn | Chưa | Vách ốp 2 mặt/lam | D22 | Vách ốp | Vách lam 1200 × 2700 |
| Lam gỗ | Chưa | Lam | D22 | Lam | Lam 40/25 |

## 6.4 WC / ban công / khác

| Sản phẩm | Dựng được? | Thiếu gì | Generator cần | Tab thuộc tính | Preset mặc định |
|---|---|---|---|---|---|
| Tủ lavabo | Có\* | Vật liệu chống ẩm, khoét ống, chân nhựa/treo | D33, D15, D10 | Loại phòng, Hậu | Tủ lavabo treo 800 |
| Tủ gương | Có\* | Cánh gương (vật liệu kính), đèn | D17, D18 | Cánh | Tủ gương 800 × 700 × 150 |
| Tủ máy giặt / sấy | Có\* | Khoang thiết bị | D34 | Khoang thiết bị | Tủ máy giặt 700 × 2000 |
| Tủ thờ | Có\* | Hộc kéo + khoang trang trí + đèn; phào | D13, D18 | Phào, Phụ kiện | Tủ thờ 1270 × 810 (theo thước Lỗ Ban, câu hỏi mở) |
| Bàn làm việc | Chưa | Bàn | D21 | Bàn | Bàn làm việc 1400 |
| Kệ âm tường | Có | Tấm lấp sát tường, không hậu | D09 filler | Dãy | Kệ âm tường 800 |

---

# 7. Preset mặc định (template dựng sẵn)

Chuẩn chung (trừ khi ghi khác): chuẩn xưởng "AIC MDF 17" (ván 17.2, hậu 8.6 rãnh 13 lùi 10), dán cạnh thùng PVC 1 mm cạnh
lộ, cánh ABS 1 mm 4 cạnh, liên kết cam + chốt, bản lề giảm chấn phủ toàn, ray bi. Có thể nạp bằng `insert_template`
(D30). Khoang ghi theo công thức `parse_split_formula`.

| # | Preset | W × H × D (mm) | Cấu trúc | Khoang / chi tiết | Vật liệu mặc định | Rule chính |
|---|---|---|---|---|---|---|
| 1 | **Tủ bếp dưới 2 cánh 800** | 800 × 810 × 560 (+ mặt đá 20–30 → 830–840) | Nóc 2 thanh giằng 100, đáy lọt, hậu rãnh, len chân 100 lùi 50 | 1 kệ di động giữa; cánh đôi phủ | MFC lõi xanh 17 thùng, Acrylic/Laminate cánh, HDF 5–9 hậu | Tay nắm AUTO (trên 60) hoặc Gola; chân nhựa 6; báo giá mét dài |
| 2 | **Tủ bếp trên 800** | 800 × 700 × 350 | Nóc/đáy lọt, hậu rãnh, thanh treo tường | 1 kệ di động; cánh đôi phủ hoặc cánh lật HK | MFC 17 thùng, Acrylic cánh | Cách mặt đá 700; tay nắm dưới 60; đáy dán cạnh trước + dưới lộ |
| 3 | **Tủ bếp góc L** | Cạnh 1100 × 810 × 560 (+ dãy vuông góc) | Góc mù (D07), tấm che mù 150, tấm lấp 50 | 1 kệ; 1 cánh 450 | Như #1 | Báo giá mét dài = w1 + w2 − d; kiểm tra tay nắm không va |
| 4 | **Tủ áo 2 cánh 1000** | 1000 × 2200 × 580 | Nóc phủ, đáy lọt, len chân 80, hậu rãnh | Khoang: 1 vách giữa `50%` → trái: thanh treo cách nóc 60; phải: 4 kệ di động (hàng lỗ 32) | MDF 17 trắng thùng, MDF vân gỗ cánh | Bản lề 4/cánh (cao > 1600); tay nắm cao 1000 từ sàn |
| 5 | **Tủ áo 4 cánh 1800** | 1800 × 2400 × 600 | Nóc phủ, len chân 80 | Chia ngang `/2` (hai khoang 900) → mỗi khoang: `VirtualH 400,*` (tầng trên 400 có 1 kệ) + dưới: khoang treo và khoang 3 ngăn kéo `600,*` | Như #4 | 4 cánh + 2 cánh trên (hoặc cánh liền); ray bi 500 |
| 6 | **Tủ áo 3 khoang kịch trần** | 2400 × 2600 (trần 2700 → che trần 100) × 600 | Tủ dưới 2200 + tủ trên 400 (VirtualH `*,400`) | Chia dọc `800,*,800`: trái treo dài, giữa kệ + 3 ngăn kéo, phải treo 2 tầng | Như #4 | Che trần (D09); cánh trên riêng; bản lề theo bảng |
| 7 | **Tủ áo cánh lùa** | 2000 × 2400 × 620 | Hệ ray 2 đường, sâu thêm 70 cho ray | Chia dọc `/2`: khoang treo + khoang kệ/ngăn kéo trong (sau cánh lùa, D04 inner) | MDF 17 thùng, cánh MDF/kính khung nhôm | Chồng 35; ngăn kéo trong lùi để không va cánh |
| 8 | **Giường 1600 × 2000** | Nệm 1600 × 2000, cao dát 400, đầu giường 1000 | Đầu, 2 vai, đuôi, dát nan 14, đà giữa, 6 chân | — | MDF 17 vân gỗ, dát plywood 12 | Báo giá theo chiếc |
| 9 | **Giường hộc kéo** | Nệm 1600 × 2000, cao dát 450 | Như #8, vai thành hộc | 2 hộc kéo mỗi bên (ray 450) | Như #8 | Không va chân; hộc cao 200 |
| 10 | **Kệ TV treo 1800** | 1800 × 350 × 400 | Nóc/đáy phủ, không len chân, treo ke ẩn | Chia `/3`: 2 ngăn kéo push-open + khoang mở giữa | MDF 17 vân gỗ | Khoét hậu luồn dây Ø60; push-open |
| 11 | **Tủ giày 1200** | 1200 × 1000 × 350 | Nóc phủ, len chân 80 | Chia `/2`, mỗi khoang 4 kệ nghiêng 15° (D14) + ghế ngồi (tùy chọn) | MFC 17 | Cánh đôi; thông thoáng (khoét hậu vent) |
| 12 | **Bàn học 1200** | 1200 × 750 × 600 | Mặt 25, chân tấm trái, hộc tủ phải 400 | Hộc 3 ngăn kéo `/3`; yếm 300; kệ trên 2 tầng (tùy chọn) | MDF 17 + mặt 25 | Khoét luồn dây Ø60 |
| 13 | **Bàn đảo 1800** | 1800 × 900 × 900 (+ mặt đá nhô 300 phía ghế) | Mở 2 mặt (D23), không hậu, vách giữa | Mặt trước 3 ngăn kéo + 1 cánh; mặt sau kệ mở | Như #1 | Mặt đá 1800 × 1200; ốp hông 2 bên |
| 14 | Tủ lavabo treo 800 | 800 × 500 × 480 | Treo, hậu khoét ống | Cánh đôi, mặt ngăn giả trên | MFC lõi xanh / Acrylic | Chống ẩm; dán cạnh 4 cạnh mọi tấm |
| 15 | Tủ lò 600 kịch trần | 600 × 2300 × 580 | Chia `720,600,*` (dưới ngăn kéo, giữa khoang lò, trên cánh lật) | Khoang lò 600, thanh đỡ | Như #1 | Khe thoát nhiệt 50 trên, khoét hậu |

---

# 8. Roadmap

| Đợt | Thời gian | Nội dung | Đầu ra kiểm tra được |
|---|---|---|---|
| **Đợt 1 (P0 nền)** | Tuần 1–2 | D01 Chuẩn xưởng + sửa H1–H24, B2, B4–B8 · D02 Hàng lỗ 32 + snap · D05 Catalog bản lề/tay nắm/push-open · D10 Dán cạnh theo nhóm + vật liệu VN · D24 Tool về core | Tủ áo/tủ bếp dựng từ đầu không cần chỉnh tay; danh sách cắt, lỗ, báo giá khớp nhau; `cargo test` có test hồi quy file cũ |
| **Đợt 1b (P0 sản xuất)** | Tuần 3–4 | D06 Cam/chốt/vít · D04 Ngăn kéo theo ray · D08 Chân/treo · D11 Nhãn + danh sách cắt gộp · D12 Báo giá mét dài/m² + hao hụt | Xuất xưởng được: nhãn, CSV máy cắt, báo giá khách |
| **Đợt 1c (P0 bếp)** | Tuần 5–6 | D09 Dãy tủ (mặt đá, len chân, tấm lấp, che trần) · D07 Tủ góc · D30 (phần preset bếp #1–#3, #15) | Bếp chữ L 3 m + 2 m dựng trong < 10 phút, báo giá mét dài |
| **Đợt 2 (P1)** | Tuần 7–8 | D20 ChangeSet zone + 2D incremental · D25 Preview core · D03 Direct edit mở rộng · D27 Căn/phân bố/xoay · D14 Kệ nghiêng/vách lửng · D15 Khoét hậu · D13 Phào/ốp | DoD §35.13; thao tác nhanh trên 2D |
| **Đợt 2b (P1)** | Tuần 9–10 | D19 Giường · D21 Bàn · D22 Kệ TV/vách ốp · D16 Cánh lùa · D17 Cánh lật · D18 Phụ kiện khoang · D34 Khoang thiết bị · D29 Bản vẽ in · D30 (đủ 15 preset) | Đủ sản phẩm cho căn hộ 2–3 PN |
| **Đợt 3 (P2)** | Tuần 11–12 | D23 Bàn đảo · D26 DXF/MPR/CIX + khoan mặt B/cạnh · D28 Array · D31 Thư viện nhóm · D32 Nối vân · D33 Luật phòng WC/thờ | Kết nối máy khoan ngang; làm việc nhóm |

---

# 9. Mapping vào roadmap / task hiện có

**Repo không có mã `TASK-00..TASK-10`**: đã grep `TASK-` trong `docs/`, `crates/`, `app/src`, không có kết quả. Vì vậy
mapping dùng các lộ trình đang có:
- **PE-Phase 1–6**: lộ trình Parametric Editor, `docs/TONG-HOP-PARAMETRIC-EDITOR.md` §3.
- **DoD-n**: Definition of Done, cùng tài liệu §4.
- **ST-Lx**: hạn chế đã biết `docs/STATUS.md`, mục "Chưa làm / hạn chế" 1–9.
- **ST-Chưa làm**: dòng "Chưa làm: tool 05, 07, 15; chân đế dạng thanh; căn/phân bố".
- **Core Phase 7**: tối ưu, hiệu năng.

Đề xuất không map được thì đặt mã **MỚI-xx**. Không đổi lộ trình cũ.

| Đề xuất | Map vào | Overlap / ghi chú |
|---|---|---|
| D01 Chuẩn xưởng | PE-Phase 5 (Rule preset, spec §26) | Mở rộng `RulePreset`; migrate preset cũ thành chuẩn. **MỚI-01** cho phần catalog tham số sản xuất |
| D02 Hàng lỗ 32 | PE-Phase 2 (kéo kệ, spec §10 "snap theo cao độ lưu sẵn") + PE-Phase 6 (gia công) | Overlap với snap kéo kệ hiện có (`EditLayer.tsx:457`) |
| D03 Direct edit 2D | PE-Phase 1 (dimension sửa trực tiếp, DoD-1) | Bổ sung các dim chưa có |
| D04 Ngăn kéo theo ray | PE-Phase 4 (spec §14 "loại/chiều dài ray, loại đáy") | Overlap `DrawerSpec` |
| D05 Catalog phụ kiện | PE-Phase 4 (spec §13 cánh) + PE-Phase 6 | **MỚI-02** cho catalog |
| D06 Cam/chốt | PE-Phase 6 (gia công) + ST-L7 | Sửa B2 (costing) |
| D07 Tủ góc | **MỚI-03** | — |
| D08 Chân/treo | ST-Chưa làm "chân đế dạng thanh" | Map trực tiếp |
| D09 Dãy tủ | **MỚI-04** | Dùng dịch tủ liền kề đã có (`resize_cabinet`) |
| D10 Dán cạnh nhóm + vật liệu | PE-Phase 6 (dán cạnh) + ST-L3 (catalog SQLite) | Làm trên JSON trước, SQLite sau |
| D11 Nhãn + cut list gộp | PE-Phase 6 (xuất sản xuất) | Mở rộng `cut_rows` |
| D12 Báo giá | PE-Phase 6 (báo giá) | **MỚI-05** cho mét dài/m² |
| D13 Phào/ốp | **MỚI-06** | Dùng `StructureRules` |
| D14 Kệ nghiêng/vách lửng | PE-Phase 2 (spec §11–12) | Sửa B1 |
| D15 Khoét hậu | PE-Phase 5 (contour, spec §22) | Dùng `ContourFeature inner` |
| D16 Cánh lùa | PE-Phase 4 (spec §13 Sliding) | — |
| D17 Cánh lật/gập | PE-Phase 4 | — |
| D18 Phụ kiện khoang | **MỚI-02** (chung catalog) | — |
| D19 Giường · D21 Bàn · D22 Vách/TV · D23 Bàn đảo | **MỚI-07** (Product framework) | — |
| D20 ChangeSet + 2D incremental | PE spec §29–30, DoD-13, Core Phase 7 | Overlap với `aic_object_relation` (ChangeSet chung) |
| D24 Tool về core | ST-Chưa làm "tool 05, 07, 15" (cùng khu vực) | Sửa vi phạm CLAUDE.md (B3) |
| D25 Preview core | PE spec §31 (Preview/Commit) | — |
| D26 Xuất máy | ST-L7 + PE-Phase 6 | — |
| D27 Căn/phân bố/xoay | ST-Chưa làm "căn/phân bố nhiều đối tượng" + ST-L4 (snap) | Map trực tiếp |
| D28 Array | PE-Phase 4 (spec §21) | Core đã có `axis`/`gap` |
| D29 Bản vẽ in | **MỚI-08** | Dựa vào view 2D hiện có |
| D30 Template dựng sẵn | PE-Phase 5 (spec §24–25) | Cần D07/D19… cho một số preset |
| D31 Thư viện nhóm | **MỚI-09** | `library.rs` |
| D32 Nối vân | PE-Phase 6 + `aic-nesting` | — |
| D33 Luật phòng · D34 Khoang thiết bị | **MỚI-10** | — |

---

# 10. Câu hỏi còn mở

1. **Chuẩn ván**: xưởng dùng chủ yếu MDF An Cường 17 (17.2 thực) hay MFC 18? Có cần "dày danh nghĩa" (in nhãn 17) khác
   "dày thực" (tính kích thước 17.2) không?
2. **Liên kết**: cam + chốt (minifix 15 mm) hay vít + chốt? Hãng và mã cam cụ thể (khoảng cách tâm cam 24/34)?
3. **Máy**: máy cắt (bàn trượt / CNC nesting), máy khoan (có khoan ngang không)? Phần mềm CAM đang dùng (định dạng
   MPR/CIX/DXF nào)?
4. **Bản lề/ray mặc định**: Hafele, Blum, Ivan hay Garis? Giá nhập để làm bảng giá mẫu?
5. **Báo giá**: bếp theo mét dài có gồm mặt đá và phụ kiện không? Tủ áo theo m² mặt đứng tính cả phần kịch trần/che trần không?
6. **Hàng lỗ 32**: khoan cả hàng (thẩm mỹ, linh hoạt) hay chỉ khoan tại vị trí kệ (nhanh, ít lỗ)? Mặc định nào?
7. **Tủ thờ**: có áp kích thước theo thước Lỗ Ban (gợi ý kích thước "đẹp") không?
8. **Đo hiện trường**: có cần nhập mặt bằng tường thật (tường lệch, không vuông) để tự tính tấm lấp không, hay chỉ nhập
   khoảng trống W?
9. **Nhãn**: khổ giấy nhãn và máy in nhãn (Brother/Zebra) đang dùng? Nhãn có cần mã vạch cho máy khoan đọc không?
10. **Thư viện nhóm**: lưu trên thư mục mạng nội bộ là đủ, hay cần tài khoản/đám mây (và ai được sửa chuẩn xưởng)?
