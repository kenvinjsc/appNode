#include "../core/cabinet.hpp"
#include "../core/nest.hpp"
#include <thread>
#include <functional>
#include <fstream>
#include <sstream>
#include <filesystem>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <cmath>
#include <chrono>
#define CHECK(c) do { if (!(c)) { std::printf("FAIL line %d: %s\n", __LINE__, #c); std::exit(1); } } while (0)

static bool overlap(const hsx::Panel& a, const hsx::Panel& b) {
  for (int k = 0; k < 3; ++k)
    if (a.o[k] + a.s[k] <= b.o[k] + 1e-6 || b.o[k] + b.s[k] <= a.o[k] + 1e-6) return false;
  return true;
}

static hsx::Params video(double w, hsx::DoorStyle ds, hsx::BackStyle bs) {
  hsx::Params p; p.width = w; p.door = ds; p.back = bs; p.loftHeight = 400; p.loftDoors = 1;
  hsx::Column a; a.doors = 2;               // khoang áo dài + cánh đôi
  hsx::Column b; b.drawers = 3; b.doors = 1; b.shelves = 0;
  p.cols = {a, b};
  return p;
}


// ---- kiểm tra hình học tấm ván ----
static bool segCross(double ax, double ay, double bx, double by, double cx, double cy, double dx, double dy) {
  auto o = [](double px, double py, double qx, double qy, double rx, double ry) { return (qx - px) * (ry - py) - (qy - py) * (rx - px); };
  double d1 = o(ax, ay, bx, by, cx, cy), d2 = o(ax, ay, bx, by, dx, dy), d3 = o(cx, cy, dx, dy, ax, ay), d4 = o(cx, cy, dx, dy, bx, by);
  return ((d1 > 1e-9 && d2 < -1e-9) || (d1 < -1e-9 && d2 > 1e-9)) && ((d3 > 1e-9 && d4 < -1e-9) || (d3 < -1e-9 && d4 > 1e-9));
}
static bool selfIntersects(const std::vector<double>& p) {   // cạnh không kề nhau không được cắt nhau
  size_t n = p.size() / 2;
  for (size_t i = 0; i < n; ++i)
    for (size_t j = i + 2; j < n; ++j) {
      if (i == 0 && j == n - 1) continue;
      size_t i2 = (i + 1) % n, j2 = (j + 1) % n;
      if (segCross(p[2*i], p[2*i+1], p[2*i2], p[2*i2+1], p[2*j], p[2*j+1], p[2*j2], p[2*j2+1])) return true;
    }
  return false;
}
static double polyArea(const std::vector<double>& p) {
  double a = 0; size_t n = p.size() / 2;
  for (size_t i = 0; i < n; ++i) { size_t j = (i + 1) % n; a += p[2*i] * p[2*j+1] - p[2*j] * p[2*i+1]; }
  return a / 2;
}
extern "C" {
int hsx_nest_start(const char*); int hsx_nest_status(int, double*); void hsx_nest_cancel(int);
int hsx_nest_result(int, char*, int); int hsx_nest_sheet(int, int, int, char*, int); int hsx_nest_export_dxf(int, const char*); void hsx_nest_free(int);
}
static std::string cabText(const hsx::Params& p) {            // tham số -> văn bản như Ruby gửi
  std::ostringstream o;
  o << "width=" << p.width << "\ndepth=" << p.depth << "\nheight=" << p.height << "\nloft_height=" << p.loftHeight << "\nloft_doors=" << p.loftDoors << "\n";
  for (auto& c : p.cols) o << "col=" << c.width << "," << c.drawers << "," << c.drawerPitch << "," << c.shelves << "," << c.doors << "\n";
  return o.str();
}
static bool runJob(const std::string& text, hsx::NestResult& out) {
  hsx::NestJob j; std::string err;
  if (!hsx::parseJob(text, j, err)) { std::printf("parseJob: %s\n", err.c_str()); return false; }
  out = hsx::runNest(j, nullptr, nullptr);
  return true;
}
static bool rectsSeparated(const hsx::Placement& a, const hsx::Placement& b, double kerf) {
  const double gx = std::max(b.x - (a.x + a.w), a.x - (b.x + b.w)), gy = std::max(b.y - (a.y + a.h), a.y - (b.y + b.h));
  return std::max(gx, gy) >= kerf - 1e-6;      // cách nhau ít nhất kerf theo một trục
}
static hsx::Params board() { hsx::Params b; b.kind = "board"; b.width = 600; b.depth = 400; b.partThick = 18; return b; }
static void near(double a, double b, double tol, const char* what, int line) {
  if (std::fabs(a - b) > tol) { std::printf("FAIL line %d: %s: %.4f vs %.4f (tol %.4f)\n", line, what, a, b, tol); std::exit(1); }
}
#define NEAR(a, b, tol) near((a), (b), (tol), #a, __LINE__)

int main() {
  for (auto ds : {hsx::DoorStyle::Overlay, hsx::DoorStyle::Half, hsx::DoorStyle::Inset})
    for (auto bs : {hsx::BackStyle::Overlay, hsx::BackStyle::Inset})
      for (double w : {1600.0, 1207.0, 900.0}) {
        auto r = hsx::solve(video(w, ds, bs));
        CHECK(r.ok);
        for (size_t i = 0; i < r.panels.size(); ++i)
          for (size_t j = i + 1; j < r.panels.size(); ++j)
            if (overlap(r.panels[i], r.panels[j])) {
              std::printf("overlap w=%g: %s <> %s\n", w, r.panels[i].name.c_str(), r.panels[j].name.c_str());
              return 1;
            }
        // resize không làm đổi độ dày ván
        for (auto& p : r.panels) if (p.role == "carcass") CHECK(std::abs(p.T - 17.5) < 1e-9);
        CHECK(r.hingeCount > 0);
      }
  // resize giữ nguyên số bộ phận (không tính hậu: số tấm hậu phụ thuộc khổ ván)
  auto nonBack = [](const hsx::Result& q) { size_t n = 0; for (auto& pn : q.panels) n += pn.role != "back"; return n; };
  CHECK(nonBack(hsx::solve(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))) ==
        nonBack(hsx::solve(video(1207, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))));
  // lỗi đầu vào
  hsx::Params bad; bad.width = 30; CHECK(!hsx::solve(bad).ok);
  // xuất DXF
  auto r = hsx::solve(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay));
  // ===== cam chốt =====
  auto count = [](const hsx::Result& q, const char* kind) { int n = 0; for (auto& pn : q.panels) for (auto& h : pn.holes) n += std::string(h.kind) == kind; return n; };
  { auto sp = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay); sp.loftHeight = 0; sp.loftDoors = 0; sp.cols = {sp.cols[0]};
    auto q = hsx::solve(sp); CHECK(q.ok);
    // 1 khoang: đáy + nóc nối vào 2 hông = 4 mối nối, mỗi mối dài 550.5 -> 3 cam; 2 chốt gỗ ở giữa
    CHECK(count(q, "cam") == 12 && count(q, "bolt") == 12 && count(q, "channel") == 12);
    CHECK(count(q, "dowel") == 16);
    for (auto& pn : q.panels) for (auto& h : pn.holes) {
      for (int a = 0; a < 3; ++a) CHECK(h.p[a] >= pn.o[a] - 1e-6 && h.p[a] <= pn.o[a] + pn.s[a] + 1e-6);   // lỗ nằm trong tấm
      const std::string kd = h.kind;
      if (kd == "bolt" || kd == "dowel") {}   // kiểm tra mặt tiếp xúc bên dưới
      if (kd == "cam") {                      // tâm cam cách đầu tấm 34 mm theo trục nối
        double best = 1e9;
        for (int a = 0; a < 3; ++a) if (a != h.axis) best = std::min(best, std::min(std::fabs(h.p[a] - pn.o[a]), std::fabs(pn.o[a] + pn.s[a] - h.p[a])));
        NEAR(best, 34, 1e-6); NEAR(h.dia, 15, 1e-9); CHECK(h.axis == pn.tAxis);
      }
      if (kd == "bolt") {                     // ty vít nằm đúng trên mặt tấm B
        const double face = h.side < 0 ? pn.o[h.axis] : pn.o[h.axis] + pn.s[h.axis];
        NEAR(h.p[h.axis], face, 1e-6); CHECK(pn.tAxis == h.axis);
      }
    }
    auto nc = sp; nc.cam = false; auto q2 = hsx::solve(nc); CHECK(q2.panels.size() == q.panels.size());
    CHECK(count(q2, "cam") == 0 && count(q2, "bolt") == 0 && count(q2, "dowel") == 0 && count(q2, "channel") == 0);
    CHECK(count(q2, "hinge_cup") == count(q, "hinge_cup"));        // bản lề không bị ảnh hưởng
  }
  { auto q = hsx::solve(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay));   // tủ 2 khoang + hộc trên
    CHECK(count(q, "cam") > 12 && count(q, "cam") == count(q, "bolt") && count(q, "cam") == count(q, "channel"));
    CHECK(count(q, "dowel") % 2 == 0); }
  // ===== dán cạnh =====
  { auto sp = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay); sp.loftHeight = 0; sp.loftDoors = 0; sp.cols = {sp.cols[0]};
    auto q = hsx::solve(sp);
    auto find = [&](const char* nm) -> const hsx::Panel& { for (auto& pn : q.panels) if (pn.name == nm) return pn; std::printf("no panel %s\n", nm); std::exit(1); };
    const auto& side = find("Hông trái"), &bot = find("Đáy"), &door = find("K1 Cánh 1");
    // hông: cạnh trước (-Y) và cạnh trên (+Z) lộ -> dán; cạnh sau có hậu che, cạnh dưới chạm sàn
    NEAR(side.band[2], 1.0, 1e-9); NEAR(side.band[3], 0, 1e-9); NEAR(side.band[4], 0, 1e-9); NEAR(side.band[5], 1.0, 1e-9);
    NEAR(side.band[0], 0, 1e-9); NEAR(side.band[1], 0, 1e-9);                // mặt dày không dán
    // đáy: chỉ cạnh trước lộ
    NEAR(bot.band[2], 1.0, 1e-9); NEAR(bot.band[3], 0, 1e-9); NEAR(bot.band[0], 0, 1e-9); NEAR(bot.band[1], 0, 1e-9);
    NEAR(bot.banding, bot.s[0], 1e-9);
    // cánh: 4 cạnh dán 2.0 (các mặt ±X, ±Z); mặt dày (±Y) không dán
    NEAR(door.band[0], 2.0, 1e-9); NEAR(door.band[1], 2.0, 1e-9); NEAR(door.band[4], 2.0, 1e-9); NEAR(door.band[5], 2.0, 1e-9);
    NEAR(door.band[2], 0, 1e-9); NEAR(door.banding, 2 * (door.s[0] + door.s[2]), 1e-9);
    // bảng cắt: kích thước cắt = thành phẩm trừ chỉ dán
    bool seen = false;
    for (auto& row : hsx::cutlist(q)) if (row.role == "front") {
      NEAR(row.cutL, row.L - row.band[2] - row.band[3], 1e-9); NEAR(row.cutW, row.W - row.band[0] - row.band[1], 1e-9);
      CHECK(row.cutL < row.L && row.cutW < row.W); seen = true; }
    CHECK(seen);
    auto nb = sp; nb.band = false; auto q3 = hsx::solve(nb);
    for (auto& pn : q3.panels) for (double v : pn.band) CHECK(v == 0);
    auto tb = sp; tb.bandThick = 0.45; auto q4 = hsx::solve(tb);
    for (auto& pn : q4.panels) if (pn.name == "Hông trái") NEAR(pn.band[2], 0.45, 1e-9);
  }
  { auto b = board(); auto q = hsx::solve(b);                                // tấm ván gia công: mặc định dán 4 cạnh
    NEAR(q.panels[0].band[0] + q.panels[0].band[1] + q.panels[0].band[2] + q.panels[0].band[3], 4.0, 1e-9);
    NEAR(q.panels[0].banding, 2 * 600 + 2 * 400, 1e-9);
    auto f = board(); f.bandEdges = "F"; auto q2 = hsx::solve(f);
    NEAR(q2.panels[0].band[2], 1.0, 1e-9); NEAR(q2.panels[0].band[3], 0, 1e-9); NEAR(q2.panels[0].banding, 600, 1e-9); }
  CHECK(hsx::exportDxf(r, (std::filesystem::temp_directory_path() / "hsx_test.dxf").string()));
  // hậu quá khổ được tách, mỗi tấm <= khổ ván, tổng rộng đúng
  { double sum = 0; int n = 0;
    for (auto& pn : r.panels) if (pn.role == "back") { CHECK(pn.s[0] <= 1220 + 1e-6); sum += pn.s[0]; ++n; }
    CHECK(n == 2 && std::fabs(sum - 1600) < 1e-6); }
  // chi tiết lẻ: đa giác hợp lệ, hộp bao khớp, DXF chạy được
  auto polyOk = [&](const hsx::Params& pp, const char* what) {
    auto q = hsx::solve(pp);
    if (!q.ok) { std::printf("part %s: %s\n", what, q.error.c_str()); std::exit(1); }
    for (auto& pn : q.panels) {
      CHECK(pn.L > 0 && pn.W > 0 && pn.T > 0);
      if (pn.prismAxis >= 0) {
        CHECK(pn.poly.size() >= 6);
        const int ia = (pn.prismAxis + 1) % 3, ib = (pn.prismAxis + 2) % 3;
        double a = 0; size_t n = pn.poly.size() / 2;       // diện tích (shoelace) > 0 vì đi ngược chiều kim đồng hồ
        for (size_t i = 0; i < n; ++i) { size_t j = (i + 1) % n;
          a += pn.poly[2*i] * pn.poly[2*j+1] - pn.poly[2*j] * pn.poly[2*i+1];
          CHECK(pn.poly[2*i] >= pn.o[ia] - 1e-6 && pn.poly[2*i] <= pn.o[ia] + pn.s[ia] + 1e-6);
          CHECK(pn.poly[2*i+1] >= pn.o[ib] - 1e-6 && pn.poly[2*i+1] <= pn.o[ib] + pn.s[ib] + 1e-6); }
        CHECK(a > 0);
      }
    }
    return q;
  };
  { hsx::Params b; b.kind = "bend"; b.radius = 300; b.angle = 90; b.partHeight = 800; b.partThick = 17.5;
    auto q = polyOk(b, "bend");
    // đường cong mịn: bán kính lớn -> nhiều đoạn hơn; sai số dây cung <= 0.3mm
    hsx::Params s = b; s.radius = 30; CHECK(hsx::solve(b).panels[0].poly.size() > hsx::solve(s).panels[0].poly.size());
    CHECK(q.panels.size() == 1); }
  { hsx::Params b; b.kind = "bend"; b.angle = 0; CHECK(!hsx::solve(b).ok); }
  { hsx::Params s; s.kind = "shelf_round"; s.width = 600; s.depth = 400; s.cornerFL = 80; s.cornerFR = 80;
    auto q = polyOk(s, "shelf_round"); CHECK(hsx::exportDxf(q, (std::filesystem::temp_directory_path() / "hsx_part.dxf").string())); }
  for (const char* pf : {"cove", "round", "bevel", "step", "rect"}) {
    hsx::Params m; m.kind = "moulding"; m.profile = pf; m.length = 2000; polyOk(m, pf);
  }
  { hsx::Params m; m.kind = "moulding"; m.profile = "zzz"; CHECK(!hsx::solve(m).ok); }
  { hsx::Params b; b.kind = "beam"; b.length = 1200; auto q = polyOk(b, "beam"); CHECK(q.panels[0].s[0] == 1200); }
  { hsx::Params b; b.kind = "nope"; CHECK(!hsx::solve(b).ok); }

  // ===== tấm ván: bo/vát góc =====
  const double PI_ = 3.14159265358979323846;
  for (const char* st : {"round", "concave", "chamfer"}) {
    auto b = board(); b.cornerStyle = st; b.cornerFL = b.cornerFR = b.cornerBR = b.cornerBL = 50;
    auto q = hsx::solve(b); CHECK(q.ok);
    const auto& pn = q.panels[0];
    CHECK(!selfIntersects(pn.poly));
    const double removed = std::string(st) == "round" ? 50.0 * 50 * (1 - PI_ / 4) : std::string(st) == "concave" ? PI_ * 50 * 50 / 4 : 50.0 * 50 / 2;
    NEAR(polyArea(pn.poly), 600.0 * 400 - 4 * removed, 4 * removed * (std::string(st) == "chamfer" ? 1e-9 : 0.04));   // đa giác nội tiếp cung: sai số sag 0.3mm
  }
  { auto b = board(); b.cornerStyle = "bogus"; CHECK(!hsx::solve(b).ok); }
  // ===== mộng dương/âm =====
  { auto b = board(); b.jointType = "finger"; b.jointMode = "tab"; b.jointEdges = "F"; b.jointCount = 3; b.jointLen = 40; b.jointDepth = 18;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pn = q.panels[0];
    CHECK(!selfIntersects(pn.poly));
    NEAR(polyArea(pn.poly), 600.0 * 400 + 3 * 40 * 18, 1e-6);
    NEAR(pn.o[1], -18, 1e-9);                                   // mộng nhô ra phía trước (y âm)
    NEAR(pn.s[1], 400 + 18, 1e-9); }
  { auto b = board(); b.jointType = "finger"; b.jointMode = "slot"; b.jointEdges = "FB"; b.jointCount = 2; b.jointLen = 40; b.jointDepth = 18;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pn = q.panels[0];
    CHECK(!selfIntersects(pn.poly));
    NEAR(polyArea(pn.poly), 600.0 * 400 - 2 * 2 * 40 * 18, 1e-6);   // 2 cạnh × 2 mộng âm
    NEAR(pn.s[1], 400, 1e-9); }
  { auto b = board(); b.jointType = "dovetail"; b.jointAngle = 10; b.jointCount = 3; b.jointLen = 40; b.jointDepth = 18;
    auto q = hsx::solve(b); CHECK(q.ok); CHECK(!selfIntersects(q.panels[0].poly));
    const double ex = 18 * std::tan(10 * PI_ / 180);
    NEAR(polyArea(q.panels[0].poly), 600.0 * 400 + 3 * 18 * (40 + ex), 1e-6); }
  { auto b = board(); b.jointType = "tee"; b.jointCount = 2; b.jointLen = 40; b.jointDepth = 20;
    auto q = hsx::solve(b); CHECK(q.ok); CHECK(!selfIntersects(q.panels[0].poly));
    const double hd = 10, h = 20;
    NEAR(polyArea(q.panels[0].poly), 600.0 * 400 + 2 * (40 * h / 2 + (40 + 2 * hd) * h / 2), 1e-6); }
  { auto b = board(); b.jointType = "finger"; b.jointCount = 30; b.jointLen = 40; CHECK(!hsx::solve(b).ok); }   // quá dày
  { auto b = board(); b.jointType = "finger"; b.jointLen = 700; CHECK(!hsx::solve(b).ok); }                      // cạnh ngắn
  { auto b = board(); b.jointType = "wat"; CHECK(!hsx::solve(b).ok); }
  // ===== khử dao (dogbone) =====
  { auto b = board(); b.jointType = "finger"; b.jointMode = "slot"; b.jointCount = 2; b.jointLen = 40; b.jointDepth = 18; b.dogbone = 12;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pn = q.panels[0];
    CHECK(!selfIntersects(pn.poly));
    auto plain = board(); plain.jointType = "finger"; plain.jointMode = "slot"; plain.jointCount = 2; plain.jointLen = 40; plain.jointDepth = 18;
    const double a0 = polyArea(hsx::solve(plain).panels[0].poly);
    // mỗi mộng âm có 2 góc đáy: dogbone cắt thêm 3/4 hình tròn trừ 1/4 đã thuộc khoang
    NEAR(polyArea(pn.poly), a0 - 4 * (0.75 * PI_ * 6 * 6), 4 * 0.75 * PI_ * 36 * 0.06);
    // bằng chứng: có đỉnh nằm ngoài khoang mộng (x < mép mộng) -> vùng khử dao
    CHECK(pn.poly.size() > hsx::solve(plain).panels[0].poly.size()); }
  // ===== đục khung + hèm kính =====
  { auto b = board(); b.frameBorder = 60;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pn = q.panels[0];
    CHECK(pn.pockets.size() == 1 && pn.pockets[0].top && std::fabs(pn.pockets[0].depth - 18) < 1e-9);
    NEAR(-polyArea(pn.pockets[0].loop), 480.0 * 280, 1e-6);       // lỗ đi thuận kim đồng hồ (diện tích âm)
    CHECK(!selfIntersects(pn.pockets[0].loop)); }
  { auto b = board(); b.frameBorder = 60; b.frameStyle = "round"; b.frameR = 20;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& lp = q.panels[0].pockets[0].loop;
    NEAR(-polyArea(lp), 480.0 * 280 - 4 * 20.0 * 20 * (1 - PI_ / 4), 4 * 20.0 * 20 * (1 - PI_ / 4) * 0.06); CHECK(!selfIntersects(lp)); }
  { auto b = board(); b.frameBorder = 60; b.dogbone = 6;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& lp = q.panels[0].pockets[0].loop;
    CHECK(!selfIntersects(lp)); CHECK(-polyArea(lp) > 480.0 * 280); }       // khử dao khoét rộng thêm ở 4 góc
  { auto b = board(); b.frameBorder = 60; b.rabbetW = 12; b.rabbetD = 8;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pk = q.panels[0].pockets;
    CHECK(pk.size() == 2 && pk[0].top && !pk[1].top);
    NEAR(pk[0].depth, 18 - 8, 1e-9); NEAR(pk[1].depth, 8, 1e-9);            // 10 + 8 = dày 18: gặp nhau, thành lỗ xuyên
    NEAR(-polyArea(pk[0].loop), 480.0 * 280, 1e-6); NEAR(-polyArea(pk[1].loop), 504.0 * 304, 1e-6); }
  { auto b = board(); b.frameBorder = 250; CHECK(!hsx::solve(b).ok); }
  { auto b = board(); b.frameBorder = 60; b.rabbetW = 70; b.rabbetD = 5; CHECK(!hsx::solve(b).ok); }
  // ===== phay rãnh =====
  { auto b = board(); b.grooveCount = 3; b.grooveW = 8; b.grooveDepth = 9; b.grooveOffset = 20;
    auto q = hsx::solve(b); CHECK(q.ok); const auto& pk = q.panels[0].pockets;
    CHECK(pk.size() == 3 && !pk[0].top);
    for (auto& g : pk) { NEAR(g.depth, 9, 1e-9); NEAR(polyArea(g.loop), 600.0 * 8, 1e-6); }
    // chia đều từ y=20 đến y=W-20: 3 rãnh -> cách đều
    NEAR(pk[1].loop[1] - pk[0].loop[1], pk[2].loop[1] - pk[1].loop[1], 1e-9);
    NEAR(pk[2].loop[1] + 8, 400 - 20, 1e-9); }
  { auto b = board(); b.grooveCount = 1; b.grooveInset = 50; b.grooveFace = "top";
    auto q = hsx::solve(b); CHECK(q.ok); NEAR(polyArea(q.panels[0].pockets[0].loop), 500.0 * 8, 1e-6); CHECK(q.panels[0].pockets[0].top); }
  { auto b = board(); b.grooveCount = 1; b.grooveDepth = 18; CHECK(!hsx::solve(b).ok); }       // xuyên -> không hợp lệ
  { auto b = board(); b.grooveCount = 1; b.grooveOffset = 395; CHECK(!hsx::solve(b).ok); }     // ngoài ván
  // ===== uốn kerf =====
  { hsx::Params k; k.kind = "kerf_bend"; k.width = 600; k.depth = 400; k.partThick = 18; k.kerfW = 3; k.kerfRemain = 2; k.bendRadius = 300; k.kerfStart = 20;
    auto q = hsx::solve(k); CHECK(q.ok); const auto& pk = q.panels[0].pockets;
    const double s = 300.0 * 3 / 16;                                  // R·k/(T-remain)
    NEAR(pk[1].loop[1] - pk[0].loop[1], s, 1e-9);
    CHECK((int)pk.size() == (int)std::floor((360 - 3) / s) + 1);
    for (auto& g : pk) { NEAR(g.depth, 16, 1e-9); CHECK(!g.top); }
    CHECK(q.info.find("rãnh") != std::string::npos);
    auto k2 = k; k2.bendRadius = 20; CHECK(!hsx::solve(k2).ok);       // rãnh chồng nhau
    // bán kính nhỏ hơn -> rãnh dày hơn (bước nhỏ hơn)
    auto k3 = k; k3.bendRadius = 150; auto q3 = hsx::solve(k3); CHECK(q3.ok && q3.panels[0].pockets.size() > pk.size()); }
  // ===== cánh khung/kính =====
  { hsx::Params d; d.kind = "door_frame"; d.width = 500; d.height = 1800; d.stile = 60; d.rail = 70; d.frameThick = 20; d.panelThick = 6; d.panelType = "glass";
    auto q = hsx::solve(d); CHECK(q.ok && q.panels.size() == 5);
    for (size_t i = 0; i < q.panels.size(); ++i) for (size_t j = i + 1; j < q.panels.size(); ++j) CHECK(!overlap(q.panels[i], q.panels[j]));
    double vol = 0; for (auto& pn : q.panels) vol += pn.s[0] * pn.s[1] * pn.s[2];
    NEAR(vol, 2 * 60.0 * 20 * 1800 + 2 * (500 - 120.0) * 20 * 70 + (500 - 120.0) * 6 * (1800 - 140.0), 1e-6);
    CHECK(q.panels[4].role == "glass");
    auto n = d; n.panelType = "none"; CHECK(hsx::solve(n).panels.size() == 4);
    auto bad = d; bad.stile = 300; CHECK(!hsx::solve(bad).ok); }

  // ===== bố cục cây khoang (quy trình như video: khung rỗng -> chia -> thêm ngăn kéo/cánh) =====
  {
    auto lay = [](const char* L, double w = 1600) { hsx::Params p; p.width = w; p.loftHeight = 0; p.loftDoors = 0; p.layout = L; return p; };
    auto cnt = [](const hsx::Result& q, const char* pre) { int n = 0; for (auto& pn : q.panels) n += pn.name.rfind(pre, 0) == 0; return n; };
    const double t = 17.5, Wi = 1600 - 2 * t;
    // khung rỗng: 2 hông + đáy + nóc + 2 tấm hậu, đúng 1 khoang bao trọn lòng tủ
    { auto q = hsx::solve(lay("V(0:L[])")); CHECK(q.ok && q.panels.size() == 6 && q.openings.size() == 1);
      const auto& o = q.openings[0]; NEAR(o.x0, t, 1e-9); NEAR(o.x1, 1600 - t, 1e-9); NEAR(o.z0, 60 + t, 1e-9); NEAR(o.z1, 2400 - t, 1e-9);
      CHECK(q.hingeCount == 0 && count(q, "hinge_cup") == 0); }
    // 3 vách đều -> 4 khoang bằng nhau
    { auto q = hsx::solve(lay("V(0:L[],0:L[],0:L[],0:L[])")); CHECK(q.ok && q.openings.size() == 4 && cnt(q, "Vách") == 3);
      for (auto& o : q.openings) NEAR(o.x1 - o.x0, (Wi - 3 * t) / 4, 1e-9); }
    // "200 cách mép": khoang đầu cố định 200, phần còn lại tự do; kéo rộng tủ thì khoang cố định giữ nguyên
    for (double w : {1600.0, 1207.0, 2000.0}) { auto q = hsx::solve(lay("V(200:L[],0:L[])", w)); CHECK(q.ok);
      NEAR(q.openings[0].x1 - q.openings[0].x0, 200, 1e-9); NEAR(q.openings[1].x1 - q.openings[1].x0, (w - 2 * t) - t - 200, 1e-9); }
    // mọi khoang cố định mà không khớp -> co/giãn tỉ lệ để vừa khít
    { auto q = hsx::solve(lay("V(300:L[],100:L[])", 1000)); CHECK(q.ok);
      NEAR((q.openings[0].x1 - q.openings[0].x0) + (q.openings[1].x1 - q.openings[1].x0) + t, 1000 - 2 * t, 1e-9);
      NEAR((q.openings[0].x1 - q.openings[0].x0) / (q.openings[1].x1 - q.openings[1].x0), 3.0, 1e-9); }
    // chia đợt ngang: khoang trên cao đúng 400
    { auto q = hsx::solve(lay("H(0:L[],400:L[])")); CHECK(q.ok && q.openings.size() == 2);
      NEAR(q.openings[1].z1 - q.openings[1].z0, 400, 1e-9); NEAR(q.openings[1].z0 - q.openings[0].z1, t, 1e-9); }
    // lồng nhau: cột trái chia ngang, cột phải có ngăn kéo + cánh
    { auto q = hsx::solve(lay("V(0:H(0:L[s=2],300:L[l=1]),0:L[d=3,l=1])")); CHECK(q.ok && q.openings.size() == 3);
      for (size_t i = 0; i < q.panels.size(); ++i) for (size_t j = i + 1; j < q.panels.size(); ++j) CHECK(!overlap(q.panels[i], q.panels[j]));
      CHECK(q.openings[1].path.size() == 2 && q.openings[1].tag == "K1.2"); }
    // cánh tự chia theo rộng tối đa một cánh: 1565 / 600 -> 3 cánh; đặt tay l=2 thì đúng 2
    { auto q = hsx::solve(lay("V(0:L[l=-1])")); CHECK(q.ok && cnt(q, "K1 Cánh") == 3);
      auto q2 = hsx::solve(lay("V(0:L[l=2])")); CHECK(q2.ok && cnt(q2, "K1 Cánh") == 2);
      auto p3 = lay("V(0:L[l=-1])"); p3.doorMaxW = 900; CHECK(cnt(hsx::solve(p3), "K1 Cánh") == 2); }
    // số bản lề tự đặt + lỗ đế nằm trên vách đúng; mỗi bản lề có 2 lỗ đế
    { auto q = hsx::solve(lay("V(0:L[l=1,h=4])")); CHECK(q.ok && q.hingeCount == 4 && count(q, "hinge_cup") == 4 && count(q, "hinge_plate") == 8);
      for (auto& pn : q.panels) for (auto& h : pn.holes) if (std::string(h.kind) == "hinge_plate") CHECK(pn.tAxis == 0 && pn.role == "carcass"); }
    // kiểu cánh riêng từng khoang: lọt lòng thụt vào frontReturn, trùm ngoài nhô ra trước
    { auto q = hsx::solve(lay("V(0:L[l=1,m=3],0:L[l=1,m=1])")); CHECK(q.ok);
      for (auto& pn : q.panels) { if (pn.name == "K1 Cánh 1") NEAR(pn.o[1], 19, 1e-9); if (pn.name == "K2 Cánh 1") NEAR(pn.o[1], -18, 1e-9); } }
    // lỗi: cú pháp, khoang quá hẹp, ngăn kéo vượt chiều cao
    for (const char* bad : {"V(", "X", "V(0:L[z=1])", "L[d=1]x", "V(0:L[d=])", "V(-5:L[])", "V(0 L[])", ""}) {
      hsx::Params p; p.layout = bad; auto q = hsx::solve(p);
      if (std::string(bad).empty()) CHECK(q.ok);    // rỗng = dùng bố cục cột kiểu cũ
      else { CHECK(!q.ok); CHECK(q.error.find("Bố cục") != std::string::npos); } }
    { auto q = hsx::solve(lay("V(0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[],0:L[])", 900)); CHECK(!q.ok); }
    { auto q = hsx::solve(lay("V(0:L[d=40])")); CHECK(!q.ok && q.error.find("Ngăn kéo") != std::string::npos); }
    { auto q = hsx::solve(lay("H(0:L[],2250:L[])")); CHECK(!q.ok); }
    // bố cục cũ (cols + hộc trên) và bố cục cây tương đương cho cùng số tấm
    { auto legacy = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay);
      auto tree = legacy; tree.layout = "V(0:H(0:L[l=2],400:L[l=1]),0:H(0:L[d=3,l=1],400:L[l=1]))";
      auto a = hsx::solve(legacy), b = hsx::solve(tree); CHECK(a.ok && b.ok && a.panels.size() == b.panels.size() && a.hingeCount == b.hingeCount); }
    // kiểm thử ngẫu nhiên: cây khoang bất kỳ hợp lệ thì không chồng tấm, mọi khoang nằm trong lòng tủ
    unsigned seed = 12345; auto rnd = [&](int n) { seed = seed * 1103515245u + 12345u; return (int)((seed >> 16) % (unsigned)n); };
    std::function<std::string(int)> gen = [&](int depth) -> std::string {
      if (depth >= 3 || rnd(3) == 0) {
        std::string s = "L[";
        if (rnd(3) == 0) s += "d=" + std::to_string(1 + rnd(3)) + ",";
        if (rnd(3) == 0) s += "s=" + std::to_string(1 + rnd(3)) + ",";
        if (rnd(2) == 0) s += "l=" + std::to_string(rnd(4) - 1) + ",";
        if (rnd(4) == 0) s += "h=" + std::to_string(2 + rnd(5)) + ",";
        if (s.back() == ',') s.pop_back();
        return s + "]";
      }
      const bool v = rnd(2) == 0; const int n = 2 + rnd(3);
      std::string s = v ? "V(" : "H(";
      for (int i = 0; i < n; ++i) { if (i) s += ","; s += std::to_string(rnd(3) == 0 ? 100 + rnd(300) : 0) + ":" + gen(depth + 1); }
      return s + ")";
    };
    int okN = 0, failN = 0;
    for (int it = 0; it < 400; ++it) {
      hsx::Params p; p.width = 800 + rnd(1800); p.height = 900 + rnd(1600); p.loftHeight = 0; p.layout = gen(0);
      auto q = hsx::solve(p);
      if (!q.ok) { ++failN; CHECK(!q.error.empty()); continue; }
      ++okN;
      for (size_t i = 0; i < q.panels.size(); ++i) for (size_t j = i + 1; j < q.panels.size(); ++j)
        if (overlap(q.panels[i], q.panels[j])) { std::printf("fuzz overlap %s: %s <> %s\n", p.layout.c_str(), q.panels[i].name.c_str(), q.panels[j].name.c_str()); return 1; }
      for (auto& o : q.openings) CHECK(o.x0 >= t - 1e-6 && o.x1 <= p.width - t + 1e-6 && o.z0 >= 60 + t - 1e-6 && o.z1 <= p.height - t + 1e-6 && o.x1 > o.x0 && o.z1 > o.z0);
    }
    std::printf("fuzz bố cục: %d hợp lệ, %d bị từ chối đúng lý do\n", okN, failN);
    CHECK(okN > 100);
  }
  // ===== gom chi tiết + xếp ván =====
  {
    const auto cab = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay);
    hsx::NestResult R;
    CHECK(runJob("kerf=4\ntrim=10\n---\ncount=3\n" + cabText(cab), R));
    CHECK(R.ok && R.totalSheets > 0 && R.partCount > 0);
    // số chi tiết = số tấm có thể cắt trong 1 tủ × 3; gom nhóm làm số dòng ít hơn số tấm
    auto sol = hsx::solve(cab); int cuttable = 0;
    for (auto& pn : sol.panels) cuttable += (pn.role == "carcass" || pn.role == "back" || pn.role == "front" || pn.role == "drawer");
    CHECK(R.partCount == 3 * cuttable && (int)R.items.size() < cuttable);
    int unitsPlaced = 0; double areaPlaced = 0;
    for (auto& m : R.mats) {
      CHECK(m.oversize.empty());
      int mp = 0;
      for (auto& sh : m.sheets) {
        for (size_t i = 0; i < sh.items.size(); ++i) {
          const auto& a = sh.items[i];
          CHECK(a.x >= 10 - 1e-6 && a.y >= 10 - 1e-6 && a.x + a.w <= 2440 - 10 + 1e-6 && a.y + a.h <= 1220 - 10 + 1e-6);   // trong vùng cắt được
          for (size_t j = i + 1; j < sh.items.size(); ++j) CHECK(rectsSeparated(a, sh.items[j], 4));                         // không chồng, đủ kerf
          areaPlaced += a.w * a.h; ++unitsPlaced; ++mp;
        }
      }
      CHECK(mp == m.parts);
      NEAR(m.sheetArea, m.sheets.size() * 2440.0 * 1220.0, 1e-6);
    }
    CHECK(unitsPlaced == R.partCount);
    double pa = 0; for (auto& m : R.mats) pa += m.partsArea; NEAR(areaPlaced, pa, 1e-3);
    CHECK(R.wastePct > 0 && R.wastePct < 100);
    // vật liệu tách theo độ dày (thân 17.5, hậu/đáy hộc 9.5, mặt cánh 18)
    CHECK(R.mats.size() >= 3);
    // gom giữa các tủ: 2 phần count=1 giống hệt = 1 phần count=2
    hsx::NestResult A, B;
    CHECK(runJob("---\ncount=1\n" + cabText(cab) + "---\ncount=1\n" + cabText(cab), A));
    CHECK(runJob("---\ncount=2\n" + cabText(cab), B));
    CHECK(A.items.size() == B.items.size() && A.partCount == B.partCount && A.totalSheets == B.totalSheets);
    for (size_t i = 0; i < A.items.size(); ++i) CHECK(A.items[i].qty == B.items[i].qty);
    // tất định
    hsx::NestResult C2; CHECK(runJob("kerf=4\ntrim=10\n---\ncount=3\n" + cabText(cab), C2));
    CHECK(hsx::nestSummaryJson(R) == hsx::nestSummaryJson(C2)); CHECK(hsx::nestSheetJson(R, 0, 0) == hsx::nestSheetJson(C2, 0, 0));
    // kerf lớn hơn -> cần nhiều ván hơn hoặc bằng
    hsx::NestResult K; CHECK(runJob("kerf=20\ntrim=10\n---\ncount=3\n" + cabText(cab), K)); CHECK(K.totalSheets >= R.totalSheets);
    // lề cắt: sheet 2440x1220 trim 10 -> khổ thực 2420x1200
    // quá khổ: tủ cao 3000 > 2420 -> hông không xếp được, báo oversize
    auto tall = cab; tall.height = 3000; hsx::NestResult O; CHECK(runJob("---\ncount=1\n" + cabText(tall), O));
    bool seen = false; for (auto& m : O.mats) for (auto& nme : m.oversize) seen = seen || nme.find("Hông") != std::string::npos; CHECK(seen);
    // xoay: ván khổ dọc 1220x2440 + hông cao 2400: chỉ xếp được khi cho xoay
    hsx::NestResult N1, N2;
    CHECK(runJob("sheet_l=1220\nsheet_w=2440\nrotate=0\n---\ncount=1\n" + cabText(cab), N1));
    CHECK(runJob("sheet_l=1220\nsheet_w=2440\nrotate=1\n---\ncount=1\n" + cabText(cab), N2));
    bool over1 = false, over2 = false;
    for (auto& m : N1.mats) over1 = over1 || !m.oversize.empty();
    for (auto& m : N2.mats) over2 = over2 || !m.oversize.empty();
    CHECK(over1 && !over2);
    bool anyRot = false; for (auto& m : N2.mats) for (auto& sh : m.sheets) for (auto& pl : sh.items) anyRot = anyRot || pl.rot; CHECK(anyRot);
    // tham số sai
    hsx::NestJob bj; std::string err;
    CHECK(!hsx::parseJob("wat=1\n", bj, err)); CHECK(!hsx::parseJob("sheet_l=50\n", bj, err));
    CHECK(!hsx::parseJob("---\ncount=1\nwidth=abc\nbogus=3\n", bj, err));
    hsx::NestResult E; CHECK(runJob("kerf=4\n", E)); CHECK(!E.ok);                     // không có chi tiết
    // xuất DXF xếp ván
    CHECK(hsx::exportNestDxf(R, (std::filesystem::temp_directory_path() / "hsx_nest.dxf").string()));
    std::ifstream f((std::filesystem::temp_directory_path() / "hsx_nest.dxf").string()); std::stringstream ss; ss << f.rdbuf(); CHECK(ss.str().find("SHEET") != std::string::npos && ss.str().find("PARTS") != std::string::npos);
  }
  // ===== luồng nền (không chặn luồng gọi) =====
  {
    using clk = std::chrono::steady_clock;
    const std::string job = "kerf=4\ntrim=10\n---\ncount=400\n" + cabText(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay));
    auto t0 = clk::now(); int h = hsx_nest_start(job.c_str());
    double startMs = std::chrono::duration<double, std::milli>(clk::now() - t0).count();
    CHECK(h > 0); CHECK(startMs < 50.0);                                               // start trả về ngay, không chờ tính xong
    double pr = 0; int st = hsx_nest_status(h, &pr);
    int polls = 0; double lastPr = 0; bool monotone = true;
    while (st == 0 && polls < 20000) {                                                 // luồng gọi vẫn "rảnh": chỉ thăm dò nhẹ
      std::this_thread::sleep_for(std::chrono::milliseconds(1)); st = hsx_nest_status(h, &pr);
      if (pr + 1e-9 < lastPr) monotone = false; lastPr = pr; ++polls;
    }
    CHECK(st == 1); CHECK(monotone); NEAR(pr, 1.0, 1e-9);
    int need = hsx_nest_result(h, nullptr, 0); CHECK(need > 50);
    std::string buf(need + 1, '\0'); CHECK(hsx_nest_result(h, &buf[0], need + 1) == need);
    CHECK(buf.find("\"ok\":true") != std::string::npos && buf.find("\"totalSheets\"") != std::string::npos);
    int sn = hsx_nest_sheet(h, 0, 0, nullptr, 0); CHECK(sn > 20);
    std::string sb(sn + 1, '\0'); hsx_nest_sheet(h, 0, 0, &sb[0], sn + 1); CHECK(sb.find("\"items\"") != std::string::npos);
    CHECK(hsx_nest_sheet(h, 99, 0, nullptr, 0) > 0);                                   // chỉ số sai -> {"ok":false}
    CHECK(hsx_nest_export_dxf(h, (std::filesystem::temp_directory_path() / "hsx_nest_thread.dxf").string().c_str()) == 1);
    hsx_nest_free(h); CHECK(hsx_nest_status(h, nullptr) == -1);
    // hủy giữa chừng
    int h2 = hsx_nest_start(("---\ncount=3000\n" + cabText(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))).c_str());
    hsx_nest_cancel(h2); int s2 = 0, w = 0;
    do { std::this_thread::sleep_for(std::chrono::milliseconds(1)); s2 = hsx_nest_status(h2, nullptr); } while (s2 == 0 && ++w < 20000);
    CHECK(s2 == 3 || s2 == 1);                                                         // hủy kịp (3) hoặc đã xong trước (1)
    if (s2 == 3) CHECK(hsx_nest_result(h2, nullptr, 0) > 0);                           // vẫn lấy được thông báo lỗi
    hsx_nest_free(h2);
    // lỗi tham số -> trạng thái 2 và có thông báo
    int h3 = hsx_nest_start("wat=1\n"); int s3 = 0, w3 = 0;
    do { std::this_thread::sleep_for(std::chrono::milliseconds(1)); s3 = hsx_nest_status(h3, nullptr); } while (s3 == 0 && ++w3 < 5000);
    CHECK(s3 == 2); int n3 = hsx_nest_result(h3, nullptr, 0); std::string b3(n3 + 1, '\0'); hsx_nest_result(h3, &b3[0], n3 + 1);
    CHECK(b3.find("\"ok\":false") != std::string::npos); hsx_nest_free(h3);
    CHECK(hsx_nest_status(-5, nullptr) == -1);
    // nhiều job cùng lúc
    int hs[4]; for (int i = 0; i < 4; ++i) hs[i] = hsx_nest_start(("---\ncount=" + std::to_string(5 + i) + "\n" + cabText(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))).c_str());
    for (int i = 0; i < 4; ++i) { int s = 0, ww = 0; do { std::this_thread::sleep_for(std::chrono::milliseconds(1)); s = hsx_nest_status(hs[i], nullptr); } while (s == 0 && ++ww < 20000); CHECK(s == 1); hsx_nest_free(hs[i]); }
  }
  // hiệu năng: 1000 lần giải + JSON phải rất nhanh (tương tác kéo giãn không giật)
  { auto t0 = std::chrono::steady_clock::now(); size_t sz = 0;
    for (int i = 0; i < 1000; ++i) sz += hsx::toJson(hsx::solve(video(1000 + i % 600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))).size();
    double ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t0).count();
    std::printf("perf: %.3f ms / lần (%zu bytes JSON)\n", ms / 1000, sz / 1000);
    CHECK(ms / 1000 < 5.0); }
  std::printf("OK: %zu panels, %d hinges, %zu cutlist rows\n", r.panels.size(), r.hingeCount, hsx::cutlist(r).size());
}
