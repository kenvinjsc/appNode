// Chi tiết lẻ dựng từ biên dạng đa giác: uốn cong, đợt bo góc, phào, xà.
#include "cabinet.hpp"
#include <algorithm>
#include <cmath>

#include "geom.hpp"

namespace hsx {
using namespace geom;

namespace {
void roundedRect(Poly& out, double x0, double y0, double x1, double y1, double fl, double fr, double br, double bl) {
  auto cl = [&](double r) { return std::max(0.0, std::min(r, std::min(x1 - x0, y1 - y0) / 2)); };
  std::vector<Vtx> vs = {{{x0, y0}, ROUND, cl(fl)}, {{x1, y0}, ROUND, cl(fr)}, {{x1, y1}, ROUND, cl(br)}, {{x0, y1}, ROUND, cl(bl)}};
  out = build(vs);
}
}  // namespace

Panel makePrism(const std::string& name, const char* role, int axis, const std::vector<double>& poly, double lo, double len) {
  Panel a;
  a.name = name; a.role = role; a.prismAxis = axis; a.poly = poly;
  const int ia = (axis + 1) % 3, ib = (axis + 2) % 3;
  double mnA = 1e18, mxA = -1e18, mnB = 1e18, mxB = -1e18;
  for (size_t i = 0; i + 1 < poly.size(); i += 2) {
    mnA = std::min(mnA, poly[i]); mxA = std::max(mxA, poly[i]);
    mnB = std::min(mnB, poly[i + 1]); mxB = std::max(mxB, poly[i + 1]);
  }
  a.o[axis] = lo; a.s[axis] = len;
  a.o[ia] = mnA; a.s[ia] = mxA - mnA;
  a.o[ib] = mnB; a.s[ib] = mxB - mnB;
  a.banding = 0;
  return a;
}

void finalizePanel(Panel& a) {
  int t0 = 0;
  for (int k = 1; k < 3; ++k) if (a.s[k] < a.s[t0]) t0 = k;
  int o1 = (t0 + 1) % 3, o2 = (t0 + 2) % 3;
  if (a.s[o1] >= a.s[o2]) { a.lAxis = o1; a.wAxis = o2; } else { a.lAxis = o2; a.wAxis = o1; }
  a.tAxis = t0; a.T = a.s[t0]; a.L = a.s[a.lAxis]; a.W = a.s[a.wAxis];
}

Result solvePart(const Params& p) {
  Result r;
  auto fail = [&](const char* m) { r.ok = false; r.error = m; return r; };
  const std::string& k = p.kind;
  if (k == "board") return solveBoard(p);
  if (k == "kerf_bend") return solveKerf(p);
  if (k == "door_frame") return solveDoor(p);

  if (k == "bend") {  // Uốn cong: dải cung tròn, đùn theo Z
    if (p.radius < 1 || p.partThick < 1 || p.partHeight < 1) return fail("Thông số uốn cong không hợp lệ");
    if (p.angle <= 0 || p.angle > 360) return fail("Góc uốn phải trong (0, 360]");
    const double ri = p.radius, ro = p.radius + p.partThick;
    Poly poly;
    arc(poly, 0, 0, ro, ro, 0, p.angle);
    // cung trong đi ngược lại để đóng vùng
    const int n = segsFor(p.angle, ri);
    for (int i = n; i >= 0; --i) {
      double a = p.angle * i / n * PI / 180.0;
      pt(poly, ri * std::cos(a), ri * std::sin(a));
    }
    r.panels.push_back(makePrism("Uốn cong", "bend", 2, poly, 0, p.partHeight));
    // dán cạnh: 2 mép cung (trên/dưới) theo chiều dài cung ngoài
    r.panels.back().banding = 2 * ro * p.angle * PI / 180.0;
  } else if (k == "shelf_round") {  // Đợt bo góc
    if (p.width < 20 || p.depth < 20 || p.partThick < 1) return fail("Thông số đợt bo góc không hợp lệ");
    Poly poly;
    roundedRect(poly, 0, 0, p.width, p.depth, p.cornerFL, p.cornerFR, p.cornerBR, p.cornerBL);
    r.panels.push_back(makePrism("Đợt bo góc", "carcass", 2, poly, 0, p.partThick));
    r.panels.back().banding = p.width;  // cạnh trước
  } else if (k == "moulding") {  // Phào chỉ: biên dạng YZ đùn theo X
    if (p.length < 10 || p.sizeA < 2 || p.sizeB < 2) return fail("Thông số phào không hợp lệ");
    const double A = p.sizeA, B = p.sizeB;
    Poly poly;
    if (p.profile == "cove") {        // cung lõm
      pt(poly, 0, 0); pt(poly, A, 0);
      arc(poly, A, B, A, B, 270, 180, true);
    } else if (p.profile == "round") {  // cung lồi
      pt(poly, 0, 0); arc(poly, 0, 0, A, B, 0, 90);
    } else if (p.profile == "bevel") {
      pt(poly, 0, 0); pt(poly, A, 0); pt(poly, 0, B);
    } else if (p.profile == "step") {
      pt(poly, 0, 0); pt(poly, A, 0); pt(poly, A, B / 2); pt(poly, A / 2, B / 2); pt(poly, A / 2, B); pt(poly, 0, B);
    } else if (p.profile == "rect") {
      pt(poly, 0, 0); pt(poly, A, 0); pt(poly, A, B); pt(poly, 0, B);
    } else return fail("Kiểu phào không hợp lệ (cove|round|bevel|step|rect)");
    r.panels.push_back(makePrism("Phào " + p.profile, "moulding", 0, poly, 0, p.length));
    r.panels.back().banding = 0;
  } else if (k == "beam") {  // Xà: thanh chữ nhật
    if (p.length < 10 || p.sizeA < 2 || p.sizeB < 2) return fail("Thông số xà không hợp lệ");
    Panel a;
    a.name = "Xà"; a.role = "carcass";
    a.o[0] = a.o[1] = a.o[2] = 0;
    a.s[0] = p.length; a.s[1] = p.sizeA; a.s[2] = p.sizeB;
    a.banding = 2 * p.length;
    r.panels.push_back(a);
  } else {
    return fail("kind không hợp lệ (cabinet|bend|shelf_round|moulding|beam|board|kerf_bend|door_frame)");
  }
  for (auto& a : r.panels) finalizePanel(a);
  return r;
}

}  // namespace hsx
