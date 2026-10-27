// Lõi tính toán tủ tham số (không phụ thuộc SketchUp). Đơn vị: mm.
// Hệ trục SketchUp: X = rộng (trái->phải), Y = sâu (mặt trước y=0 -> hậu y=D), Z = cao.
#pragma once
#include <string>
#include <vector>

namespace hsx {

enum class BackStyle { Overlay, Inset };           // Kiểu hậu: Phủ / Âm
enum class DoorStyle { Overlay, Half, Inset };     // Cánh: Trùm ngoài / Trùm nửa / Lọt lòng

struct Column {            // 1 khoang giữa 2 vách đứng
  double width = 0;        // 0 = tự chia đều phần còn lại
  int drawers = 0;         // số ngăn kéo (xếp từ đáy lên)
  double drawerPitch = 0;  // cao mỗi ngăn kéo, 0 = 180
  int shelves = 0;         // số đợt ngang chia đều
  int doors = 0;           // số cánh (0 = hở)
};

struct Params {
  double width = 1600, depth = 560, height = 2400;   // Rộng, Sâu, Cao
  double thick = 17.5, backThick = 9.5;              // Dày ván, Dày hậu
  double plinth = 60;                                // Len chân
  bool hasBack = true;
  BackStyle back = BackStyle::Overlay;
  DoorStyle door = DoorStyle::Overlay;
  double doorThick = 18, gap = 2;
  double frontReturn = 19;     // "Hồi lên trước": cánh lọt lòng thụt vào so với mặt trước thân tủ
  double loftHeight = 0;       // khoang trên (hộc tủ trên); 0 = không có
  int loftDoors = 0;           // số cánh mỗi khoang ở hộc trên
  bool cam = false;            // sinh lỗ cam chốt (để sau; mặc định tắt)
  std::vector<Column> cols;
};

struct Hole {
  double p[3];       // tâm lỗ, toạ độ thế giới
  int axis;          // trục khoan (0=X,1=Y,2=Z)
  double dia, depth;
  const char* kind;  // "cam","bolt","hinge_cup","hinge_plate"
};

struct Panel {
  std::string name, role;   // role: carcass | back | front | drawer
  double o[3];              // góc min
  double s[3];              // kích thước theo X,Y,Z
  double banding = 0;       // tổng chiều dài dán cạnh (mm)
  std::vector<Hole> holes;
  // suy ra khi finalize
  int tAxis = 0, lAxis = 0, wAxis = 0;
  double L = 0, W = 0, T = 0;
};

struct Result {
  bool ok = true;
  std::string error;
  std::vector<Panel> panels;
  int hingeCount = 0;
};

Result solve(const Params& p);

// Bảng cắt gộp theo (vật liệu, kích thước).
struct CutRow { std::string role; double L, W, T; int qty; double banding; };
std::vector<CutRow> cutlist(const Result& r);

// Xuất DXF xếp ván 2440x1220 (kèm lỗ khoan mặt) cho CNC.
bool exportDxf(const Result& r, const std::string& path, double sheetL = 2440, double sheetW = 1220, double kerf = 8);

// JSON / chuỗi tham số cho Ruby.
bool parseParams(const std::string& text, Params& out, std::string& err);
std::string toJson(const Result& r);

}  // namespace hsx
