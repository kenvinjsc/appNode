// Gom chi tiết nhiều tủ -> bảng cắt gộp -> xếp ván tối ưu (MaxRects). Chạy được ở luồng nền: có tiến độ + hủy.
#pragma once
#include <atomic>
#include <string>
#include <utility>
#include <vector>
#include "cabinet.hpp"

namespace hsx {

struct NestOpts { double sheetL = 2440, sheetW = 1220, kerf = 4, trim = 10; bool rotate = false; };

struct NestItem {                    // một chi tiết đã gộp (cùng vật liệu, kích thước cắt, chỉ dán)
  std::string name, mat;
  double thick = 0, w = 0, h = 0;    // w >= h; w chạy dọc chiều dài tấm (vân gỗ)
  int qty = 0;
  double band[4] = {0, 0, 0, 0};
  std::string role;
};
struct Placement { int item; double x, y, w, h; bool rot; };
struct SheetOut { std::vector<Placement> items; double used = 0; };
struct MatOut {
  std::string mat; double thick = 0;
  std::vector<SheetOut> sheets;
  double partsArea = 0, sheetArea = 0;
  int parts = 0;
  std::vector<std::string> oversize;
};
struct NestResult {
  bool ok = true; std::string error;
  NestOpts opts;
  std::vector<NestItem> items;
  std::vector<MatOut> mats;
  int totalSheets = 0, partCount = 0;
  double wastePct = 0;
};

struct NestJob { NestOpts opts; std::vector<std::pair<int, Params>> cabs; };  // (số lượng, tham số)

// Văn bản job: phần đầu là tuỳ chọn (sheet_l, sheet_w, kerf, trim, rotate); mỗi phần sau (ngăn bằng dòng "---")
// gồm "count=N" và các dòng tham số như hsx_solve.
bool parseJob(const std::string& text, NestJob& job, std::string& err);

// Chạy đồng bộ (gọi từ luồng nền). cancel/progress có thể null.
NestResult runNest(const NestJob& job, const std::atomic<bool>* cancel, std::atomic<double>* progress);

std::string nestSummaryJson(const NestResult& r);                 // tóm tắt, không kèm toạ độ
std::string nestSheetJson(const NestResult& r, int mat, int sheet);  // toạ độ từng tấm ván, lấy theo yêu cầu
bool exportNestDxf(const NestResult& r, const std::string& path);

}  // namespace hsx
