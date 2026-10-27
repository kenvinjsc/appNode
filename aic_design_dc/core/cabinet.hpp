// Lõi tính toán tủ tham số (không phụ thuộc SketchUp). Đơn vị: mm.
// Hệ trục SketchUp: X = rộng (trái->phải), Y = sâu (mặt trước y=0 -> hậu y=D), Z = cao.
#pragma once
#include <string>
#include <vector>

namespace hsx {

enum class BackStyle { Overlay, Inset, Flush };    // Kiểu hậu: Phủ (ốp sau hông) / Âm (lùi 8 mm) / Lọt (nằm giữa hông, sát mép sau)
enum class DoorStyle { Overlay, Half, Inset };     // Cánh: Trùm ngoài / Trùm nửa / Lọt lòng

struct Column {            // 1 khoang giữa 2 vách đứng
  double width = 0;        // 0 = tự chia đều phần còn lại
  int drawers = 0;         // số ngăn kéo (xếp từ đáy lên)
  double drawerPitch = 0;  // cao mỗi ngăn kéo, 0 = 180
  int shelves = 0;         // số đợt ngang chia đều
  int doors = 0;           // số cánh (0 = hở, -1 = tự theo rộng tối đa một cánh)
  int doorStyle = -1;      // kiểu cánh riêng của khoang: 0 trùm ngoài, 1 trùm nửa, 2 lọt lòng; -1 = theo tủ
  int hinges = 0;          // số bản lề mỗi cánh (0 = tự theo chiều cao)
  double fixH = 0;         // tấm fix che phía dưới khoang (cao fixH), cánh bắt đầu phía trên tấm fix
  double ductW = 0;        // ống hút mùi: hộp ống rộng ductW nằm giữa khoang, sát hậu (2 tấm dọc + tấm fix hậu)
  double doorGap = 0;      // cánh bắt đầu cách đáy khoang doorGap mm (cánh ngắn hơn khoang, tấm fix nằm sau cánh); 0 = tự
  int hingeSide = 0;       // cánh đơn: 1 bản lề trái, 2 bản lề phải, 0 = trái
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
  bool cam = true;             // liên kết cam chốt: tự dò mối nối giữa các tấm thân tủ và sinh lỗ cam/ty/chốt gỗ
  double camSpacing = 320, camEdge = 37, camBack = 34;  // bước tối đa, cách đầu mối nối, tâm cam cách đầu tấm
  bool band = true;            // dán cạnh theo độ phơi lộ của từng cạnh
  double bandThick = 1.0, bandFrontThick = 2.0;   // dày chỉ cạnh thân tủ / mặt cánh, hộc kéo
  std::string bandEdges = "FRBL";                 // tấm ván gia công: các cạnh dán (F trước, R phải, B sau, L trái)
  double sheetW = 1220;        // khổ ván: hậu rộng hơn sẽ tách thành nhiều tấm bằng nhau
  std::string drawerSlide = "side";   // side = ray bi hông (12.7/bên) | under = ray âm (21/bên, dài slideLen)
  double slideLen = 450;       // chiều dài ray âm tối đa
  double doorMaxW = 600;       // rộng tối đa một cánh (cánh tự chia khi khoang rộng hơn)
  std::string cabType = "std"; // std | base (tủ dưới: không nóc, thay bằng thanh giằng) | wall (tủ treo) | tall (tủ cao)
  double railW = 70;           // rộng thanh giằng (tủ dưới)
  int railCount = 2;           // số thanh giằng (2 = trước + sau; >2 chia đều giữa)
  double topCover = 0;         // tấm đỉnh phủ mặt trên (dày topCover, 0 = không), vd. 9 mm cho tủ dưới
  double topCoverOverhang = 19; // tấm đỉnh nhô ra phía trước
  std::string endPanel;        // tấm ốp hông ngoài thân tủ: "L", "R" hoặc "LR"
  double endPanelW = 50;       // rộng (theo chiều sâu) tấm ốp hông
  bool sideOnPlinth = false;   // hồi (hông) đứng trên chân thay vì chạy xuống sàn
  bool plinthFront = false;    // thêm tấm chân trước (cao = plinth) lùi vào plinthSetback
  double plinthSetback = 50;
  double hangZ = 0;            // độ cao treo: nâng cả tủ lên (tủ treo), đáy tủ cách sàn hangZ
  std::string layout;          // bố cục cây khoang (xem layout.hpp); rỗng = dùng cols/loft kiểu cũ

  // --- chi tiết lẻ (kind != "cabinet") ---
  std::string kind = "cabinet";  // cabinet | bend | shelf_round | moulding | beam | board | kerf_bend | door_frame
  double radius = 100, angle = 90;         // bend: bán kính trong, góc uốn (độ)
  double partHeight = 400, partThick = 17.5;  // bend/shelf_round: cao, dày
  double cornerFL = 0, cornerFR = 0, cornerBR = 0, cornerBL = 0;  // shelf_round: bo góc (dùng width/depth)
  double length = 1000, sizeA = 60, sizeB = 60;  // moulding/beam: dài, rộng(Y), cao(Z)
  std::string profile = "cove";            // moulding: cove | round | bevel | step | rect

  // --- tấm ván (kind = board): bo/vát góc, mộng, đục khung, rãnh, khử dao ---
  std::string cornerStyle = "round";       // round (cung lồi) | concave (cung lõm) | chamfer (vát)
  std::string jointType = "none";          // none | finger (ngón) | dovetail (đuôi én) | tee (chữ T)
  std::string jointMode = "tab";           // tab = mộng dương (nhô ra) | slot = mộng âm (khoét vào)
  std::string jointEdges = "F";            // các cạnh có mộng: F trước, R phải, B sau, L trái
  int jointCount = 3;
  double jointLen = 40, jointDepth = 18, jointMargin = 30, jointAngle = 10;
  double dogbone = 0;                      // đường kính dao khử góc trong (0 = không khử)
  double frameBorder = 0, frameR = 0;      // đục khung: bề rộng khung gỗ còn lại, bo góc trong
  std::string frameStyle = "square";       // square | round | chamfer
  double rabbetW = 0, rabbetD = 0;         // hèm kính: rộng thêm mỗi bên, sâu (mặt dưới)
  int grooveCount = 0;
  double grooveW = 8, grooveDepth = 8, grooveOffset = 20, grooveSpacing = 0, grooveInset = 0;
  std::string grooveFace = "bottom";       // bottom | top
  // uốn kerf (kind = kerf_bend): dùng width/depth/partThick
  double kerfW = 3, kerfRemain = 2, bendRadius = 300, kerfZone = 0, kerfStart = 20;
  // cánh khung/kính (kind = door_frame): dùng width/height
  double stile = 60, rail = 60, frameThick = 18, panelThick = 6;
  std::string panelType = "glass";         // glass | flat | none
  std::vector<Column> cols;
};

struct Hole {
  double p[3];       // tâm lỗ, toạ độ thế giới
  int axis;          // trục khoan (0=X,1=Y,2=Z)
  double dia, depth;
  const char* kind;  // "cam","bolt","channel","dowel","hinge_cup","hinge_plate"
  int side = 0;      // khoan từ mặt nhỏ (-1) / mặt lớn (+1) theo trục khoan; 0 = không xác định
};

struct Panel {
  std::string id;           // ID ổn định: suy ra từ tên cấu trúc (khoang + vai trò), không đổi khi chỉ đổi kích thước
  std::string name, role;   // role: carcass | back | front | drawer
  double o[3];              // góc min
  double s[3];              // kích thước theo X,Y,Z
  double banding = 0;       // tổng chiều dài dán cạnh (mm)
  double band[6] = {0, 0, 0, 0, 0, 0};  // dày chỉ dán theo mặt thế giới: -X,+X,-Y,+Y,-Z,+Z (0 = không dán)
  // Hình lăng trụ: đa giác `poly` (cặp tọa độ thế giới trên 2 trục còn lại: a=(axis+1)%3, b=(axis+2)%3)
  // đùn dọc trục `prismAxis` từ o[axis] dài s[axis]. -1 = hộp thẳng.
  int prismAxis = -1;
  std::vector<double> poly;
  // Khoét/phay trên mặt: đa giác `loop`, mặt "top"/"bottom" theo trục đùn, sâu `depth` (>= dày = xuyên). Thực hiện theo thứ tự.
  struct Pocket { std::vector<double> loop; bool top; double depth; std::string kind; };
  std::vector<Pocket> pockets;
  std::vector<Hole> holes;
  // suy ra khi finalize
  int tAxis = 0, lAxis = 0, wAxis = 0;
  double L = 0, W = 0, T = 0;
};

// Khoang lá (toạ độ cục bộ của tủ, mm): để công cụ chọn khoang bằng chuột.
struct Opening {
  std::string tag;                 // vd. K1, K1.2
  std::vector<int> path;           // đường đi trong cây bố cục
  double x0, x1, z0, z1;
  int drawers = 0, shelves = 0, doors = 0;
};

struct Result {
  bool ok = true;
  std::string error;
  std::vector<Panel> panels;
  int hingeCount = 0;
  std::string info;          // ghi chú tính toán (vd. bước xẻ rãnh)
  std::vector<Opening> openings;   // các khoang lá của bố cục (kind = cabinet)
};

void assignIds(Result& r);          // gán Panel::id ổn định
Result solve(const Params& p);      // kind=cabinet -> tủ; còn lại -> chi tiết lẻ
Result solvePart(const Params& p);
Result solveBoard(const Params& p);
Result solveKerf(const Params& p);
Result solveDoor(const Params& p);
void addJoinery(Result& r, const Params& p);    // lỗ cam/ty/chốt tại mọi mối nối chữ T giữa các tấm thân tủ
void applyBanding(Result& r, const Params& p);  // dán cạnh: mặt cánh/hộc kéo 4 cạnh, thân tủ các cạnh không bị che
void finalizePanel(Panel& a);       // suy ra trục dày/dài/rộng từ hộp bao
// Lăng trụ từ đa giác (hộp bao tự tính): đùn dọc `axis` từ lo, dài len.
Panel makePrism(const std::string& name, const char* role, int axis, const std::vector<double>& poly, double lo, double len);

// Bảng cắt gộp theo (vật liệu, kích thước).
struct CutRow {
  std::string role; double L, W, T; int qty; double banding;
  double cutL, cutW;   // kích thước cắt = thành phẩm trừ chỉ dán
  double band[4];      // dày chỉ: cạnh dài ở W nhỏ, cạnh dài ở W lớn, cạnh ngắn ở L nhỏ, cạnh ngắn ở L lớn
};
std::vector<CutRow> cutlist(const Result& r);

// Xuất DXF xếp ván 2440x1220 (kèm lỗ khoan mặt) cho CNC.
bool exportDxf(const Result& r, const std::string& path, double sheetL = 2440, double sheetW = 1220, double kerf = 8);

// JSON / chuỗi tham số cho Ruby.
bool parseParams(const std::string& text, Params& out, std::string& err);
std::string toJson(const Result& r);

}  // namespace hsx
