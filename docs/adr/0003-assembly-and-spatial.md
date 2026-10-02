# ADR-0003: Quan hệ lắp ghép bằng SAT hộp–hộp

- Broad phase: sort-and-sweep trên AABB (O(n log n)), nới rộng thêm `max_relation_gap_mm`.
- Narrow phase: SAT trên 15 trục giữa hai OBB. Khi các chi tiết tiếp xúc nhau bằng mặt, như tấm nội thất thông thường,
  khoảng hở và độ xuyên tính ra là **chính xác**. Chưa dùng parry3d để tránh thêm phụ thuộc; có thể thay vào sau cùng interface.
- Vùng tiếp xúc: chiếu hai mặt đối diện lên mặt phẳng tiếp xúc rồi cắt đa giác lồi (Sutherland–Hodgman), từ đó
  ra diện tích và polygon trong tọa độ world.
- Phân loại: TOUCH/GAP/PENETRATE (sai số 0,01 mm); PARALLEL/PERPENDICULAR/OBLIQUE theo góc giữa hai pháp tuyến;
  FACE/EDGE/END theo mặt của hộp tham gia tiếp xúc.
- Lưu một chiều (source id < target id). `relations_of(id)` suy ra chiều ngược lúc chạy.
- Narrow phase chạy song song bằng rayon; kết quả cache theo `revision` của document.
