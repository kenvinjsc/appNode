# Tổng hợp tính năng: Parametric Cabinet Editor

Nguồn: spec "nâng cấp AIC CAD thành Parametric Cabinet Editor" (35 mục).
Kiến trúc đích:
- **Web** là 2D editor, inspector và quản lý logical model.
- **SketchUp** hiển thị và chỉnh 3D native.
- **`aic_object_relation`** là core canonical, quản lý object, relation, zone, dependency và state.
- Web, core và SketchUp đồng bộ 2 chiều qua WebSocket/RPC.
- Mọi cập nhật đều incremental.

`aic_object_relation` là dự án riêng. Prompt để triển khai nó nằm ở
[PROMPT-aic-object-relation.md](PROMPT-aic-object-relation.md).

Cột **AIC CAD** cho biết repo này đã có tính năng đó chưa. Chỗ đã có là mẫu tham khảo, có thể dùng lại logic.
Ký hiệu: ✅ có · 🟡 một phần · ⬜ chưa.

## 1. Ba cấp thao tác

| Cấp | Nội dung | AIC CAD |
|---|---|---|
| Cabinet | W/H/D, template, rule, transform, material preset | 🟡 W/H/D, kiểu khung, luật liên kết, luật dán cạnh; chưa có template/preset lưu được |
| Zone | chia khoang, chia tầng, kệ, vách, cánh, ngăn kéo | ✅ ZoneTree, ghim vùng, TAB, menu chuột phải "Dựng nhanh" |
| Panel | dimension, offset, constraint, edge band, machining, contour, relation | 🟡 co giãn, dán cạnh, gia công, bo/cắt; chưa có anchor/offset tường minh |

## 2. Danh sách tính năng theo nhóm

### A. Khung giao diện
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 2 | Layout | Ribbon, Scene Tree, vùng 3D (mirror SketchUp), 2D, Inspector phải | ✅ |
| 2 | Tab inspector | Khung · Tạo tấm · Chỉnh tấm · Quản lý · Thư viện · Cài đặt, nâng thành contextual | ✅ tab có sẵn · 🟡 chưa tự đổi theo loại đối tượng |
| 23 | Context menu | Cabinet: Edit Size, Move, Rotate, Duplicate, Mirror, Save Template, Hide, Lock, Report, Delete · Zone: Add Shelf/Divider/Door/Drawer, Split H/V, Equal Divide · Panel: Edit, Move, Resize, Duplicate, Hide, Lock, Edge Band, Machining, Relations, Delete | 🟡 menu vùng và menu tấm cơ bản; thiếu Mirror, Save Template, Split H/V |
| 32 | Thông báo | Chỉ khi geometry sai, xung đột constraint, mất relation, zone sai, lệnh lỗi, lưu template | ✅ chỉ báo lỗi, bằng tiếng Việt |

### B. 2D editor
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 3 | View | Front, Top, Left, Right, Section, Panel Detail | 🟡 Front / Side / Top; Panel Detail ở workspace Gia công |
| 3 | Chọn | select, multi-select, chọn zone; mỗi hình 2D map `ObjectId`, mỗi zone map `ZoneId` | 🟡 chọn tấm có; chọn zone chỉ ở 3D |
| 5 | Dimension sửa trực tiếp | click số → nhập → `SET_PARAMETER` (W/H/D tủ, rộng/cao khoang, cao kệ, offset, khe cánh, cao ngăn kéo) | ⬜ |
| 9 | Kéo vách | kéo là PREVIEW, thả chuột là COMMIT, chỉ cập nhật zone/tấm liên quan | ⬜ |
| 10 | Kéo kệ | live dimension; snap theo lưới, mặt tấm, trung điểm, chia đều, cao độ lưu sẵn; thả → `SET_SHELF_LEVEL` | ⬜ |
| 14 | Kéo đường chia ngăn kéo | đổi `drawer_height` | ⬜ |
| 16 | Handle resize tấm | 4 handle; FREE (độc lập) / CONSTRAINED (giữ anchor) | ⬜ ở 2D (co giãn bằng số có) |
| 22 | Sửa contour | Cut Corner, Chamfer, Radius, Arc, Notch, Custom Polygon | 🟡 bo/vát góc, cắt xiên, cắt theo tấm, khấu góc; chưa có arc và polygon vẽ tay |

### C. Tham số và cấu trúc
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 4 | Resize tủ có anchor | W: Keep Left/Center/Right · H: Keep Bottom/Center/Top · D: Keep Front/Center/Back; không scale mesh | 🟡 đổi W/H/D theo tham số; chưa có anchor mode |
| 6 | LOCK / AUTO / PERCENT | mỗi khoang có mode; 600 LOCK · AUTO · 400 LOCK → 1600→1800 cho 600·800·400; % giữ tỉ lệ | 🟡 khóa theo vị trí tấm (Tỷ lệ / Cách A / Cách B, chấm đỏ); chưa có mode theo từng khoang |
| 7 | Split Horizontal | 2/3/4/Custom; Equal / Percent / Fixed+Auto / Custom values; sinh vách đứng | 🟡 thêm n vách chia đều hoặc 1 vách theo %; chưa có Custom values |
| 8 | Split Vertical | tương tự, sinh kệ/đợt | 🟡 như trên |
| 11 | Thêm kệ vào zone | 1–9 kệ; Equal / By distance / By level | ✅ Equal và 1 kệ theo khoảng cách · 🟡 By level |
| 12 | Thêm vách vào zone | Center / Percent / cách trái / cách phải, có preview | ✅ (preview chỉ ở form) |
| 13 | Cánh theo zone | 1–4 cánh; Overlay / Inset / Sliding; gap từng phía; bản lề L/R/T/B; tự cập nhật | 🟡 1–n cột × hàng, phủ/lọt/lùa, bản lề L/R/T, thanh chặn; gap từng phía chưa có |
| 14 | Ngăn kéo | Count, loại mặt, phủ/lọt, side clearance, khe mặt, loại/chiều dài ray, loại đáy | 🟡 count, phủ/lọt, khe, hộc, ray tự chọn theo độ sâu; chưa cao từng ngăn |
| 17 | Dynamic anchor | `Object.Edge → Target.Face + offset`; hồi dịch thì kệ tự dài/ngắn; UI liệt kê constraint | 🟡 ngầm qua zone (kệ bám hồi); chưa có anchor tường minh |
| 18 | Relation phủ/lọt | Overlay / Inset / Flush / Gap giữa 2 tấm | 🟡 nóc/đáy phủ/lọt/giằng, cánh phủ/lọt |
| 19 | Offset | Front/Back/Left/Right/Top/Bottom lưu thành tham số (ví dụ setback 30) | 🟡 lùi kệ chung cho cả tủ; co giãn 4 cạnh |

### D. Panel inspector và multi-edit
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 15 | Panel inspector | General (Name, Role, ObjectId, ModelId) · Size · Position · Offset · Constraint · Material · Edge band · Machining · Relations | 🟡 thiếu Offset, Constraint, ModelId |
| 20 | Multi-edit | chọn nhiều tấm, giá trị khác nhau hiện "Mixed", sửa một lần | ⬜ |
| 21 | Copy / Array | clone cả tham số, constraint, dán cạnh, gia công, relation, metadata; Duplicate, Array H/V/Equal | 🟡 Duplicate tủ; chưa có Array |

### E. Template và preset
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 24 | Save as Template | lưu logical model, zone, tham số, relation, constraint, vật liệu, rule phụ kiện, rule gia công | ⬜ (định nghĩa Cabinet đã serialize được, dùng làm nền) |
| 25 | Insert Template | nhập W/H/D, core solve lại cấu trúc | ⬜ |
| 26 | Rule Preset | ví dụ "AIC Wardrobe Standard": 17.2 / hậu 8.6 / rãnh 13 / khe cánh 2 / lùi kệ 30 / dán cạnh trước / cam + dowel | 🟡 giá trị mặc định có, chưa lưu preset theo tên |

### F. Đồng bộ và hiệu năng
| # | Tính năng | Yêu cầu chính | AIC CAD |
|---|---|---|---|
| 27 | Logical model actions | MOVE / ROTATE / COPY / DELETE / HIDE / LOCK_MODEL theo `model_id` (SketchUp không có Group cha) | ➖ không áp dụng (Three.js có cây cha–con) |
| 28 | Đồng bộ Web ↔ SketchUp | Web → Command → Core → Adapter → SU; SU Observer → Interpreter → Core → ChangeSet → 2D patch | ⬜ |
| 29 | Incremental | sửa 1 tấm chỉ invalidate projection của tấm, dimension, zone và relation cục bộ | 🟡 core có event và cache mesh; 2D còn vẽ lại cả view |
| 30 | ChangeSet | `objectsUpdated / zonesUpdated / dimensionsUpdated / relationsUpdated` | 🟡 có event theo object; chưa có zone/dimension/relation |
| 31 | Preview / Commit | kéo: patch cục bộ, SU preview có throttle, không lưu · thả: solve, history, lưu, ChangeSet | 🟡 gizmo move có preview; tham số chưa có |

### G. Sản xuất (Phase 6)
| Tính năng | AIC CAD |
|---|---|
| Gia công (khoan, rãnh, pocket, cắt, VBit, LED) | ✅ |
| Dán cạnh theo luật và theo từng cạnh | ✅ |
| Relations inspector | 🟡 |
| Xuất sản xuất (danh sách cắt, nesting, G-code, báo giá) | ✅ |

## 3. Lộ trình (MVP)

| Phase | Nội dung | Test bắt buộc (§34) |
|---|---|---|
| 1 | Cabinet W/H/D + anchor, dimension sửa được, LOCK/AUTO/PERCENT, chọn zone | A. Resize 1600→1800 |
| 2 | Split zone, kéo vách, kéo kệ, thêm kệ, thêm vách | B. Kéo vách 742→900 · C. Kéo kệ 1133→1250 |
| 3 | Dynamic constraint, phủ/lọt, offset, multi-edit | D. Shelf.Right→RightPanel.InnerFace · E. Multi-edit 6 kệ |
| 4 | Cánh / ngăn kéo tham số, Copy/Array | |
| 5 | Custom contour, Template, Rule preset | F. Lưu template → chèn với W/H/D khác |
| 6 | Gia công, dán cạnh, relations inspector, xuất sản xuất | |

## 4. Definition of Done (§35)

1. Chỉnh tủ bằng dimension trực tiếp.
2. Có LOCK / AUTO / PERCENT.
3. Có Zone Editor.
4. Kéo được kệ và vách.
5. Thêm kệ, vách trực tiếp trên zone.
6. Có constraint động.
7. Có phủ/lọt.
8. Có offset.
9. Có multi-edit.
10. Cánh và ngăn kéo tham số.
11. Có template.
12. Web ↔ SketchUp đồng bộ incremental.
13. Không redraw toàn bộ 2D.
14. Không cần Group cha trong SketchUp.
15. `aic_object_relation` quản lý Logical Model và dependency.
