# Prompt: AI agent phân tích và đề xuất tính năng / tùy chọn cho AIC CAD

> Dán toàn bộ phần dưới vào Claude Code (hoặc AI agent khác) đang mở repo AIC CAD.
> Có thể kèm ảnh chụp plugin tham chiếu (bảng Thuộc tính, Tạo tấm, Tool…) để agent so sánh.
> Kết quả là **một báo cáo đề xuất**. Agent **không sửa code**, trừ khi bạn yêu cầu thêm.

---

## Vai trò

Bạn đóng vai **chuyên gia sản phẩm phần mềm thiết kế nội thất gỗ công nghiệp** (tủ bếp, tủ áo, kệ, giường, bàn…),
đồng thời là **kỹ sư CAD/CAM** hiểu quy trình xưởng: cắt ván, dán cạnh, khoan, CNC, lắp ráp.
Nhiệm vụ: **phân tích AIC CAD hiện có**, rồi **đề xuất các tính năng và tùy chọn (option) còn thiếu**, để người
thiết kế dựng được sản phẩm thật **mà không phải chỉnh tay**.

## Bối cảnh sản phẩm (đọc trước)

1. Đọc theo thứ tự: `CLAUDE.md`, `README.md`, `docs/STATUS.md`, `docs/HUONG-DAN-DUNG-TU.md`,
   `docs/api-mapping.md`, `docs/TONG-HOP-PARAMETRIC-EDITOR.md`.
2. Kiến trúc: core Rust (`crates/`) là nguồn dữ liệu gốc; UI React (`app/`) chỉ gửi request và vẽ.
   Mô hình tủ gồm các file sau:
   - `crates/aic-domain/src/zone.rs`: vùng/khoang KHÓA/%/AUTO, cánh, ngăn kéo.
   - `crates/aic-domain/src/layout.rs`: sinh tấm, part mods, ràng buộc.
   - `crates/aic-domain/src/structure.rs`: hậu, thanh giằng, len chân.
   - `crates/aic-domain/src/cabinet.rs`: preset tủ.
3. Danh sách request có trong `crates/aic-api/src/protocol.rs`. Bảng Thuộc tính kết cấu nằm ở
   `crates/aic-api/src/structure_api.rs`. Các tool nằm ở `app/src/features/tools/ToolColumn.tsx`.
4. Nếu chạy được thì chạy thử để kiểm chứng thay vì đoán:
   - `cargo run -p aic-dev-server --release`
   - `cd app && npm run dev`
   - Tạo vài tủ, thử Tạo tấm, Chỉnh tấm, bảng Thuộc tính, Báo cáo.

## Bối cảnh nội thất Việt Nam (bắt buộc bám theo)

Phần mềm phục vụ **xưởng và công ty thiết kế – thi công nội thất gỗ công nghiệp tại Việt Nam**. Đề xuất phải khớp
với cách người Việt đặt làm nội thất trọn gói theo **từng phòng của căn hộ / nhà phố / biệt thự**.

**Sản phẩm theo phòng** (mỗi phòng nêu tủ/đồ nào hay làm, tùy chọn gì hay bị hỏi):

| Phòng | Sản phẩm thường làm |
|---|---|
| Bếp | Tủ bếp dưới, tủ bếp trên, tủ kịch trần, tủ đồ khô, tủ lò / lò vi sóng âm, tủ lạnh âm, tủ góc (L, U, góc chéo, mâm xoay), bàn đảo, quầy bar, chậu rửa / bếp âm (khoét mặt đá), hút mùi (khoang trống) |
| Phòng ngủ (PN1–PN4) | Tủ quần áo (cánh mở / cánh lùa / kịch trần / góc L / có tủ trên), giường (hộc kéo, đầu giường bọc / ốp gỗ, giường tầng trẻ em), tab đầu giường, bàn trang điểm, bàn học, kệ sách, vách đầu giường |
| Phòng khách | Kệ tivi (treo / đặt sàn), vách ốp tivi, tủ trang trí / tủ rượu, tủ giày + ghế ngồi, vách ngăn / lam gỗ, trần gỗ |
| WC / ban công / khác | Tủ lavabo (chống ẩm), tủ gương, tủ giặt – máy sấy, tủ thờ, bàn làm việc, kệ âm tường |

**Kích thước hay dùng** (làm mặc định / gợi ý, cho sửa được):
- Tủ bếp dưới cao 810–850 kể cả mặt đá, sâu 560–600, chân 80–100.
- Tủ bếp trên cao 700–900 hoặc kịch trần, sâu 320–350, cách mặt đá 650–750.
- Tủ áo cao 2000–2700 (thường sát trần), sâu 550–600, thanh treo cách nóc 50–60.
- Giường 1200 / 1400 / 1600 / 1800 × 2000, mặt dát cao 350–450.
- Kệ tivi sâu 350–450.
- Bước lỗ chốt tầng 32 mm.

**Vật liệu và phụ kiện phổ biến** (danh mục, đơn giá và độ dày phải phản ánh đúng):
- MDF / MFC chống ẩm lõi xanh.
- Bề mặt: Melamine, Laminate, Acrylic bóng gương, veneer, sơn bệt.
- Ván dày 17–18 mm; hậu 5–9 mm.
- Chỉ dán cạnh PVC / ABS 0,5–2 mm.
- Phụ kiện kiểu Hafele / Blum / Ivan / Garis: bản lề giảm chấn, ray âm, tay nắm âm (profile nhôm), push-open, giá bát đĩa, rổ gia vị.

**Cách xưởng làm việc:**
- Đo hiện trường theo tường và trần, có khe bù (tấm lấp) sát tường / sát trần.
- Báo giá **tủ bếp theo mét dài**, tủ áo theo m² mặt đứng, hoặc bóc chi tiết theo m² ván + phụ kiện.
- Xuất danh sách cắt và dán nhãn tấm cho máy cắt / CNC nesting.
- Lắp đặt tại công trình theo từng phòng.

Với mỗi loại sản phẩm ở trên, cho biết:
- phần mềm đã dựng được chưa;
- nếu chưa, cần **generator** (bộ sinh chi tiết) nào;
- cần **tab Thuộc tính** nào;
- cần **mẫu mặc định** nào (ví dụ "Tủ áo 3 cánh 1800 kịch trần", "Tủ bếp chữ L 3 m + 2 m", "Giường 1m6 hộc kéo").

## Phạm vi phân tích

Phân tích theo **từng nhóm** dưới đây. Với mỗi nhóm, liệt kê: **đã có**, **có nhưng thiếu tùy chọn**, và **chưa có**.

| # | Nhóm | Gợi ý nội dung cần soi |
|---|---|---|
| 1 | Khung tủ | Kiểu nóc/đáy (phủ, lọt, giằng), hồi (lửng, âm, vát), tủ góc L, tủ góc chéo, tủ cong, tủ mở 2 mặt, tủ treo / tủ đứng / tủ chân, chân tăng chỉnh, len chân 3 mặt |
| 2 | Kết cấu phụ | Thanh giằng trên / dưới / sau, thanh chặn cánh, nẹp, phào nóc, phào chân, ốp hông, tấm che hở trần, tấm lấp (filler) |
| 3 | Hậu | Rãnh, lọt, ốp, chia tấm, hậu phụ, hậu đôi, khoét hậu (ổ điện, ống nước) |
| 4 | Khoang & tấm chia | KHÓA/%/AUTO, chia theo modul, kệ lùi / kệ nghiêng, kệ cố định vs di động (chốt tầng, bước lỗ 32 mm), vách lửng, kệ góc |
| 5 | Cánh | Phủ / lọt / nửa phủ, cánh lật lên / xuống, cánh gập, cánh kính / khung nhôm, khe từng phía, số bản lề theo chiều cao, loại bản lề (thẳng / cong / bật), tay nắm (vị trí, âm, không tay nắm – push open), chiều mở |
| 6 | Ngăn kéo | Hộc gỗ / hộc kim loại (tandem, box), ray bi / ray âm giảm chấn, chiều dài ray theo sâu, ngăn kéo trong (sau cánh), mặt ngăn giả, chia hộc |
| 7 | Phụ kiện | Thanh treo, giá giày, giá kéo, rổ bếp, đèn LED (rãnh, nguồn), chân tủ, ke góc, cam/chốt/vít (số lượng theo luật) |
| 8 | Vật liệu & dán cạnh | Bộ vật liệu theo tủ / theo nhóm tấm, vân (hướng, nối vân nhiều cánh), dán cạnh theo luật + ngoại lệ, độ dày chỉ ảnh hưởng kích thước cắt |
| 9 | Gia công & sản xuất | Khoan liên kết (cam, chốt, minifix), lỗ bản lề, rãnh, pocket, contour; nhãn tấm, danh sách cắt, nesting, G-code, xuất DXF / MPR / CSV cho máy |
| 10 | Báo giá | Đơn giá theo vật liệu / phụ kiện / công, hệ số hao hụt, báo giá theo phòng / tầng / dự án, xuất PDF / Excel |
| 11 | Thao tác (UX) | Chọn, kéo, nhập số khi kéo, bắt điểm, căn chỉnh / phân bố nhiều tủ, xoay 90°, nhân dãy, dãy tủ bếp (mặt đá, chỉ chân), chuột phải, phím tắt, undo |
| 12 | Mẫu & thư viện | Template tủ, mẫu vùng, mẫu tab thuộc tính, rule preset, chia sẻ thư viện (đồng bộ đám mây, nhóm) |
| 13 | Loại nội thất khác (theo phòng ở trên) | **Giường** (đầu giường, vai, dát, chân, hộc kéo, giường tầng), **bàn** (học, làm việc, trang điểm, ăn), **kệ tivi**, **tủ giày**, **vách ốp tường / lam**, **bàn đảo bếp** — mỗi loại cần những tham số và tab thuộc tính nào |
| 14 | Hiển thị & bản vẽ | Mặt đứng / mặt cắt / chi tiết tấm, ghi kích thước tự động, xuất bản vẽ in (khổ A3, khung tên), phối cảnh vật liệu |

## Cách làm

1. **Kiểm kê hiện trạng.** Với mỗi nhóm, tìm trong code và tài liệu xem có gì (tên request, tên tham số, tab, tool).
   Ghi đường dẫn file cho từng điểm. Không kết luận "chưa có" nếu chưa tìm trong code.
2. **So sánh với thực tế xưởng và plugin tham chiếu** (nếu có ảnh). Nêu rõ tùy chọn nào người thợ / người thiết kế
   hay cần, và hiện đang phải chỉnh tay ở đâu.
3. **Đề xuất.** Mỗi đề xuất phải có:
   - **Tên + mô tả ngắn** (tiếng Việt, đúng thuật ngữ xưởng).
   - **Tùy chọn / tham số cụ thể**: tên, kiểu (số / chọn / bật-tắt), đơn vị, giá trị mặc định, khoảng hợp lệ.
   - **Chỗ đặt trong UI**: tab Khung / Tạo tấm / Chỉnh tấm / bảng Thuộc tính (tab nào) / Tool / chuột phải / 2D.
   - **Ảnh hưởng tới core**: model nào đổi (zone, `PartMod`, `StructureRules`, request mới…). Giữ nguyên tắc:
     kích thước là tham số, không scale; mọi thay đổi là Command có undo; UI không tính toán CAD.
   - **Ảnh hưởng sản xuất**: danh sách cắt, dán cạnh, khoan, báo giá.
   - **Ưu tiên**: P0 (chặn dựng sản phẩm thật) / P1 (hay dùng) / P2 (nâng cao).
   - **Độ khó**: S / M / L, kèm lý do ngắn.
   - **Tiêu chí nghiệm thu**: 1–3 kịch bản kiểm tra được, ví dụ "tủ 1600 → 1800, phào nóc tự dài theo, danh sách cắt
     có phào 1800 + 2×hông".
4. **Gom thành lộ trình**: đợt 1 (P0), đợt 2, đợt 3. Mỗi đợt dài khoảng 1–2 tuần làm việc.
5. **Bảng theo phòng**: với mỗi phòng (Bếp, Phòng ngủ, Phòng khách, WC/khác), liệt kê sản phẩm, trạng thái trong
   phần mềm (dựng được / thiếu tùy chọn / chưa có) và các đề xuất liên quan.

## Định dạng kết quả

Viết file `docs/DE-XUAT-TINH-NANG.md` gồm:

1. **Tóm tắt** (≤ 10 dòng): 5 khoảng trống lớn nhất và vì sao quan trọng.
2. **Bảng hiện trạng** theo 14 nhóm, mỗi nhóm có 3 cột: Đã có · Thiếu tùy chọn · Chưa có, kèm đường dẫn file.
3. **Danh sách đề xuất**, theo mẫu:

   ```
   ### [P0][M] Phào nóc
   - Mô tả: …
   - Tùy chọn: Bật (bật/tắt, mặc định tắt) · Cao (mm, 60, 20–200) · Nhô trước (mm, 20) · Chạy 3 mặt / mặt trước · Vật liệu
   - UI: Bảng Thuộc tính → tab "Phào" · lưu mẫu tab
   - Core: StructureRules.cornice; sinh tấm "c:cornice_*" trong layout::carcass; dán cạnh mặt trước
   - Sản xuất: 1–3 thanh, cắt vát 45° ở góc (contour), báo giá theo m dài
   - Nghiệm thu: …
   ```

4. **Bảng theo phòng** (Bếp · Phòng ngủ · Phòng khách · WC/khác): sản phẩm → trạng thái → đề xuất liên quan.
5. **Bộ mẫu mặc định** nên có sẵn (tên, kích thước, cấu trúc), ví dụ "Tủ bếp dưới 2 cánh 800", "Tủ áo 4 cánh
   kịch trần 2400", "Giường 1m8 hộc kéo 2 bên", "Kệ tivi treo 1800".
6. **Lộ trình** theo đợt.
7. **Câu hỏi còn mở** cho chủ sản phẩm (tối đa 10), ví dụ tiêu chuẩn xưởng hay máy CNC đang dùng.

## Quy tắc

- Viết **tiếng Việt**, thuật ngữ xưởng (hồi, nóc, đáy, hậu, giằng, len chân, phào, nẹp, cánh phủ / lọt, ray bi,
  chốt tầng, cam, minifix…).
- Mỗi đề xuất phải **cụ thể đến mức lập trình viên làm được ngay**. Không viết chung chung kiểu "cải thiện UX".
- Không đề xuất điều trái với kiến trúc trong `CLAUDE.md`.
- Nếu thấy lỗi hoặc điểm không nhất quán trong hiện trạng (ví dụ một giá trị cứng không sửa được), ghi vào mục
  **Lỗi / điểm cứng phát hiện**, kèm file và dòng.
- Nếu có thể chạy app, chụp màn hình minh họa chỗ còn thiếu và lưu vào `docs/screenshots/de-xuat-*.png`.

## Chia việc cho nhiều agent (tùy chọn)

Nếu dùng được nhiều agent song song, chia theo nhóm. Mỗi agent làm phần của mình theo đúng *Cách làm* ở trên, rồi một
agent tổng hợp gộp lại, bỏ trùng và xếp lộ trình.

| Agent | Nhóm |
|---|---|
| A | 1–4 (khung, kết cấu phụ, hậu, khoang) |
| B | 5–7 (cánh, ngăn kéo, phụ kiện) |
| C | 8–10 (vật liệu, gia công, báo giá) |
| D | 11–12, 14 (thao tác, thư viện, bản vẽ) |
| E | 13 (giường, bàn, kệ tivi… mỗi loại một bảng tham số + tab thuộc tính) |
