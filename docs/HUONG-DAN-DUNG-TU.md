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
- Cánh / ngăn kéo: lề, phủ/lọt, khe hở, thanh chặn.

**Chuột phải vào một tấm** để thao tác nhanh:
- Đổi kệ di động ↔ cố định.
- Căn giữa vùng (50%).
- Chia đều lại.
- Co giãn trên ±20.
- Mở Chỉnh tấm.

## 5. Tool (tab Tool bên trái)
Chọn tấm rồi chọn tool. Các tool đã dùng được:

| Tool | Cách dùng |
|---|---|
| 01 Xóa | Xóa tấm đang chọn (hoàn tác được). |
| 02 Ẩn/Hiện | Ẩn hoặc hiện tấm. |
| 03 Khấu góc | Khoét góc xuyên tấm. |
| 08 Khấu bề mặt | Khoét một phần bề mặt. |
| 11 Co giãn | Chọn cạnh → nhập mm → Enter. Cộng dồn, ± để bù thêm, có nút bỏ co giãn. |
| 12 Tạo rãnh | Đặt rãnh hậu của tủ. |
| 13 Đảo phủ/lọt | Đổi cánh hoặc ngăn kéo giữa phủ bì và lọt lòng. |
| 16 Rãnh LED | Tạo rãnh LED. |
| 17 | Mở phần Ngăn kéo. |
| 18 Vbit | Rãnh V-bit. |
| 19 Xóa tool cả tủ | Xóa mọi gia công đã thêm của tủ. |

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
