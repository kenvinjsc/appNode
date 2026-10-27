// Tấm ván gia công (bo/vát góc, mộng dương/âm, khử dao, đục khung + hèm kính, phay rãnh),
// ván uốn kerf và cánh khung/kính. Toàn số học thuần; mặt ván nằm trong mặt phẳng XY, dày dọc Z.
#include "cabinet.hpp"
#include "geom.hpp"
#include <cstdio>

namespace hsx {
using namespace geom;

namespace {

bool cornerKind(const std::string& s, Kind& k) {
  if (s == "round") k = ROUND; else if (s == "concave") k = CONCAVE; else if (s == "chamfer") k = CHAMFER;
  else return false;
  return true;
}

Panel::Pocket rectPocket(double x0, double y0, double x1, double y1, bool top, double depth, const char* kind) {
  Panel::Pocket pk;
  pk.loop = {x0, y0, x1, y0, x1, y1, x0, y1};  // ngược chiều kim đồng hồ
  pk.top = top; pk.depth = depth; pk.kind = kind;
  return pk;
}

double perimeter(const Poly& p) {
  double s = 0; const size_t n = p.size() / 2;
  for (size_t i = 0; i < n; ++i) { size_t j = (i + 1) % n; s += std::hypot(p[2*j] - p[2*i], p[2*j+1] - p[2*i+1]); }
  return s;
}

// Dán cạnh tấm biên dạng: F/B = cạnh y min/max, L/R = cạnh x min/max.
void applyBoardBanding(Panel& pn, const Params& p) {
  if (!p.band) return;
  const char* letters = "LRFB";                         // thứ tự mặt thế giới -X,+X,-Y,+Y
  double len = 0;
  for (int i = 0; i < 4; ++i)
    if (p.bandEdges.find(letters[i]) != std::string::npos) { pn.band[i] = p.bandThick; len += pn.s[i < 2 ? 1 : 0]; }
  pn.banding = len;
}

}  // namespace

Result solveBoard(const Params& p) {
  Result r;
  auto fail = [&](const char* m) { r.ok = false; r.error = m; return r; };
  const double L = p.width, W = p.depth, T = p.partThick;
  if (L < 50 || W < 50 || T < 1) return fail("Kích thước tấm ván không hợp lệ");
  Kind ck;
  if (!cornerKind(p.cornerStyle, ck)) return fail("Kiểu bo góc không hợp lệ (round|concave|chamfer)");
  const double cs[4] = {p.cornerFL, p.cornerFR, p.cornerBR, p.cornerBL};
  const V2 C[4] = {{0, 0}, {L, 0}, {L, W}, {0, W}};  // FL, FR, BR, BL — ngược chiều kim đồng hồ, mặt trước y=0
  const double rdog = std::max(0.0, p.dogbone) / 2;

  const bool joint = p.jointType != "none";
  if (joint && p.jointType != "finger" && p.jointType != "dovetail" && p.jointType != "tee")
    return fail("Kiểu mộng không hợp lệ (finger|dovetail|tee)");
  if (joint && p.jointMode != "tab" && p.jointMode != "slot") return fail("jointMode phải là tab hoặc slot");

  std::vector<Vtx> vs;
  const char* letters = "FRBL";
  for (int i = 0; i < 4; ++i) {
    const double sz = std::max(0.0, std::min(cs[i], std::min(L, W) / 2));
    vs.push_back({C[i], sz > 0 ? ck : SQUARE, sz});
    if (!joint || p.jointEdges.find(letters[i]) == std::string::npos) continue;

    const V2 S = C[i], E = C[(i + 1) % 4];
    const double el = len(E - S);
    const V2 d = unit(E - S), n = {d.y, -d.x};            // n = pháp tuyến hướng ra ngoài
    const bool slot = p.jointMode == "slot";
    const double h = slot ? -std::fabs(p.jointDepth) : std::fabs(p.jointDepth);
    const double jl = p.jointLen;
    if (jl < 2 || std::fabs(h) < 1 || p.jointCount < 1) return fail("Thông số mộng không hợp lệ");
    double ex = 0;                                         // phần nở thêm mỗi bên
    if (p.jointType == "dovetail") ex = std::fabs(h) * std::tan(std::min(std::max(p.jointAngle, 0.0), 45.0) * PI / 180.0);
    if (p.jointType == "tee") ex = jl * 0.25;
    const double mS = std::max(p.jointMargin, cs[i]) + ex, mE = std::max(p.jointMargin, cs[(i + 1) % 4]) + ex;
    const int cnt = p.jointCount;
    if (el - mS - mE < jl) return fail("Cạnh quá ngắn để đặt mộng");
    const double pitch = cnt > 1 ? (el - mS - mE - jl) / (cnt - 1) : 0;
    if (cnt > 1 && pitch < jl + 2 * ex + 1) return fail("Mộng quá dày/nhiều so với chiều dài cạnh");
    auto P = [&](double t, double hh) { return S + d * t + n * hh; };
    const Kind dk = (slot && rdog > 0) ? DOGBONE : SQUARE;
    for (int k = 0; k < cnt; ++k) {
      const double a = cnt > 1 ? mS + k * pitch : (el - jl) / 2;
      auto add = [&](V2 q, bool bottom) { vs.push_back({q, bottom ? dk : SQUARE, bottom ? rdog : 0.0}); };
      if (p.jointType == "finger") {
        add(P(a, 0), false); add(P(a, h), true); add(P(a + jl, h), true); add(P(a + jl, 0), false);
      } else if (p.jointType == "dovetail") {
        add(P(a, 0), false); add(P(a - ex, h), true); add(P(a + jl + ex, h), true); add(P(a + jl, 0), false);
      } else {  // tee
        const double hd = ex;
        add(P(a, 0), false); add(P(a, h / 2), true); add(P(a - hd, h / 2), true); add(P(a - hd, h), true);
        add(P(a + jl + hd, h), true); add(P(a + jl + hd, h / 2), true); add(P(a + jl, h / 2), true); add(P(a + jl, 0), false);
      }
    }
  }

  Poly outline = build(vs);
  if (outline.size() < 6 || area(outline) <= 0) return fail("Biên dạng tấm không hợp lệ");
  Panel pn = makePrism("Tấm ván", "carcass", 2, outline, 0, T);
  pn.banding = perimeter(outline);

  // đục khung (+ hèm kính)
  if (p.frameBorder > 0) {
    const double b = p.frameBorder;
    if (L - 2 * b < 20 || W - 2 * b < 20) return fail("Khung quá rộng so với tấm ván");
    const bool rabbet = p.rabbetW > 0 && p.rabbetD > 0;
    if (rabbet && (p.rabbetD >= T - 1 || b - p.rabbetW < 5)) return fail("Hèm kính quá sâu/rộng");
    Kind fk = SQUARE; double fs = 0;
    if (p.frameStyle == "round") { fk = ROUND; fs = p.frameR; }
    else if (p.frameStyle == "chamfer") { fk = CHAMFER; fs = p.frameR; }
    else if (p.frameStyle == "square") { if (rdog > 0) { fk = DOGBONE; fs = rdog; } }
    else return fail("Kiểu góc khung không hợp lệ (square|round|chamfer)");
    auto holeLoop = [&](double m, double extra) {  // vòng thuận kim đồng hồ (lỗ)
      std::vector<Vtx> hv = {{{m, m}, fk, fs + extra}, {{m, W - m}, fk, fs + extra}, {{L - m, W - m}, fk, fs + extra}, {{L - m, m}, fk, fs + extra}};
      return build(hv);
    };
    Panel::Pocket hole; hole.loop = holeLoop(b, 0); hole.top = true; hole.kind = "frame";
    hole.depth = rabbet ? T - p.rabbetD : T;
    pn.pockets.push_back(hole);
    if (rabbet) {
      Panel::Pocket rb; rb.loop = holeLoop(b - p.rabbetW, fk == ROUND || fk == CHAMFER ? p.rabbetW : 0);
      rb.top = false; rb.depth = p.rabbetD; rb.kind = "rabbet";
      pn.pockets.push_back(rb);
    }
  }

  // phay rãnh
  if (p.grooveCount > 0) {
    const double gw = p.grooveW, gd = p.grooveDepth;
    if (gw < 1 || gd < 0.5 || gd >= T) return fail("Rãnh không hợp lệ (sâu phải nhỏ hơn dày ván)");
    if (p.grooveFace != "bottom" && p.grooveFace != "top") return fail("grooveFace phải là bottom hoặc top");
    const int n = p.grooveCount;
    double pitch = p.grooveSpacing > 0 ? gw + p.grooveSpacing : (n > 1 ? (W - 2 * p.grooveOffset - gw) / (n - 1) : 0);
    if (n > 1 && pitch < gw + 1) return fail("Các rãnh chồng nhau");
    const double xa = std::max(0.0, p.grooveInset), xb = L - std::max(0.0, p.grooveInset);
    if (xb - xa < 5) return fail("Rãnh quá ngắn");
    for (int i = 0; i < n; ++i) {
      const double y0 = p.grooveOffset + i * pitch;
      if (y0 < 0 || y0 + gw > W) return fail("Rãnh nằm ngoài tấm ván");
      pn.pockets.push_back(rectPocket(xa, y0, xb, y0 + gw, p.grooveFace == "top", gd, "groove"));
    }
  }
  applyBoardBanding(pn, p);
  r.panels.push_back(pn);
  finalizePanel(r.panels.back());
  return r;
}

Result solveKerf(const Params& p) {
  Result r;
  auto fail = [&](const char* m) { r.ok = false; r.error = m; return r; };
  const double L = p.width, W = p.depth, T = p.partThick;
  if (L < 50 || W < 50 || T < 3) return fail("Kích thước ván uốn không hợp lệ");
  if (p.kerfW < 0.5 || p.kerfRemain < 0.5 || p.kerfRemain >= T - 0.5) return fail("Kerf/độ dày còn lại không hợp lệ");
  const double cutDepth = T - p.kerfRemain;
  // Mỗi rãnh khép lại một góc ≈ kerfW / cutDepth -> bước rãnh trên cung bán kính R: s = R·kerfW / cutDepth
  const double s = p.bendRadius * p.kerfW / cutDepth;
  if (s < p.kerfW * 1.5) return fail("Bán kính uốn quá nhỏ so với độ dày: rãnh chồng nhau");
  const double zone = p.kerfZone > 0 ? p.kerfZone : W - 2 * p.kerfStart;
  if (p.kerfStart < 0 || p.kerfStart + zone > W + 1e-9 || zone < p.kerfW) return fail("Vùng uốn nằm ngoài tấm ván");
  const int n = (int)std::floor((zone - p.kerfW) / s) + 1;
  Panel pn = makePrism("Ván xẻ rãnh uốn", "carcass", 2, {0, 0, L, 0, L, W, 0, W}, 0, T);
  applyBoardBanding(pn, p);
  for (int i = 0; i < n; ++i) {
    const double y0 = p.kerfStart + i * s;
    pn.pockets.push_back(rectPocket(0, y0, L, y0 + p.kerfW, false, cutDepth, "kerf"));
  }
  char buf[160];
  std::snprintf(buf, sizeof buf, "%d rãnh, bước %.2f mm, sâu %.2f mm (còn %.2f mm)", n, s, cutDepth, p.kerfRemain);
  r.info = buf;
  r.panels.push_back(pn);
  finalizePanel(r.panels.back());
  return r;
}

Result solveDoor(const Params& p) {
  Result r;
  auto fail = [&](const char* m) { r.ok = false; r.error = m; return r; };
  const double W = p.width, H = p.height, st = p.stile, rl = p.rail, ft = p.frameThick, pt = p.panelThick;
  if (W < 100 || H < 100 || ft < 5) return fail("Kích thước cánh không hợp lệ");
  if (st < 10 || rl < 10 || 2 * st > W - 40 || 2 * rl > H - 40) return fail("Đố/thanh ngang quá to so với cánh");
  if (p.panelType != "glass" && p.panelType != "flat" && p.panelType != "none") return fail("panelType phải là glass|flat|none");
  auto box = [&](const char* name, const char* role, double x, double y, double z, double sx, double sy, double sz) {
    Panel a; a.name = name; a.role = role;
    a.o[0] = x; a.o[1] = y; a.o[2] = z; a.s[0] = sx; a.s[1] = sy; a.s[2] = sz;
    a.banding = 2 * (sx + sz);
    r.panels.push_back(a);
  };
  box("Đố trái", "front", 0, 0, 0, st, ft, H);
  box("Đố phải", "front", W - st, 0, 0, st, ft, H);
  box("Thanh dưới", "front", st, 0, 0, W - 2 * st, ft, rl);
  box("Thanh trên", "front", st, 0, H - rl, W - 2 * st, ft, rl);
  if (p.panelType != "none") {
    if (pt < 1 || pt > ft) return fail("Độ dày tấm/kính không hợp lệ");
    box(p.panelType == "glass" ? "Kính" : "Tấm cánh", p.panelType == "glass" ? "glass" : "back",
        st, (ft - pt) / 2, rl, W - 2 * st, pt, H - 2 * rl);
    r.panels.back().banding = 0;
  }
  for (auto& a : r.panels) finalizePanel(a);
  return r;
}

}  // namespace hsx
