# TODO cho Claude trên cloud (dev tiếp AicDesignDC)

Cập nhật: 2026-10-27. Trạng thái dựa trên commit `92b10b3`. Đọc `CLAUDE.md` và `README.md` trước.

## 1. Đã kiểm tra thật / chưa
- ĐÃ chạy (Windows, MSVC, Release): `test_core.exe` → `OK: 30 panels, 15 hinges, 12 cutlist rows`; `ruby -c` sạch.
- ĐÃ chạy trong SketchUp 2025 thật (bản d5c9389, trước commit `92b10b3`): extension nạp được, `hsx_core.dll` nạp được, `Core.solve` trả 29 tấm.
- CHƯA chạy trong SketchUp: bản `92b10b3` (cây khoang, công cụ Đợt/Vách/Ngăn kéo/Cánh, option bar, nest nền). Chưa dựng thử hình thật.
- CHƯA dựng lại được tủ bếp mẫu (model `Tu-bep-01`) bằng plugin. Đây là việc ưu tiên số 1 (mục 3).

## 2. Lỗi môi trường đã sửa ở test (Windows)
`tests/test_core.cpp` ghi file vào `/tmp/...` (không có trên Windows) → FAIL. Đã đổi sang `std::filesystem::temp_directory_path()`
(5 chỗ; riêng `hsx_nest_export_dxf` cần `.string().c_str()`). Assertion giữ nguyên. Nếu cloud (Linux) merge, vẫn chạy được.

## 3. Việc còn thiếu, theo thứ tự ưu tiên

### 3.1 Tủ bếp nhiều module (từ video "kéo FredoScale không sai")
Video mẫu cho thấy tủ bếp gồm 3 khối: tủ cao (có cánh trên, 2 thanh ngang), tủ treo 3 khoang, tủ dưới 4 khoang có thanh giằng trên. Người dùng kéo giãn cả cụm mà ván giữ độ dày.
- Plugin hiện chỉ có MỘT tủ (một `Params`/cây khoang). Cần: nhóm nhiều module tủ (cao / treo / dưới) trong một cụm, co giãn chung, mỗi module dựng lại từ lõi.
- Cần kiểu tủ bếp: tủ dưới KHÔNG nóc, thay bằng 2 thanh giằng trên (trước/sau); tủ treo; tủ cao có hộc trên. Chưa có mẫu trong `library.rb`.
- Kiểm tra: kéo giãn bằng Scale gốc (`observers.rb`) trên cụm nhiều module phải ra đúng kích thước, ván giữ độ dày. Chưa thử.
- Việc cụ thể: thêm `Params` cho dạng tủ (base/wall/tall), mẫu thư viện "Tủ bếp", test lõi (không chồng lấn, số tấm đúng), rồi thử trong SketchUp.

### 3.2 Đối chiếu với plugin tham chiếu "Hùng Sản xuất" (video 1:42)
Quan sát từ khung hình (KHÔNG có âm thanh/phụ đề → phần giải thích nút là SUY ĐOÁN, cần người dùng xác nhận):
- Thanh công cụ ~16 nút: logo H, chọn, mắt (ẩn/hiện), bảng "Thông số", căn/chia, đổ/phủ, bo góc, ... , cài đặt (bánh răng), xuất DXF.
- Thanh tuỳ chọn trong lúc dùng công cụ: `Giới hạn: tắt | Tẩy | Xóa cánh | Phủ`.
- Thanh trạng thái hướng dẫn phím: `gõ 1..9: số cột mỗi khoang | ALT: DỌC | Click: chia khe GIỮA(2) | TAB: đổi khe sắp chia (xà 25)`; `SHIFT: Cài đặt (dày cánh, khe, rộng tối đa 1 cánh)`.
- Hộp thoại "Thông số" riêng.
- Cánh hiển thị đường chéo nét đứt (ký hiệu chiều mở).
- Ván bo tròn góc (cạnh dày bo) như ở phút 1:04.
So với plugin mình (đã có): chia khoang bằng click, gõ số ván, Alt đổi đợt/vách, Shift mở cài đặt, hộp Thông số, bo góc ván gia công, DXF.
Còn thiếu / chưa rõ:
- [ ] "Tẩy" (xoá ván/ngăn vừa thêm trong khoang) và "Xóa cánh" — kiểm tra `opening_tools.rb` đã có chưa.
- [ ] "Giới hạn" (giới hạn vùng áp dụng?) — chưa rõ ý nghĩa, hỏi người dùng.
- [ ] "Xà" khi chia khe (TAB đổi khe sắp chia, mặc định 25) — chia khe/xà ở mức khoang.
- [ ] Ký hiệu chiều mở cánh (đường chéo nét đứt) trên mặt cánh.
- [ ] Nút ẩn/hiện (mắt) nhóm chi tiết.
- [ ] Thanh toolbar đủ nút như bản gốc; icon hiện có chỉ: `draw, params, part, divide, door, drawer`.

### 3.3 Mục "Chưa làm" đã có trong README
Bo góc/cong cho thân tủ liền khối; khấu ván khi các tấm giao cắt (cần boolean); ghép hậu theo khổ 1830.
Giả định cần xác nhận: hồi lên trước 19; cam cách đầu 34 / đầu mối nối 37 / bước ≤320; chốt gỗ Ø8 giữa hai cam.

### 3.4 Hạ tầng
- [ ] CI Windows (MSVC) chạy `test_core` — bắt lỗi kiểu `/tmp` sớm.
- [ ] Test tự động nạp thư viện qua Fiddle trên bản đã cài (script trong CLAUDE.md đã đủ, thêm vào CI).
- [ ] Ngoài lề: ruby `Fiddle` trên SketchUp 2025 (Ruby 3.2) đã chạy được; SketchUp ≤2022 (Ruby 2.7) chưa thử trên máy thật.

## 4. Cách thử nhanh trên máy người dùng (Windows)
1. `cmake -S . -B build` → `cmake --build build --config Release` → `build\Release\test_core.exe`.
2. Chép `ruby\aic_design_dc.rb` + `ruby\aic_design_dc\` + `build\Release\hsx_core.dll` (vào `aic_design_dc\`) vào
   `%APPDATA%\SketchUp\SketchUp 2025\SketchUp\Plugins\`.
3. Trong SketchUp (MCP `supex-win`, cổng 9876): `load` file `aic_design_dc.rb` để nạp không cần khởi động lại.
4. Dựng thử vào model MỚI hoặc nhóm riêng; không sửa model thật của người dùng khi chưa được phép.

## 5. Lưu ý cho agent
- Không đổi tên `hsx_core` hay khóa `parseParams` (xem `CLAUDE.md`).
- Mọi thay đổi lõi C++ kèm test trong `tests/test_core.cpp`; báo rõ cái gì đã chạy thật, cái gì chưa.

## 6. Dựng thử tủ bếp "Tu-bep-01" bằng plugin (2026-10-27, SketchUp 2025 thật)
Đã làm: đọc model (120 nhóm ván của plugin khác, tên kiểu `[2705] THANH GIẰNG`), dựng 10 khối bằng `Builder.build` (5 tủ dưới + 5 tủ treo) vào nhóm "AicDC dựng lại tủ bếp (thử)". Kết quả: ra **50 tấm (chỉ khung rỗng, 5 tấm/khối)**, bố cục và kích thước khung đúng, KHÔNG có cánh/ngăn kéo/thanh giằng.

### Kết cấu tủ bếp gốc (mm, toạ độ model: X rộng, Y sâu, Z cao; tổng 3000 × 591 × 2425, x 1449..4449)
- **Tủ dưới** (5 khối, rộng 400 / 900 / 450 / 800 / 400): sâu 561, đáy dày 18 ở z=75 (trên chân 75), hồi 17–18 cao ~779, hậu 9 ở y=2244, **không có nóc**: thay bằng THANH GIẰNG 70×18 (trước, sau, hai bên) ở z≈796–866 (+ "ĐỈNH" 9 mm ở z861–866 rộng bằng khối). Chân trước (CHÂN TRƯỚC) liền 1282 × 17 × 75.
- **Tủ treo** (5 khối, rộng 400 / 900 / 550 / 600 / 500): đáy ở z=1495, cao 880 (khối 600: đáy z=1677–1692, cao 673), sâu 331 (khối trái 350), có ĐỢT DI ĐỘNG, nóc "ĐỈNH" 18 ở z≈2347–2357, PHÀO 50×18 chạy suốt 1450 trên 2 khối (x1849..3299, z2365).
- **Cánh**: CÁNH TRÁI/PHẢI A 400–448 × 785 (z45..830) cho tủ dưới; 448 × 880 cho tủ treo. 6 cánh trái + 6 cánh phải.
- **Ngăn kéo** (3 ngăn ở khối 450): MẶT NGĂN KÉO, ĐÁY NGĂN KÉO, TNK TRÁI/PHẢI/SAU (thành ngăn kéo), **RAY ÂM 450** (ray giấu).
- Khác: TẤM DỌC ×3, TẤM FIX ×2, "HT: 30"/"HT: 20" ×2 (chưa rõ nghĩa, hỏi người dùng).

### Lỗ hổng plugin phát hiện khi dựng (cần Claude Code sửa)
1. [x] (đã sửa: `Params.sanitize` cho `cols` có hiệu lực khi không truyền `layout`) `Builder.build(model, {cols: [[0,drawers,0,0,doors]]})` KHÔNG tạo cánh/ngăn kéo vì `DEFAULTS[:layout]='V(0:L[])'` (khung rỗng) đè lên `cols`. Cần: hoặc `cols` có hiệu lực khi `layout` rỗng/không truyền, hoặc API Ruby thêm cánh/ngăn kéo theo khoang không cần click chuột (để script/test dựng được).
2. [x] (đã làm: `cab_type` base/wall/tall, `rail_w`, `rail_count`, `hang_z`; test lõi) Loại tủ: thiếu `kind = base|wall|tall`. Tủ dưới phải **bỏ nóc, thêm 2–4 thanh giằng** (kích thước/vị trí tham số hoá); tủ treo cần tham số độ cao treo (z đáy 1495/1692) thay vì phải tự dịch bằng Transformation.
3. [x] (đã làm: `plinth_front`, `plinth_setback`, role `plinth`) Chân: `plinth` chỉ 60 mỗi khối; bếp dùng chân 75 liền nhiều khối (CHÂN TRƯỚC).
4. [x] (đã làm: `drawer_slide=under`, `slide_len`; ray ghi vào `info`, chưa vẽ hình ray) Ngăn kéo **ray âm 450** (TNK trái/phải/sau, đáy, mặt); hiện chỉ có ray 12.7.
5. [x] (đã làm: `cluster.rb`, mẫu thư viện "Cụm bếp mẫu", Scale gốc chia lại rộng theo khối `fixed`, phào/thanh che chạy suốt hàng; test `tests/cluster_test.rb`; CHƯA thử trong SketchUp thật) Cụm nhiều khối: nhóm + co giãn chung + phào chạy suốt qua nhiều khối (hiện phào chỉ là chi tiết lẻ).
6. [ ] Chưa kiểm tra: bấm chuột bằng DivideTool/DoorTool/DrawerTool (chỉ test qua API), dialog/toolbar mở bằng menu.
7. [ ] Quy ước trục: lõi dùng Y=sâu, Z=cao (đúng). Lưu ý `Sketchup::BoundingBox#height`=Y, `#depth`=Z khi viết test/đo.
Việc dựng thử có thể chạy lại: 5 tủ dưới `{width:W, depth:561, height:866, plinth:75}`, 5 tủ treo `{depth:331, height:880, plinth:0}`, đặt bằng `Geom::Transformation.translation`.

### Bổ sung (đo trên khối dựng thử): hồi / chân / phào
Mỗi khối `Builder.build` chỉ ra 5 tấm: Hông trái, Hông phải, Đáy, Nóc, Hậu.
8. [x] (đã làm: `side_on_plinth`) **Hồi**: hông chạy từ z=0 (sàn) đến hết chiều cao; tủ bếp gốc đặt hồi TRÊN chân (z≈89 trở lên, hồi dưới cao ~779). Cần tham số "hồi đứng trên chân" khi có chân.
9. [x] (đã làm) **Chân**: `plinth` chỉ nâng Đáy lên z=plinth, KHÔNG tạo tấm CHÂN TRƯỚC (gốc: 1 tấm liền 1282 × 17 × 75 cho nhiều khối, lùi vào so với mặt trước). Cần tấm chân trước (và tuỳ chọn chân sau/bên) có độ lùi.
10. [x] (phần phào chạy suốt hàng đã làm trong cụm; phào riêng lẻ gắn vào một khối thì chưa) **Phào**: không có tấm phào; gốc có PHÀO 50 × 18 ở đỉnh tủ treo, chạy suốt nhiều khối (x1849..3299, z2365). Cần phào gắn theo khối/cụm (dùng `moulding` trong `core/parts.cpp`, nhưng hiện chỉ đặt rời, chưa gắn vào tủ).

### Bổ sung: ống hút mùi, cánh, HT (đọc từ model gốc, mm)
Khối treo rộng 600 (x3299..3899) là khối HÚT MÙI (đáy nâng lên z1692, cao 673):
11. [x] (đã làm: khóa khoang `x=`, 2 tấm dọc + tấm fix hậu; test lõi) **Ống hút mùi** = 2 TẤM DỌC (17 × 202 deep × 650 cao, tại x3490 và x3691, y2051, z1696..2346) + 1 TẤM FIX hậu 220 × 18 × 650 (x3489, y2034). Tức một hộp ống rộng ~220 nằm lệch trong khoang. Plugin chưa có "ống/hộp ống" (cần chi tiết hộp rỗng không đáy gắn vào khoang, có tham số rộng, sâu, cao, vị trí ngang).
12. [x] (đã làm: khóa khoang `f=`, cánh bắt đầu trên tấm fix) **TẤM FIX 600 × 18 × 155** (x3299, y1950, z1540..1695) che phía dưới khoang hút mùi; cánh khối này chỉ cao 810 (z1565..2375, 2 lá 298), KHÔNG phải 880 như các khối treo khác. Cần cánh ngắn hơn khoang + tấm fix bù phía dưới.
13. [x] (đã làm: `r=` bản lề trái/phải cho cánh đơn; kích thước cánh theo khoang trừ khe; cánh 1/2 lá) **Cánh**: kích thước theo khoang trừ khe, không đều: tủ dưới 400 / 448 / 397 (cao 785, z45..830, sâu y1690 = phủ ngoài), tủ treo 447 / 448 / 548 / 498 (cao 880, z1495..2375). Có khoang 2 lá (448+447, 397+397) và 1 lá (400, 548, 498). Cần cánh 1 lá/2 lá, bản lề trái/phải theo khoang, "cánh A" (tên loại) — kiểm tra DoorTool sinh đúng kích thước này.
14. [x] (đã làm trong cụm: thanh che `rect` chạy suốt hàng) **HT: 30 / HT: 20**: thanh che mặt trước đỉnh tủ treo, cao 55 / 50, dày 18, chạy suốt (HT 30: x1849..3299 và x3299..4449, z2365; HT 20 ở khối trái cao 50 tại z2375, cả mặt trước 383 và mặt hông 313). Gần giống PHÀO nhưng liền nhiều khối. Gộp chung với mục 10 (phào/thanh che chạy suốt cụm).
15. [x] (đã làm: `end_panel=L|R|LR`, `end_panel_w`) **TẤM DỌC** ốp hông cuối tủ dưới: 17 × 50 deep × 780 cao (x4402, y1708, z75..855).
Ghi chú: component "ThomThom" 803×803×1856 trong model là hình người mẫu (entourage), không phải tủ.

## 7. Dựng lại tủ bếp bằng bản f9bcf06 (SketchUp 2025 thật, 2026-10-27)
`Cluster.create(model, Cluster.sample_kitchen, tr)` → thành công: nhóm "Cụm tủ", **95 tấm**, 2967 × 579 × 2430 mm (gốc 3000 × 591 × 2425, 120 nhóm). Có: hồi, đáy, nóc (tủ treo), chân trước ×5, thanh giằng trước/sau ×5, cánh 1 lá/2 lá, 3 ngăn kéo (khối 450), ống dọc trái/phải + tấm fix hậu + tấm fix 155 (khối hút mùi), tấm ốp hông phải, phào chạy suốt. Đã đối chiếu bằng ảnh chụp cạnh tủ gốc: bố cục tổng khớp. Còn lệch (cần sửa):
16. [x] (đã sửa mẫu: khối trái hộc mở `s=2`, sâu 350) Khối treo ngoài cùng bên trái (rộng 400, sâu 350) ở tủ gốc là **hộc MỞ 3 tầng, không cánh**; mẫu `kitchen_spec` đang gắn cánh (`l=-1`). Sửa mẫu: layout hộc mở có đợt (vd `V(0:L[s=2])`) và sâu 350.
17. [x] (đã làm: `top_cover`, `top_cover_overhang`; mẫu tủ dưới dùng 9 mm; tấm đỉnh theo từng khối, chưa liền qua nhiều khối) Tủ dưới gốc có **ĐỈNH 9 mm** phủ liền mặt trên các khối (z≈861–866, sâu ~580–581, dài 371/900/1650); bản dựng thử để hở (chỉ có thanh giằng). Thêm tấm đỉnh/mặt phủ (tuỳ chọn, rộng chạy qua nhiều khối).
18. [x] (xử lý: thanh che HT nhô `ext_r`=50 ra phải → dài 3000 như gốc; tổng 2950 + ốp hông 17.5; cần bên test đo lại) Tổng rộng 2967 so với gốc 3000: gốc có thêm ốp/hồi chồng ~33 mm (khối trái gốc bắt đầu x1449 và tủ dưới kéo tới x4402+50). Kiểm tra cách tính `Cluster.placements` (khe giữa khối, hồi dùng chung hay hồi đôi).
19. [x] (đã làm: khóa khoang `g=60` → cánh 810 phủ lên tấm fix, đỉnh z2375; cần đo lại trong SketchUp) Khối hút mùi: đã có ống + tấm fix + cánh 2 lá; cần đo lại kích thước cánh (gốc 298 × 810, z1565..2375) so với bản dựng.
20. [ ] Chưa kiểm tra bằng chuột: co giãn cụm bằng Scale gốc, DivideTool/DoorTool/DrawerTool, mở dialog/toolbar qua menu.

## 8. Người dùng chụp mặt SAU: kết cấu CHƯA khớp (thu hồi nhận định "khớp" ở mục 7, vốn chỉ dựa vào ảnh nhìn trước)
Ảnh người dùng gửi (Screenshot 2026-10-27 123447.png): trái = bản dựng plugin, phải = tủ gốc, nhìn từ phía sau. Số đo (mm, gốc toạ độ = góc trái-trước của cụm gốc x1449, y1690):
21. [ ] **Chưa có ĐỢT**. Gốc có 10 tấm ĐỢT DI ĐỘNG, bản dựng 0. Vị trí gốc (x rộng+, y sâu, z cao):
    - Tủ dưới: 3 đợt — khối 400 (rộng 365, sâu 514, z462), khối 800 (rộng 765, z463), khối 400 cuối (365, z462). Các khối 900 và 450 không có đợt (900: cánh đôi, chỉ thanh giằng; 450: ngăn kéo).
    - Tủ treo khối trái 400 (hộc mở, sâu 350): 2 đợt rộng 365, sâu 314, z1790 và z2086 → 3 hộc.
    - Tủ treo 900: 1 đợt 865 × 284 sâu, z1945; 550: 1 đợt 515, z1945; 500: 1 đợt 465, z1945.
    - Khối hút mùi 600: 2 đợt 172 rộng × 220 sâu (z1945) nằm HAI BÊN ống (x≈3317 và 3709 tuyệt đối), không phải một đợt suốt.
    Cần: đợt theo khoang (`L[s=n]`) trong `kitchen_spec` + đợt hai bên ống hút mùi; kiểm tra đợt lùi mặt trước (sâu 514 so với thân 561; 284 so với 331).
22. [ ] **Hậu**: gốc hậu dày 9, rộng = khoang − 19 (400→381, 900→880/881, 450→431, 800→781, 550→531, 600→581, 500→481), cao 781 (tủ dưới, z84..865) / 851–861 (tủ treo, z1504..), đặt sát lưng (tủ dưới y554..563; tủ treo y564..573), có lỗ khoan (thấy các chấm cam ở thanh giằng sau). Bản dựng: hậu dày 10, rộng bằng đúng khoang (phủ ngoài, back_style=overlay), cao 791 từ z75. Dùng `back_style: 'inset'`, `back_thick: 9` trong `kitchen_spec` và kiểm tra hậu khối hút mùi cao 668 (z1687).
23. [ ] **Khối treo trái 400**: gốc KHÔNG có hậu như các khối khác (màu tối riêng, sâu 350) — kiểm tra lại có hậu hay hở lưng.
24. [ ] **Tủ dưới nhìn từ sau**: gốc thấy thanh giằng sau + lỗ khoan ở 4 góc, mặt ĐỈNH 9 mm phủ phía trên; bản dựng hở. (Gộp mục 17.)
25. [ ] Cần so lại bằng ảnh sau mỗi lần sửa: chụp cả mặt trước lẫn mặt sau, đo từng nhóm tấm (tên, rộng, sâu, cao, vị trí) so với bảng ở trên, KHÔNG kết luận "khớp" khi mới nhìn một mặt.

## 9. Ảnh xuyên thấu (Screenshot 123549): khối hút mùi sai chiều đứng; thiếu đợt/cánh chéo
26. [ ] **Khối hút mùi (600)**: ngang đúng (ống trái x+190, phải x+393, tấm fix hậu 185–220 rộng ở giữa) nhưng DỌC SAI: bản dựng cao 880 từ z1495 với ống cao 845 từ z1513; gốc khối này cao **673, đáy nâng lên z1677 (hồi z1692..2365)**, ống cao **650 (z1696..2346)** nằm trên tấm fix mặt trước 600×155 (z1540..1695), cánh 2 lá cao 810 (z1565..2375). Cần: `hang_z`/chiều cao riêng cho khối hút mùi trong `kitchen_spec` (cao 673, hang 1692) và ống tính theo chiều cao khoang trừ tấm fix.
27. [ ] **Đợt hai bên ống**: mỗi bên 172 × 220 sâu tại z1945, trong 650 cao của ống → trong khối 673.
28. [ ] Gốc cánh có đường chéo nét đứt (ký hiệu mở) và lỗ khoan/cam ở 4 góc thanh giằng, bản dựng chưa có ở mặt cánh (đường chéo) — xem mục 3.2.
Nhắc lại cách kiểm: chụp xuyên thấu cả hai bên cạnh nhau và đo bảng số, không kết luận bằng mắt một mặt.

## 10. YÊU CẦU người dùng: hậu phải chọn được kiểu — PHỦ / LỌT / ĐÂM XUYÊN (và khớp hậu gốc)
Hiện: `BackStyle { Overlay, Inset }` (`core/cabinet.hpp:9`), khóa `back_style` = `overlay|inset` (`core/export.cpp:117`), dialog có "Phủ / Âm" (`dialog.html:32`). Người dùng nói hậu bản dựng "chưa khớp hậu của họ" và cần **3 kiểu chọn được (tùy biến, người dùng tự chọn)**:
29. [ ] **Phủ**: hậu phủ ngoài lưng, rộng/cao bằng khung ngoài (kiểu hiện tại, dày 10 → dùng `back_thick`).
30. [ ] **Lọt**: hậu nằm trong khung, ăn vào rãnh hồi/đáy/nóc. **Đây là kiểu của tủ bếp gốc** (đo từ model): hậu dày 9, rộng = khoang − 19 (khoang 400→381, 450→431, 550→531, 600→581, 800→781, 900→880/881, 500→481). Hông trong rộng 365 (400 − 17.5×2) mà hậu 381 ⇒ mỗi bên ăn vào hồi ~8 mm (rãnh). Cao 781 tủ dưới (z84..865), 851–861 tủ treo; đặt sát lưng (tủ dưới y554..563 trong sâu 561; tủ treo y564..573). Cần tham số rãnh: sâu rãnh (8), lùi từ lưng (tủ treo 18), dày hậu.
31. [ ] **Đâm xuyên** (người dùng ĐÃ xác nhận): hậu chạy LIỀN SUỐT qua nhiều khối, không chia theo từng khoang. Là tùy chọn người dùng tự chọn (không bật mặc định): với cụm nhiều khối, khi chọn "đâm xuyên" thì tạo một tấm hậu liền cho cả hàng (tủ dưới / tủ treo) thay vì mỗi khối một hậu; kích thước theo cả hàng trừ viền, vẫn tôn trọng dày hậu và rãnh. Chọn riêng được cho từng hàng hoặc từng khối (vd chỉ gộp các khối liền kề), thể hiện ở `Cluster.create/rebuild`. Cần cắt hậu liền khi có ống hút mùi/khối nâng cao khác độ cao.
32. [ ] UI: chọn kiểu hậu ở dialog "Thông số" (`dialog.html`) và option bar; khóa `back_style` nhận `overlay|inset|through` (+ tham số `back_groove`, `back_setback`). Không đổi tên khóa cũ (`CLAUDE.md`): chỉ thêm giá trị mới; test lõi mỗi kiểu (không chồng lấn tấm, kích thước hậu đúng bảng trên).
33. [ ] `Library.kitchen_spec` dùng kiểu "Lọt" với dày hậu 9 và hậu khối hút mùi cao 668 (z1687) để khớp gốc.

## 11. Chức năng chưa BÁM SÁT video "Hùng Sản xuất" (xem lại 40 khung hình, không có tiếng — vài chỗ là suy luận, đã đánh dấu "?")
Người dùng phản hồi: "chức năng chưa bám sát video". Đối chiếu video với `option_bar.rb`, `dialog.html`, `library.rb`, `draw_tool.rb`:
34. [ ] **Một công cụ "thêm ván vào khoang" có tab loại ván**: video có thanh chip `Hồi Phải | Đợt | Nóc | Đáy | Hậu | + | − | ⚙` (chọn loại ván cần thêm/bớt trong khoang đang chọn) và hàng 2 `Xem trước: bật | Cánh phủ | Cánh lọt`. Gõ số: `3` = 3 vách đều, `/3` = 3 khoang, `200` = cách mép 200 → xem trước rồi Enter; Alt đổi đợt↔vách. Plugin hiện TÁCH 3 công cụ (DivideTool/DrawerTool/DoorTool), chip chỉ có "Đợt (ngang)/Vách (đứng)", không có chọn Hồi/Nóc/Đáy/Hậu, không có +/−, không có công tắc "Xem trước".
35. [ ] **Chip "Giới hạn: tắt | Tẩy | Xóa cánh | Phủ"** (xuất hiện khi dùng công cụ): "Tẩy" (xóa ván vừa thêm), "Xóa cánh", "Phủ" (kiểu cánh phủ) và "Giới hạn" (? giới hạn vùng áp dụng) — plugin chỉ có thao tác "Xóa ngăn kéo/Xóa cánh" trong từng công cụ, chưa có "Tẩy" và "Giới hạn".
36. [ ] **Hộp "Thông số"** của video: nhóm "Khung tủ" với `Dày ván 17.5 | Dày hậu 9.5 | Len chân 60 | Len nóc 0 | Có hậu ☑ | Kiểu hậu Phủ | Cánh Phủ… | Hồi lên trước 19`, nút `Tạo mới | Đóng`. Thiếu **"Len nóc"** (tham số nâng/lùi nóc) và nút "Tạo mới/Đóng" kiểu đó; dialog plugin thì nhiều trường hơn nhưng không theo nhóm/thứ tự như video.
37. [ ] **Chế độ "ĐÃ NHẬN KHUNG TỦ — click tạo liên tục"** (thanh trạng thái video): công cụ nhận khung tủ đã chọn rồi cho click liên tục nhiều lần để thêm ván, không phải chọn lại. Và lúc vẽ: `Click điểm 1 trên mặt tủ | SHIFT: Cài đặt (dày cánh, khe, rộng tối đa 1 cánh)` (? vẽ/đặt trên mặt tủ). `draw_tool.rb` hiện: click 2 điểm rộng×sâu trên mặt đất.
38. [ ] **Thư viện** là panel HTML có cột danh mục kèm số lượng (`Tất cả 21 | Khung tủ 14 | Ngăn kéo 2 | Phào 1 | Uốn cong 2 | Xà 1 | Thư viện người dùng 1`), ô "Tìm kiếm...", nút "Lưu mẫu", lưới thumbnail: Khung tủ, Khung tủ quá khổ, Khung line, Bàn trang điểm treo, Kệ vuông, Khung góc thùng, Khung tủ áo góc cong, Khung tủ bếp dưới, Ngăn kéo treo, Hệ phào… (21 mẫu). Kiểm tra `library.rb`/`dialog.html`: có thumbnail? có đếm theo danh mục? có tìm kiếm? chưa rõ.
39. [ ] **Thân tủ bo cong liền khối** (khung góc thùng, tủ áo góc cong, kệ cong bậc thang) và **ván uốn rãnh (kerf)** thấy ở phút ~1:04 và 1:34–1:39: tấm cạnh bo tròn (màu xanh), kệ bậc cong, ván có rãnh song song. Plugin có `bend`/`shelf_round`/kerf rời nhưng KHÔNG có thân tủ cong liền khối (README ghi "Chưa làm").
40. [ ] **Toolbar "Hùng Sản xuất" 16 nút** (H, chọn, mắt, thông số, căn/chia, đổ/tô, bo góc, mộng×3, góc L, thanh ngang, cam, chia, ghép, gộp, cài đặt) + nhóm `.DXF / in / xuất / xuống / chạy` bên cạnh. Plugin chỉ có icon `draw/params/part/divide/door/drawer`.
41. [ ] Văn bản thanh trạng thái/tooltip nên dùng đúng chữ video (xem trên) để người dùng nhận ra công cụ.
Cách kiểm sau khi sửa: mở video, so từng cảnh với giao diện thật; không kết luận "khớp" khi chưa đối chiếu cả thao tác lẫn chữ.

## 12. NGUỒN CHUẨN: kho kết cấu của DesignApp (chỉ đọc) — ưu tiên hơn số tôi tự đo ở mục 6-11
Phiên "DesignApp" (Claude khác trên máy này) đã trích kết cấu THẬT của xưởng từ chính `Tu-bep-01.skp` ra kho:
`D:\AIC\KHO-KET-CAU-NOI-THAT\mau\BEP-B2.02-10\` — `MO-TA.md` (§2 kết cấu chung, §4 bảng từng tủ), `BANG-TAM.csv`, `KHOI-THE-GIOI.csv`, `VI-TRI-TU.csv`, `KHE-HO.md`, `LO.csv` (713 lỗ). Dùng để đối chiếu ±0,5 mm. KHÔNG ghi vào kho; cần thêm thì nhắn DesignApp (mã tấm + số của mình). File gốc có thêm một bản sao cả bếp đặt lệch ~10 m: bỏ qua.
Sửa so với mục 6-11 (số tôi đo bằng bounding box bị cộng 3–6 mm do khối con rãnh HAU-8; DesignApp đã đo bằng khối đặc):
42. [ ] **Hồi tủ dưới cao 773,5** (z17,5→791), không phải 779; **hồi ĐỨNG TRÊN mặt đáy** (không chui vào đáy), đáy **trùm** rộng bằng cả thùng, dày 17,5; ván thùng 17,5, hậu 9, mặt ngoài cánh 17,5 (B900 dùng 18 cho giằng/hồi). Góc trên-trước hồi **khấu 70×17,5** (z721→791, y0→17,5) cho giằng trước.
43. [ ] **Không nóc**: 2 giằng đứng cao 70, mặt trên bằng mặt trên hồi (z791). Giằng trước dài hết rộng, nằm trong khấu, **khoét hình thang** giữa mép trên (sâu 20, miệng 400 x200→600, đáy 300 x250→550). Giằng sau = rộng − 35, lọt giữa hai hồi, ngay sau hậu (y543→560,5).
44. [ ] **Hậu lọt rãnh HAU-8** trên 2 hồi + đáy: rãnh rộng 10, sâu 9,5, dao T1-D5, mép rãnh cách mép sau 17,5; hậu ăn mỗi rãnh 8 ⇒ hậu = lọt lòng + 16 = rộng − 19 (800 → 781 rộng × 781,5 cao, z9,5→791). **Rãnh đâm xuyên lố 3 mm mỗi đầu (`extra_grooves=3`) BẮT BUỘC khi gia công** (người dùng duyệt 26/10); đó là thông số dao, không phải kích thước tấm, và không phải hồi đâm vào đáy.
45. [ ] ⚠ **MÂU THUẪN về chữ "đâm xuyên"** — cần người dùng chốt trước khi code mục 31: (a) hồi người dùng trả lời tôi hôm nay: "hậu chạy liền suốt qua nhiều khối, không chia theo từng khoang"; (b) tài liệu kho/DesignApp: "đâm xuyên" = rãnh hậu chạy lố 3 mm mỗi đầu. Có thể cả hai cần làm: kiểu hậu (Phủ/Lọt/Liền suốt) và tùy chọn rãnh lố 3 mm. Đừng chọn một mà bỏ cái kia.
46. [ ] **Đợt di động** lọt lòng (rộng − 35), sâu 514, lùi 20; lỗ chốt đợt Ø5 sâu 13, 6 lỗ/hồi.
47. [ ] **Cánh phủ bì 17,5**, khe dọc 2 giữa mọi mặt liền nhau (cả giữa 2 tủ); mặt cánh dưới z45→830 cả dãy (chùm xuống len chân 30), mép trên thấp hơn hồi 36; 2 cánh 397,3 × 785 (tủ 800); mặt hộc B450 khe ngang 25; cánh trên: mặt trước y240,5. Chi tiết `KHE-HO.md`.
48. [ ] **ĐỈNH 9** trên cụm tủ dưới (B400-1: 1650×580×9 ở z791; B900: 900×580,5×9 ở z786). Chân trước liền: B900 chạy x20→1303 (1282); cụm B400-1 chạy x1303→3000 (1697).
49. [ ] **Liên kết**: cam Ø15 (CCG-35, sâu = dày − 3,5) + chốt cam Ø5 (hồi↔đáy 5 cam/hồi, hồi↔giằng), chốt gỗ đáy↔hồi (2/bên), bản lề 2 bộ/cánh (cốc trên cánh, đế trên hồi). Tọa độ từng lỗ: `LO.csv`.
50. [ ] **Tủ khác nhau, không phải 5 tủ giống nhau**: B400-2 (0→400) sâu 590,5 cao 875; B900 sâu 580,5 cao 795; B450 & B800 sâu 560,5 cao 791 (lùi y20) nằm dưới ĐỈNH cụm B400-1; W400 sâu 350 cao 880 (y240,5, z1495); W900/W550/W500 sâu 330,5 cao 870; W600 hút mùi cao 687,5 ở z1677,5. Thứ tự x thật: B400-2, B900, B450, B800, B400-1 (dưới) và W400, W900, W550, W600, W500 (trên) — khớp thứ tự rộng đã dùng (400/900/450/800/400).
51. [ ] **Tủ trên**: đáy/đỉnh sâu 330,5 có rãnh hậu (lố 3 cũng bắt buộc); hồi 852,5 (W400 862,5; W600 670); hậu 9 rãnh; phào/tấm ngang HT:30 phía trên. **PHÀO/HT KHÁC NHAU từng tủ** (W400 HT:20; W500/W550 HT:30; phào 1450 chạy x400→1850 và 1150 chạy x1850→3000) — KHÔNG lấy 50–55 làm luật chung (sửa mục 14): xem §4 MO-TA.md.
52. [ ] **B450**: 3 ngăn kéo ray âm; thanh giằng 415×60 ở z170,5 và z550.
53. [ ] Cách kiểm: dựng từng tủ theo §4, xuất bảng tấm CSV, so với `BANG-TAM.csv` ±0,5 mm. DesignApp có thang chấm `giao/THANG-DIEM.md` (mục B "kết cấu" dùng §2).
