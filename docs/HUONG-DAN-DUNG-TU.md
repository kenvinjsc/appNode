# Hướng dẫn dựng tủ từng bước

Tài liệu này đi theo đúng thứ tự thao tác khi dựng một dự án nội thất gỗ:
**Dự án → Tầng → Phòng → Tủ (Khung) → Tạo tấm → Chỉnh tấm → Tool → Báo cáo → Sản xuất**.

Chạy chương trình:

```bash
cargo run -p aic-dev-server --release   # core, cổng 8787
cd app && npm run dev                   # giao diện, http://localhost:5173
```

Bố cục màn hình:
- **Trái**: *Cây đối tượng* (tầng, phòng, tủ, tấm) và tab *Tool*.
- **Giữa**: 3D phối cảnh và 2D bản vẽ.
- **Phải**: bảng thiết kế với các tab *Khung · Tạo tấm · Chỉnh tấm · Quản lý · Thư viện · Cài đặt*.

---

## 1. Chia dự án theo tầng và phòng

![Tầng và phòng](screenshots/01-tang-phong.png)

Một dự án gồm nhiều **tầng**, mỗi tầng có nhiều **phòng**, mỗi phòng có nhiều **tủ**.
Hai hàng tab ở đầu *Cây đối tượng* dùng để chọn tầng và phòng.

| Thao tác | Kết quả |
|---|---|
| **+ Chia tầng** (lần đầu) | Tạo "Tầng 1" và bật hàng tab tầng. |
| **+ Tầng** | Thêm tầng mới (Tầng 2, Tầng 3, Tầng 4…). Tầng mới có khu đặt tủ riêng, cách các tầng khác 3 m trên mặt bằng. |
| Bấm một tab tầng | Chỉ hiện tủ của tầng đó (cây, 3D, 2D) và camera tự căn khung nhìn. Hàng dưới chỉ liệt kê phòng của tầng này. |
| **+ Phòng** | Thêm phòng vào tầng đang chọn, ví dụ Bếp, Khách, PN1, WC1. |
| Bấm một tab phòng | Chỉ hiện tủ của phòng đó. Tủ tạo mới sẽ vào đúng phòng này. |
| Bấm đúp một tab | Đổi tên tầng/phòng: mọi tủ bên trong được chuyển theo, Ctrl+Z hoàn tác được. |
| **Mọi tầng / Tất cả / Cả tầng** | Bỏ lọc. |

Số nhỏ cạnh tên tab là số tủ đang có. Có thể chuyển một tủ sang tầng hoặc phòng khác bằng cách sửa ô **Tầng**
hoặc **Tên phòng** ở tab *Chỉnh tấm* khi đang chọn tủ đó.

## 2. Tạo tủ (tab Khung)

![Tạo tủ](screenshots/02-tao-tu-khung.png)

1. Chọn tầng và phòng ở cây bên trái. Chip phòng trong form cũng đổi được phòng.
2. Tab **Khung → Thông tin tủ**:
   - *Tên tủ*: để trống thì tự đánh số theo kiểu khung (BếpDưới01, TủQA01…).
   - *Kích thước*: Rộng, Cao, Sâu.
   - *Dạng tủ*.
   - *Kiểu khung*: 01 BếpDưới … 06.
3. **Luật liên kết**:
   - Nóc: phủ bì, lọt lòng hoặc giằng.
   - Đáy.
   - Độ sâu rãnh hậu: 0 là hậu lọt, 13 là hậu âm rãnh.
4. **Luật dán cạnh**:
   - Dán hở bỏ khuất, Dán toàn bộ hoặc Không dán.
   - Loại chỉ: mặc định Đơn 1mm.
   - Độ dày không dán: mặc định bỏ 8.6.
   - Ngưỡng dán cạnh và bỏ cạnh ngắn.
5. Bấm **[TAB] Tạo tủ** (hoặc phím TAB).
   - Nếu đang chọn một tủ cùng phòng, tủ mới được đặt **nối tiếp bên phải** tủ đó, dùng để tạo dãy tủ bếp.
   - Nếu không, tủ nối tiếp dãy của phòng. Phòng trống thì tủ được đặt ở một khu mới.

Thông số mặc định: ván 17.2, hậu 8.6, rãnh 13, chỉ Đơn 1mm.

## 3. Dựng chi tiết bên trong (tab Tạo tấm)

### 3.1 Ghim vùng
![Ghim vùng](screenshots/03-ghim-vung.png)

1. Bấm vào tủ để chọn tủ.
2. Mở tab **Tạo tấm**. Các **vùng** (khoảng trống giữa các tấm) hiện ra, kèm kích thước lọt lòng.
3. Ghim vùng:
   - **Click một vùng** để ghim vùng đó.
   - **Ctrl+click** để ghim thêm vùng khác, áp một thao tác cho nhiều vùng cùng lúc.
   - Phím **~** bật/tắt hiện vùng, **Alt+~** cô lập tủ.

### 3.2 Dựng nhanh bằng chuột phải
![Dựng nhanh](screenshots/04-dung-nhanh.png)

**Chuột phải vào một vùng** để mở menu *Dựng nhanh*. Chọn một mục là dựng xong ngay, không phải điền form:

| Nhóm | Lựa chọn |
|---|---|
| Tấm ngang | Kệ di động ×1…×5 (chia đều), Kệ cố định giữa |
| Tấm đứng / hậu | Hông giữa (2 khoang), Chia 3 khoang, Hậu phụ |
| Cánh | Cánh đơn lề trái/phải, Cánh đôi phủ bì/lọt lòng, Cánh lật (lề trên), Cửa lùa 2 cánh |
| Ngăn kéo | ×1…×4 phủ bì, ×2 lọt lòng (tự có hộc + ray bi theo chiều sâu) |
| Liên kết | Thanh treo oval |

Mục cuối, *Tùy chọn chi tiết…*, mở form đầy đủ ở mục 3.3.

### 3.3 Dựng bằng form
Sáu nút tròn ở đầu tab lần lượt là: Tấm ngang, Tấm đứng, Hậu, Cánh, Ngăn kéo, Liên kết.

- **Tấm ngang / đứng / hậu**:
  - Số lượng (1–9).
  - Độ dày.
  - Loại: kệ di động (tự khoan chốt tầng) hoặc kệ cố định.
  - **Vị trí** với chấm đỏ = tham số bị khóa:
    - *Tỷ lệ (%)*: giữ tỷ lệ khi tủ đổi kích thước.
    - *Cách dưới* hoặc *Cách trên*: giữ khoảng cách cố định.
  - Nghiêng.
  - Khung kệ.
- **Cánh**:
  - Số cột × hàng.
  - Đơn, Đôi hoặc Lùa.
  - Phủ bì hoặc lọt lòng.
  - Lắp lề.
  - Thanh chặn: chữ L hoặc thẳng, với cao, che lên, chân, lùi.
- **Ngăn kéo**:
  - Số tầng, số cột, phủ bì hoặc lọt lòng.
  - Hộc: khe hở, độ dày thành, đáy.
  - Khoảng hở ray.
- **Liên kết**: thanh oval, cách trên. Danh sách liên kết đã áp có nút × để xóa.

Bấm **[TAB] Thêm** để áp vào các vùng đang ghim.

## 4. Chỉnh chi tiết (tab Chỉnh tấm)
![Chỉnh tấm](screenshots/05-chinh-tam.png)

Click một tấm, rồi mở tab **Chỉnh tấm**:
- **Vị trí**: bấm tên dòng (Tỷ lệ / Cách dưới / Cách trên) để đổi cách khóa. Tấm giữ nguyên vị trí; tham số có chấm đỏ được giữ khi vùng đổi kích thước.
- **Co giãn** 4 cạnh (mm): kéo dài hoặc thu ngắn tấm.
- Độ dày, vật liệu, tên, dán cạnh từng cạnh.
- Cánh / ngăn kéo: lề, phủ/lọt, khe giữa cánh, **khe từng phía** (trái/phải/dưới/trên), thanh chặn.

- **Offset (lùi mặt)**: lùi mặt trước/sau/trái/phải/trên/dưới theo hướng tủ (ví dụ kệ lùi trước 30). Lưu thành tham số, tủ đổi kích thước vẫn giữ.
- **Ràng buộc (bám mặt tấm khác)**: chọn cạnh (trái/phải/dưới/trên) → tấm đích → mặt trong/ngoài → offset → *Thêm*.
  Ví dụ: kệ *Cạnh phải → Hồi phải · mặt trong · 0*: hồi phải dịch vào (Lùi phải 30) thì kệ tự ngắn lại 30. Mỗi cạnh một
  ràng buộc; nút × để bỏ.
- **Neo rộng / cao / sâu** (khi chọn tủ): giữ trái/giữa/phải… khi đổi kích thước.

**Chọn nhiều tấm** (Ctrl+click trên cây, 3D hoặc 2D): tab Chỉnh tấm hiện các thông số chung; ô có giá trị khác nhau hiện
"Nhiều giá trị". Sửa một lần áp cho tất cả, Ctrl+Z hoàn tác cả nhóm. Bảng bên phải tự chuyển tab: chọn tấm → Chỉnh tấm,
ghim vùng → Tạo tấm.

**Chuột phải vào một tấm** để thao tác nhanh:
- Đổi kệ di động ↔ cố định.
- Căn giữa vùng (50%).
- Chia đều lại.
- Co giãn trên ±20.
- Mở Chỉnh tấm.

## 4b. Sửa trực tiếp trên bản vẽ 2D
![2D editor](screenshots/07-2d-editor.png)

Chọn một tủ. Ở **Mặt đứng (Trước)**, bản vẽ 2D trở thành trình chỉnh sửa:

| Thao tác | Kết quả |
|---|---|
| Bấm số **W / H** màu cam | Nhập kích thước tủ mới (Enter). Tủ giữ vị trí theo **Neo** (Chỉnh tấm → Neo rộng: Giữ trái / giữa / phải). Các tủ **đứng liền trong cùng dãy** (cùng cao độ, cùng hướng) tự dịch theo để dãy luôn khít; tủ trên, tủ đặt cách khe không bị ảnh hưởng. |
| Bấm số kích thước khoang | Nhập mm → khoang thành **KHÓA**; nhập `40%` → khoang thành **%**. |
| Bấm nhãn **KHÓA / % / AUTO** trên số | Đổi chế độ khoang: **KHÓA** giữ mm · **%** giữ tỉ lệ · **AUTO** chia phần còn lại. Ví dụ `600 KHÓA · AUTO · 400 KHÓA`: tủ 1600 → 1800 thì chỉ khoang giữa tăng 200. |
| Kéo vách / kệ | Xem trước số đo 2 khoang kề khi kéo; thả chuột mới lưu. Tự bắt điểm chia đều; giữ **Shift** để bước 10 mm. Chỉ 2 khoang kề thay đổi. |
| Click vùng trống | Ghim vùng (Ctrl+click ghim thêm). **Chuột phải** → Dựng nhanh, Chia ngang/dọc 2–4 khoang, Chia đều lại. |

| Số cao ngăn kéo (bên phải chồng ngăn) | Nhập mm / `%`, bấm nhãn để đổi KHÓA / % / AUTO; kéo khe giữa 2 ngăn để chia lại. |

Màu số: xanh dương = AUTO, đỏ = KHÓA, xanh lá = %. Nếu thay đổi làm một khoang nhỏ hơn 1 mm, phần mềm từ chối và giữ nguyên.

## 4c. Template, rule preset, lật gương, nhân dãy
![Template](screenshots/08-template.png)

- **Lưu template**: chuột phải tủ → *Lưu làm template…* (hoặc Cài đặt → Template tủ). Template lưu cấu trúc logic:
  khoang KHÓA / % / AUTO, cánh, ngăn kéo, luật liên kết, dán cạnh, vật liệu.
- **Chèn template**: tab Khung → *Template & Rule preset* → chọn template, nhập W/H/D → **[TAB] Tạo tủ**. Phần mềm tính
  lại: khoang KHÓA giữ mm, khoang AUTO lấy phần còn lại. Nếu không đủ chỗ thì báo và không tạo.
- **Rule preset**: "AIC Wardrobe Standard" (17.2 / hậu 8.6 / rãnh 13 / khe cánh 2 / lùi kệ 30), "AIC Bếp dưới", hoặc
  *Lưu từ tủ*. Áp khi tạo tủ (tab Khung) hoặc cho tủ đang chọn (Cài đặt).
- **Chuột phải tủ**: Lật gương trái ↔ phải (khoang, bản lề), Nhân dãy tủ sang phải, Sửa kích thước, Báo cáo.
- **Chuột phải kệ / vách**: Nhân tấm (thêm n tấm giống, chia đều), Chia đều lại.

## 4d. Quan hệ 2 tấm, kéo cạnh, mặt cắt
![Kéo cạnh](screenshots/09-keo-canh.png)

**Quan hệ 2 tấm**: chọn tấm A, Ctrl+click tấm B → chuột phải (hoặc Tool 23):

| Lựa chọn | Kết quả |
|---|---|
| A phủ B | A chạy tới mặt ngoài B, B dừng ở A (ví dụ Đáy phủ Hồi: hồi đứng trên đáy). |
| A lọt B | A dừng ở mặt trong B, B chạy qua A. |
| Bằng mặt trước | Cạnh trước A bằng mặt cạnh trước B (ví dụ kệ không lùi). |
| Khe… | Như lọt, chừa khe g mm. |
| Bỏ quan hệ | Xóa ràng buộc giữa A và B. |

Quan hệ lưu thành ràng buộc nên đổi kích thước tủ vẫn giữ.

**Kéo 4 cạnh tấm**: chọn một tấm của tủ, ở Mặt đứng xuất hiện 4 ô vuông cam ở giữa các cạnh. Kéo để xem trước
kích thước mới (Shift: bước 10 mm), thả chuột để lưu. Nút trên thanh bản vẽ đổi chế độ:
- **Giữ ràng buộc**: cạnh đang bám mặt tấm khác chỉ đổi khe tới mặt đó.
- **Tự do**: bỏ ràng buộc của cạnh đó và đổi offset của tấm.

![Mặt cắt](screenshots/10-mat-cat.png)

**View**: Mặt đứng (Trước), Mặt bên (Trái), Mặt bên (Phải), Mặt bằng (Trên), **Mặt cắt dọc (theo X)** và **Mặt cắt
ngang (theo cao)**. Với mặt cắt, kéo thanh trượt để chọn vị trí cắt (số mm hiện bên cạnh); tấm bị cắt tô gạch chéo,
phần phía trước mặt cắt được bỏ đi.

## 5. Tool (tab Tool bên trái)
Chọn tấm rồi chọn tool. Các tool đã dùng được:

| Tool | Cách dùng |
|---|---|
| 01 Xóa | Xóa tấm đang chọn (hoàn tác được). |
| 02 Ẩn/Hiện | Ẩn hoặc hiện tấm. |
| 03 Khấu góc | Khoét góc xuyên tấm. |
| 04 Cắt tự do | Cắt xiên một góc (nhập 2 khoảng cách) hoặc cắt theo đường qua 2 điểm; chọn giữ phần lớn / trái / phải. |
| 06 Hợp tấm | Ctrl+click ≥ 2 tấm cùng mặt phẳng, cùng độ dày, chạm nhau → gộp vào tấm chọn đầu. |
| 08 Khấu bề mặt | Khoét một phần bề mặt. |
| 09 Cắt theo tấm | Lấy một tấm làm "tấm cắt", chọn các tấm bị cắt, khe hở mỗi phía. Xuyên ở mép → đổi biên dạng; xuyên giữa → khoét lỗ; không xuyên hết → khấu mặt. |
| 10 Bo/Vác góc | Chọn góc trên sơ đồ, bo tròn R hoặc vát C. Có nút "Bỏ hình dạng". |
| 11 Co giãn | Chọn cạnh → nhập mm → Enter. Cộng dồn, ± để bù thêm, có nút bỏ co giãn. |
| 12 Tạo rãnh | Đặt rãnh hậu của tủ. |
| 13 Đảo phủ/lọt | Đổi cánh hoặc ngăn kéo giữa phủ bì và lọt lòng. |
| 14 Ghép bề dày | Nhân độ dày theo số lớp (2 lớp 17.2 → 34.4); "Bỏ ghép" về độ dày gốc. |
| 16 Rãnh LED | Tạo rãnh LED. |
| 17 | Mở phần Ngăn kéo. |
| 18 Vbit | Rãnh V-bit. |
| 19 Xóa tool cả tủ | Xóa mọi gia công đã thêm của tủ. |
| 20 Chia tấm | Chia một tấm (hậu, cánh…) thành 2–50 tấm theo chiều cao hoặc rộng, có khe; tự cập nhật khi đổi kích thước tủ. |
| 21 Cung cạnh | Chọn cạnh, cung lõm (khoét vào) hoặc lồi (phình ra), độ cong mm. |
| 22 Biên dạng tự do | Click trên sơ đồ tấm để đặt điểm (bắt 5 mm, Shift 1 mm) hoặc nhập X/Y; chọn Cắt bỏ vùng / Khoét lỗ xuyên / Thay cả biên dạng. Đa giác tự cắt bị từ chối. |
| 23 Quan hệ 2 tấm | Như menu chuột phải: phủ / lọt / bằng mặt / khe / bỏ. |

![Biên dạng tự do: cung lõm cạnh trên + lỗ tam giác](screenshots/11-bien-dang.png)

Các tool ghi "Sắp có" chưa làm.

## 6. Báo cáo
![Báo cáo](screenshots/06-bao-cao.png)

Mở bằng nút **Báo cáo** trên ribbon. Báo cáo gồm:
- **Costing Report**:
  - m² ván theo vật liệu và độ dày.
  - Mét chỉ dán cạnh.
  - Phụ kiện: bản lề, chốt tầng, ray bi, tay nắm, cam & dowel, thanh oval.
  - Đơn giá sửa được; Ctrl+Z hoàn tác.
  - TỔNG CỘNG.
- **Danh sách cắt** (xuất CSV): tên dạng `[Phòng - Tủ] Tấm`, kích thước cắt, mã chỉ cạnh.
- **Cabinet List**: danh sách tủ theo phòng.

## 7. Sản xuất
Thanh bên trái có các mục sau:
- **Gia công**: bản vẽ trải phẳng mặt A/B.
- **Xếp tấm**: nesting.
- **CNC**: đường chạy dao, mô phỏng, xuất G-code.

---

### Phím tắt hay dùng
| Phím | Việc |
|---|---|
| TAB | Tạo tủ (tab Khung) / Thêm tấm (tab Tạo tấm) |
| ~ | Bật/tắt hiện vùng |
| Alt+~ | Cô lập tủ đang chọn |
| Ctrl+click | Ghim thêm vùng / chọn thêm |
| Chuột phải | Dựng nhanh (vùng) / thao tác nhanh (tấm) |
| Ctrl+Z / Ctrl+Y | Hoàn tác / làm lại |
