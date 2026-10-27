#include "cabinet.hpp"
#include "layout.hpp"
#include <algorithm>
#include <cmath>
#include <map>

namespace hsx {
namespace {

struct Builder {
  const Params& p;
  Result& r;
  Builder(const Params& pp, Result& rr) : p(pp), r(rr) {}

  // --- hằng số của tủ đang dựng ---
  double t = 0, W = 0, D = 0, H = 0, bt = 0, Wi = 0, zb = 0, zt = 0, innerDepth = 0;
  struct Plate { double x, z; };        // bản lề cần lỗ đế trên vách tương ứng (gán sau khi có đủ tấm)
  std::vector<Plate> plates;
  std::string failMsg;
  int slides = 0; double slideLenUsed = 0;   // ray âm đã dùng (ghi vào info)

  int add(const std::string& name, const char* role, double x, double y, double z,
          double sx, double sy, double sz, double banding) {
    Panel a;
    a.name = name; a.role = role;
    a.o[0] = x; a.o[1] = y; a.o[2] = z;
    a.s[0] = sx; a.s[1] = sy; a.s[2] = sz;
    a.banding = banding;
    r.panels.push_back(a);
    return (int)r.panels.size() - 1;
  }
  void hole(int idx, double x, double y, double z, int axis, double dia, double depth, const char* kind) {
    Hole h{{x, y, z}, axis, dia, depth, kind};
    r.panels[idx].holes.push_back(h);
  }

  // Phần chồng của cánh lên ván: outer = mép ngoài thân tủ, inner = vách dùng chung
  static double cover(DoorStyle ds, double th, bool outer) {
    switch (ds) {
      case DoorStyle::Overlay: return outer ? th : th / 2;
      case DoorStyle::Half:    return outer ? th / 2 : th / 4;
      default:                 return 0;
    }
  }
  static double gapShare(DoorStyle ds, double gap, bool outer) {
    if (!outer) return gap / 2;
    return ds == DoorStyle::Inset ? gap : 0;
  }
  double ext(DoorStyle ds, bool outer) const { return cover(ds, t, outer) - gapShare(ds, p.gap, outer); }
  double frontY(DoorStyle ds) const { return ds == DoorStyle::Inset ? p.frontReturn : -p.doorThick; }

  static int hingesFor(double h) {
    if (h <= 900) return 2;
    if (h <= 1600) return 3;
    if (h <= 2000) return 4;
    if (h <= 2400) return 5;
    return 6;
  }

  static bool near(double a, double b) { return std::fabs(a - b) < 0.01; }
  static std::string tagOf(const std::vector<int>& path) {
    if (path.empty()) return "K1";
    std::string s = "K";
    for (size_t i = 0; i < path.size(); ++i) s += (i ? "." : "") + std::to_string(path[i] + 1);
    return s;
  }

  // Chia kích thước các khoang con: size>0 cố định, 0 = tự chia đều phần còn lại.
  // Khi không còn khoang tự do mà tổng không khớp thì co/giãn tỉ lệ (kéo giãn tủ vẫn hợp lệ).
  bool distribute(const LNode& nd, double total, std::vector<double>& out) {
    const int n = (int)nd.kids.size();
    const double avail = total - (n - 1) * t;
    double fixedSum = 0; int autoN = 0;
    for (double s : nd.size) { if (s > 0) fixedSum += s; else ++autoN; }
    out.assign(n, 0);
    if (autoN == 0) {
      if (avail < 50.0 * n) { failMsg = "Khoang quá hẹp để chia"; return false; }
      const double k = avail / fixedSum;
      for (int i = 0; i < n; ++i) out[i] = nd.size[i] * k;
    } else {
      const double aw = (avail - fixedSum) / autoN;
      if (aw < 50) { failMsg = "Các khoang chia vượt quá kích thước còn lại (khoang nhỏ hơn 50 mm)"; return false; }
      for (int i = 0; i < n; ++i) out[i] = nd.size[i] > 0 ? nd.size[i] : aw;
    }
    return true;
  }

  void layoutNode(const LNode& nd, double x0, double x1, double z0, double z1, std::vector<int> path) {
    if (!failMsg.empty()) return;
    if (!nd.split) { leaf(nd.leaf, x0, x1, z0, z1, path); return; }
    const bool vert = nd.dir == 'V';
    std::vector<double> sz;
    if (!distribute(nd, vert ? x1 - x0 : z1 - z0, sz)) return;
    double pos = vert ? x0 : z0;
    for (size_t i = 0; i < nd.kids.size(); ++i) {
      auto kp = path; kp.push_back((int)i);
      if (vert) layoutNode(nd.kids[i], pos, pos + sz[i], z0, z1, kp);
      else      layoutNode(nd.kids[i], x0, x1, pos, pos + sz[i], kp);
      pos += sz[i];
      if (i + 1 < nd.kids.size()) {                       // ván chia giữa hai khoang
        const std::string where = path.empty() ? "" : tagOf(path) + "-";
        if (vert) add("Vách " + where + std::to_string(i + 1), "carcass", pos, 0, z0, t, innerDepth, z1 - z0, z1 - z0);
        else      add("Đợt " + (path.empty() ? "" : tagOf(path) + "-") + std::to_string(i + 1), "carcass", x0, 0, pos, x1 - x0, innerDepth, t, x1 - x0);
        pos += t;
      }
    }
  }

  // Nội dung một khoang lá: ngăn kéo (từ đáy khoang lên), đợt chia đều, cánh phủ phần còn lại.
  void leaf(const Column& c, double x0, double x1, double z0, double z1, const std::vector<int>& path) {
    const std::string tag = tagOf(path);
    const DoorStyle ds = c.doorStyle >= 0 && c.doorStyle <= 2 ? (DoorStyle)c.doorStyle : p.door;
    const bool lOuter = near(x0, t), rOuter = near(x1, W - t), bOuter = near(z0, zb), tOuter = near(z1, zt);
    const double w = x1 - x0;
    const double fx0 = x0 - ext(ds, lOuter), fx1 = x1 + ext(ds, rOuter);
    const double dh = c.drawerPitch > 0 ? c.drawerPitch : 180;
    double doorLo = z0;                 // mép dưới vùng cánh
    bool doorLoOuter = bOuter;

    Opening op; op.tag = tag; op.path = path; op.x0 = x0; op.x1 = x1; op.z0 = z0; op.z1 = z1;
    op.drawers = c.drawers; op.shelves = c.shelves; op.doors = c.doors;
    r.openings.push_back(op);

    if (c.drawers > 0) {
      const double capZ = z0 + c.drawers * dh;
      if (capZ + t > z1 + 1e-6) { failMsg = "Ngăn kéo vượt quá chiều cao khoang " + tag; return; }
      add(tag + " Đợt ngăn kéo", "carcass", x0, 0, capZ, w, innerDepth, t, w);
      const double fLo = z0 - ext(ds, bOuter), fHi = capZ + ext(ds, false);
      const double fh = (fHi - fLo - (c.drawers - 1) * p.gap) / c.drawers;
      const bool under = p.drawerSlide == "under";
      const double sl = under ? 21 : 12.7, dt = 15;
      const double y0 = (ds == DoorStyle::Inset ? p.frontReturn + p.doorThick : 0) + 2;
      double bd = std::floor((innerDepth - y0 - 20) / 50.0) * 50.0;
      if (under) {
        bd = std::min(bd, std::floor(p.slideLen / 50.0) * 50.0);
        if (bd < 250) { failMsg = "Khoang " + tag + " quá nông cho ray âm (tối thiểu 250 mm)"; return; }
        slides += c.drawers; slideLenUsed = bd;
      }
      const double bw = w - 2 * sl, bx = x0 + sl;
      for (int d = 0; d < c.drawers; ++d) {
        const double fz = fLo + d * (fh + p.gap);
        add(tag + " Mặt hộc kéo " + std::to_string(d + 1), "front", fx0, frontY(ds), fz, fx1 - fx0, p.doorThick, fh, 2 * (fx1 - fx0) + 2 * fh);
        const double bh = dh - 30, bz = z0 + d * dh + 10;
        const std::string dn = tag + " Hộc kéo " + std::to_string(d + 1);
        add(dn + " hông T", "drawer", bx, y0, bz, dt, bd, bh, bd);
        add(dn + " hông P", "drawer", bx + bw - dt, y0, bz, dt, bd, bh, bd);
        add(dn + " hậu", "drawer", bx + dt, y0 + bd - dt, bz, bw - 2 * dt, dt, bh, 0);
        add(dn + " đáy", "drawer", bx + dt, y0, bz + 8, bw - 2 * dt, bd - dt, bt, 0);
      }
      doorLo = capZ + t; doorLoOuter = false;
    }

    if (c.shelves > 0) {
      const double lo = (c.drawers > 0) ? doorLo : z0;
      const double step = (z1 - lo - c.shelves * t) / (c.shelves + 1);
      if (step < 20) { failMsg = "Quá nhiều đợt trong khoang " + tag; return; }
      for (int s = 0; s < c.shelves; ++s)
        add(tag + " Đợt " + std::to_string(s + 1), "carcass", x0, 0, lo + step * (s + 1) + s * t, w, innerDepth, t, w);
    }

    if (c.fixH > 0) {   // tấm fix che phía dưới khoang (vd. dưới hộp hút mùi); cánh bắt đầu phía trên
      if (doorLo + c.fixH + 100 > z1) { failMsg = "Tấm fix quá cao trong khoang " + tag; return; }
      add(tag + " Tấm fix", "carcass", x0, 0, doorLo, w, t, c.fixH, w);
      if (c.doorGap <= 0) { doorLo += c.fixH; doorLoOuter = false; }   // có doorGap thì cánh phủ lên tấm fix
    }
    if (c.ductW > 0) {   // ống hút mùi: 2 tấm dọc + tấm fix hậu, nằm giữa khoang sát hậu
      const double dw = c.ductW, dd = 202.0;
      if (dw < 100 || dw > w - 40 || dd > innerDepth - 100) { failMsg = "Ống hút mùi không vừa khoang " + tag; return; }
      const double cx = (x0 + x1) / 2, yb = innerDepth - dd, h = z1 - z0;
      add(tag + " Ống dọc trái", "carcass", cx - dw / 2, yb, z0, t, dd, h, h);
      add(tag + " Ống dọc phải", "carcass", cx + dw / 2 - t, yb, z0, t, dd, h, h);
      add(tag + " Ống tấm fix hậu", "carcass", cx - dw / 2 + t, yb - t, z0, dw - 2 * t, t, h, dw - 2 * t);
    }

    if (c.doorGap > 0) { doorLo = z0 + c.doorGap; doorLoOuter = false; }

    int count = c.doors;
    if (count < 0) count = std::max(1, (int)std::ceil((fx1 - fx0) / std::max(100.0, p.doorMaxW) - 1e-9));   // tự theo rộng tối đa một cánh
    if (count > 0) {
      const double fLo = doorLo - ext(ds, doorLoOuter), fHi = z1 + ext(ds, tOuter);
      const double fh = fHi - fLo;
      if (fh < 150) { failMsg = "Khoang " + tag + " quá thấp để đặt cánh"; return; }
      const double lw = ((fx1 - fx0) - (count - 1) * p.gap) / count;
      if (lw < 80) { failMsg = "Cánh quá hẹp trong khoang " + tag; return; }
      const int hn = c.hinges >= 2 ? std::min(c.hinges, 8) : hingesFor(fh);
      for (int k = 0; k < count; ++k) {
        const double lx = fx0 + k * (lw + p.gap);
        bool left = (k == 0) || (k % 2 == 0 && k != count - 1);
        if (count > 1 && k == count - 1) left = false;
        if (count == 1 && c.hingeSide == 2) left = false;
        const int id = add(tag + " Cánh " + std::to_string(k + 1), "front", lx, frontY(ds), fLo, lw, p.doorThick, fh, 2 * lw + 2 * fh);
        const double hx = left ? lx + 22.5 : lx + lw - 22.5;          // lỗ chén Ø35 cách mép 22.5, cách đầu 100
        const double px = left ? x0 : x1;
        for (int h = 0; h < hn; ++h) {
          const double z = fLo + 100 + h * (fh - 200) / (hn - 1);
          hole(id, hx, frontY(ds) + p.doorThick / 2, z, 1, 35, 12.5, "hinge_cup");
          plates.push_back({px, z});
          ++r.hingeCount;
        }
      }
    }
  }

  void run() {
    t = p.thick; W = p.width; D = p.depth; H = p.height; bt = p.backThick;
    if (W < 4 * t || H < p.plinth + 4 * t || D < 150) { r.ok = false; r.error = "Kích thước khung quá nhỏ"; return; }
    LNode root;
    if (p.layout.empty()) root = layoutFromColumns(p);
    else { std::string e; if (!parseLayout(p.layout, root, e)) { r.ok = false; r.error = e; return; } }

    double sideDepth = D; innerDepth = D;
    if (p.hasBack) {
      if (p.back == BackStyle::Overlay) sideDepth = innerDepth = D - bt;
      else if (p.back == BackStyle::Flush) innerDepth = D - bt;       // hậu lọt: hông giữ nguyên sâu D, hậu nằm sát mép sau giữa hai hông
      else innerDepth = D - bt - 8;
    }
    Wi = W - 2 * t; zb = p.plinth + t; zt = H - t;

    // khung: hông, đáy, nóc, hậu
    const double sz0 = p.sideOnPlinth ? p.plinth : 0, sh = H - sz0;
    add("Hông trái", "carcass", 0, 0, sz0, t, sideDepth, sh, sh);
    add("Hông phải", "carcass", W - t, 0, sz0, t, sideDepth, sh, sh);
    if (p.plinthFront && p.plinth > 1) {
      const double px = p.sideOnPlinth ? 0 : t, pw = p.sideOnPlinth ? W : Wi;
      const double sb = std::max(0.0, std::min(p.plinthSetback, innerDepth - 2 * t));
      add("Chân trước", "plinth", px, sb, 0, pw, t, p.plinth, 0);
    }
    add("Đáy", "carcass", t, 0, p.plinth, Wi, innerDepth, t, Wi);
    if (p.cabType == "base") {   // tủ dưới: không nóc, thanh giằng ngang thay thế
      const int n = std::max(2, std::min(p.railCount, 6));
      const double rw = std::max(20.0, std::min(p.railW, innerDepth / 2));
      for (int i = 0; i < n; ++i) {
        const std::string nm = i == 0 ? "Thanh giằng trước" : i == n - 1 ? "Thanh giằng sau" : "Thanh giằng " + std::to_string(i + 1);
        add(nm, "carcass", t, i * (innerDepth - rw) / (n - 1), zt, Wi, rw, t, Wi);
      }
    } else add("Nóc", "carcass", t, 0, zt, Wi, innerDepth, t, Wi);
    if (p.hasBack) {  // hậu rộng hơn khổ ván -> tách các tấm bằng nhau
      const bool ov = p.back == BackStyle::Overlay, fl = p.back == BackStyle::Flush;
      const double bx = ov ? 0 : t, bw = ov ? W : Wi, by = ov ? D - bt : fl ? D - bt : innerDepth;
      const double bz = (ov || fl) ? p.plinth : zb, bh = (ov || fl) ? H - p.plinth : zt - zb;
      const int pieces = p.sheetW > 0 ? std::max(1, (int)std::ceil(bw / p.sheetW - 1e-9)) : 1;
      for (int k = 0; k < pieces; ++k)
        add(pieces > 1 ? "Hậu " + std::to_string(k + 1) : "Hậu", "back", bx + k * bw / pieces, by, bz, bw / pieces, bt, bh, 0);
    }

    if (p.topCover > 0) {   // tấm đỉnh phủ liền mặt trên thân tủ (đặt lên trên, nhô ra trước)
      const double oh = std::max(0.0, p.topCoverOverhang);
      add("Đỉnh", "carcass", 0, -oh, H, W, sideDepth + oh, p.topCover, W);
    }
    for (const char ch : p.endPanel) {   // tấm ốp hông ngoài thân tủ (vd. cuối dãy tủ dưới), cao đến dưới mặt hông 10 mm
      if (ch != 'L' && ch != 'R') continue;
      const double ew = std::max(20.0, std::min(p.endPanelW, innerDepth));
      add(ch == 'L' ? "Tấm ốp hông trái" : "Tấm ốp hông phải", "carcass", ch == 'L' ? -t : W, 0, sz0, t, ew, sh - 10, sh - 10);
    }
    layoutNode(root, t, W - t, zb, zt, {});
    if (!failMsg.empty()) { r.ok = false; r.error = failMsg; return; }
    for (auto& a : r.panels) finalizePanel(a);

    // lỗ đế bản lề: tìm vách đứng có mặt nằm tại mép khoang
    for (const Plate& pl : plates) {
      for (auto& P : r.panels) {
        if (P.role != "carcass" || P.prismAxis >= 0 || P.tAxis != 0) continue;
        if (!(near(P.o[0], pl.x) || near(P.o[0] + P.s[0], pl.x))) continue;
        if (pl.z < P.o[2] - 0.01 || pl.z > P.o[2] + P.s[2] + 0.01) continue;
        for (double dy : {37.0, 69.0}) {
          Hole h{{pl.x, dy, pl.z}, 0, 5, 12, "hinge_plate"};
          P.holes.push_back(h);
        }
        break;
      }
    }
    if (p.hangZ > 0) {   // tủ treo: nâng cả tủ (tấm, lỗ, khoang) trước khi dò mối nối/dán cạnh để đáy không bị coi là chạm sàn
      for (auto& P : r.panels) { P.o[2] += p.hangZ; for (auto& h : P.holes) h.p[2] += p.hangZ; }
      for (auto& o : r.openings) { o.z0 += p.hangZ; o.z1 += p.hangZ; }
    }
    if (slides > 0) r.info += "Ray âm " + std::to_string((int)slideLenUsed) + " x " + std::to_string(slides) + " bộ; ";
    if (p.cam) addJoinery(r, p);
    if (p.band) applyBanding(r, p);
  }
};

}  // namespace

// ID ổn định = tên cấu trúc; trùng tên thì thêm hậu tố #n theo thứ tự sinh (xác định, không phụ thuộc kích thước).
void assignIds(Result& r) {
  std::map<std::string, int> seen;
  for (auto& a : r.panels) {
    const int n = seen[a.name]++;
    a.id = n ? a.name + "#" + std::to_string(n + 1) : a.name;
  }
}

Result solve(const Params& p) {
  if (p.kind != "cabinet") { Result q = solvePart(p); assignIds(q); return q; }
  Result r;
  Builder b(p, r);
  b.run();
  if (!r.ok) r.panels.clear();
  assignIds(r);
  return r;
}

}  // namespace hsx
