# AicDesignDC – hướng dẫn cho Claude Code

Plugin SketchUp dựng tủ tham số. Lõi C++17 (`core/`) tính danh sách tấm + chi tiết lẻ, lớp Ruby (`ruby/`) dựng hình trong SketchUp.

## Build & test (làm trước khi sửa bất cứ gì)
- Linux/Mac: `cmake -S . -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build && ./build/test_core`
- Windows (cmd/PowerShell, KHÔNG dùng `./`): `cmake -S . -B build` → `cmake --build build --config Release` → `build\Release\test_core.exe`
- Kết quả đúng: dòng `OK: 30 panels, 15 hinges, 12 cutlist rows` (và dòng `perf:` < 5 ms/lần; thực tế ~0.1–0.3 ms).
- Có `FAIL`/`overlap`: tìm nguyên nhân gốc trong `core/`, không sửa/xóa test cho qua.

## Cài vào SketchUp
Chép `ruby/aic_design_dc.rb` + thư mục `ruby/aic_design_dc/` vào Plugins, kèm thư viện build ra:
`hsx_core.dll` (Win, lấy ở `build\Release\`), `libhsx_core.dylib` (Mac), `libhsx_core.so` (Linux) đặt trong `aic_design_dc/`.
Bản SketchUp 64-bit → build 64-bit, dùng Release (Debug cần runtime VS). Lỗi xem ở Window > Ruby Console.

## Kiểm tra cầu Ruby ↔ C++ không cần SketchUp
```
ruby -e 'class Numeric; def mm; self*0.0393701; end; end
require "./ruby/aic_design_dc/bridge"
r = AicDesignDC::Core.solve(width: 1207, loft_height: 400, loft_doors: 1, cols: [[0,0,0,0,2],[0,3,0,0,1]])
puts r["ok"], r["panels"].size, r["hinges"]'
```
Chạy `ruby -c` cho mọi file `.rb`. Code Ruby phải chạy trên Ruby 2.7 (SketchUp ≤ 2022): không dùng `def x = ...`.

## Quy ước
- Cam chốt + dán cạnh BẬT mặc định (`core/joinery.cpp`); lỗ khoan/dán cạnh là dữ liệu số, không tạo hình trong model.
- Gom chi tiết/xếp ván phải chạy ở luồng nền (`hsx_nest_*`); KHÔNG gọi thẳng việc nặng trên luồng UI SketchUp, và Ruby quét model phải theo lát nhỏ bằng `UI.start_timer` (xem `nest_session.rb`).
- Chạy thêm sanitizer khi sửa code đa luồng: `g++ -std=c++17 -fsanitize=thread -pthread tests/test_core.cpp core/*.cpp -o t && ./t`.
- Không đổi tên thư viện `hsx_core` hay khóa tham số trong `parseParams` (`core/export.cpp`).
- Mọi thay đổi lõi C++ phải kèm test trong `tests/test_core.cpp`.
- Cung tròn chia theo sai số 0.3 mm (`kSag` trong `core/geom.hpp`).
- Báo rõ cái gì đã chạy thật và cái gì chưa (ví dụ chưa có SketchUp).
