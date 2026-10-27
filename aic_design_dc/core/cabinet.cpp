#include "cabinet.hpp"
#include <algorithm>
#include <cmath>

namespace hsx {
namespace {

struct Builder {
  const Params& p;
  Result& r;
  Builder(const Params& pp, Result& rr) : p(pp), r(rr) {}

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
  double cover(bool outer) const {
    switch (p.door) {
      case DoorStyle::Overlay: return outer ? p.thick : p.thick / 2;
      case DoorStyle::Half:    return outer ? p.thick / 2 : p.thick / 4;
      default:                 return 0;
    }
  }
  double gapShare(bool outer) const {
    if (!outer) return p.gap / 2;
    return p.door == DoorStyle::Inset ? p.gap : 0;
  }
  double ext(bool outer) const { return cover(outer) - gapShare(outer); }  // lấn ra khỏi mép khoang

  double frontY() const {  // mặt sau của cánh
    return p.door == DoorStyle::Inset ? p.frontReturn : -p.doorThick;
  }

  static int hingesFor(double h) {
    if (h <= 900) return 2;
    if (h <= 1600) return 3;
    if (h <= 2000) return 4;
    if (h <= 2400) return 5;
    return 6;
  }

  void run() {
    const double t = p.thick, W = p.width, D = p.depth, H = p.height, bt = p.backThick;
    if (W < 4 * t || H < p.plinth + 4 * t || D < 150) { r.ok = false; r.error = "Kích thước khung quá nhỏ"; return; }
    std::vector<Column> cols = p.cols;
    if (cols.empty()) cols.push_back(Column{});
    const int n = (int)cols.size();

    double sideDepth = D, innerDepth = D;
    if (p.hasBack) {
      if (p.back == BackStyle::Overlay) sideDepth = innerDepth = D - bt;
      else innerDepth = D - bt - 8;
    }
    const double Wi = W - 2 * t;
    const double zb = p.plinth + t, zt = H - t;

    // chia bề rộng khoang
    double fixedSum = 0; int autoN = 0;
    for (auto& c : cols) { if (c.width > 0) fixedSum += c.width; else ++autoN; }
    double autoW = autoN ? (Wi - (n - 1) * t - fixedSum) / autoN : 0;
    if ((autoN && autoW < 50) || (!autoN && std::fabs(Wi - (n - 1) * t - fixedSum) > 0.5)) {
      r.ok = false; r.error = "Bề rộng các khoang không khớp với bề rộng tủ"; return;
    }

    // thân
    int sideL = add("Hông trái", "carcass", 0, 0, 0, t, sideDepth, H, H);
    int sideR = add("Hông phải", "carcass", W - t, 0, 0, t, sideDepth, H, H);
    int bottom = add("Đáy", "carcass", t, 0, p.plinth, Wi, innerDepth, t, Wi);
    int top = add("Nóc", "carcass", t, 0, zt, Wi, innerDepth, t, Wi);
    if (p.hasBack) {  // hậu rộng hơn khổ ván -> tách các tấm bằng nhau
      const bool ov = p.back == BackStyle::Overlay;
      const double bx = ov ? 0 : t, bw = ov ? W : Wi, by = ov ? D - bt : innerDepth;
      const double bz = ov ? p.plinth : zb, bh = ov ? H - p.plinth : zt - zb;
      int pieces = p.sheetW > 0 ? std::max(1, (int)std::ceil(bw / p.sheetW - 1e-9)) : 1;
      for (int k = 0; k < pieces; ++k)
        add(pieces > 1 ? "Hậu " + std::to_string(k + 1) : "Hậu", "back", bx + k * bw / pieces, by, bz, bw / pieces, bt, bh, 0);
    }
    const double loftZ = (p.loftHeight > 0) ? zt - p.loftHeight - t : -1;  // đáy ván ngăn hộc trên
    if (loftZ > 0 && loftZ < zb + 200) { r.ok = false; r.error = "Hộc trên quá cao"; return; }
    const double mainTop = (loftZ > 0) ? loftZ : zt;  // mặt dưới vùng khoang chính

    // vách đứng + x các khoang
    std::vector<double> x0(n), w(n);
    std::vector<int> divs(n + 1, -1);   // divs[i] = vách bên trái khoang i; [0]=sideL, [n]=sideR
    divs[0] = sideL; divs[n] = sideR;
    double x = t;
    for (int i = 0; i < n; ++i) {
      w[i] = cols[i].width > 0 ? cols[i].width : autoW;
      x0[i] = x;
      x += w[i];
      if (i < n - 1) {
        divs[i + 1] = add("Vách " + std::to_string(i + 1), "carcass", x, 0, zb, t, innerDepth, zt - zb, zt - zb);
        x += t;
      }
    }
    // cam cho từng ô ngang theo khoang
    auto camSpan = [&](int hp, int i) {
      if (!p.cam) return;
      // ván ngang đã trải toàn bề rộng; thêm cam tại 2 vách bao quanh khoang i
      Panel& h = r.panels[hp]; double zc = h.o[2] + h.s[2] / 2;
      for (double y : {37.0, innerDepth - 37}) {
        hole(hp, x0[i] + 34, y, zc, 2, 15, 12.5, "cam");
        hole(hp, x0[i] + w[i] - 34, y, zc, 2, 15, 12.5, "cam");
        hole(divs[i], x0[i], y, zc, 0, 5, 10, "bolt");
        hole(divs[i + 1], x0[i] + w[i], y, zc, 0, 5, 10, "bolt");
      }
    };
    for (int i = 0; i < n; ++i) {
      camSpan(bottom, i); camSpan(top, i);
      if (loftZ > 0) camSpan(add("K" + std::to_string(i + 1) + " Đợt hộc trên", "carcass", x0[i], 0, loftZ, w[i], innerDepth, t, w[i]), i);
    }

    const double outerBot = p.plinth, outerTop = H;
    for (int i = 0; i < n; ++i) {
      const Column& c = cols[i];
      const std::string tag = "K" + std::to_string(i + 1);
      const double dh = c.drawerPitch > 0 ? c.drawerPitch : 180;
      const double stack = c.drawers * dh;
      double doorLo = zb;        // mép dưới khoang cánh (mặt trên của ván/đáy)
      bool doorLoOuter = true;

      // trái/phải khoang có phải mép ngoài thân tủ?
      const bool lOuter = (i == 0), rOuter = (i == n - 1);
      const double fx0 = x0[i] - ext(lOuter), fx1 = x0[i] + w[i] + ext(rOuter);

      auto zLeafLo = [&](double openLo, bool outer) { return openLo - ext(outer); };
      auto zLeafHi = [&](double openHi, bool outer) { return openHi + ext(outer); };

      // ngăn kéo
      if (c.drawers > 0) {
        double capZ = zb + stack;
        int cap = add(tag + " Đợt ngăn kéo", "carcass", x0[i], 0, capZ, w[i], innerDepth, t, w[i]);
        (void)cap;
        double fLo = zLeafLo(zb, true), fHi = zLeafHi(capZ, false);
        double fh = (fHi - fLo - (c.drawers - 1) * p.gap) / c.drawers;
        const double sl = 12.7, dt = 15;
        double y0 = (p.door == DoorStyle::Inset ? p.frontReturn + p.doorThick : 0) + 2;
        double depthAvail = innerDepth - y0 - 20;
        double bd = std::floor(depthAvail / 50.0) * 50.0;
        double bw = w[i] - 2 * sl, bx = x0[i] + sl;
        for (int d = 0; d < c.drawers; ++d) {
          double fz = fLo + d * (fh + p.gap);
          add(tag + " Mặt hộc kéo " + std::to_string(d + 1), "front", fx0, frontY(), fz, fx1 - fx0, p.doorThick, fh,
              2 * (fx1 - fx0) + 2 * fh);
          double bh = dh - 30, bz = zb + d * dh + 10;
          std::string dn = tag + " Hộc kéo " + std::to_string(d + 1);
          add(dn + " hông T", "drawer", bx, y0, bz, dt, bd, bh, bd);
          add(dn + " hông P", "drawer", bx + bw - dt, y0, bz, dt, bd, bh, bd);
          add(dn + " hậu", "drawer", bx + dt, y0 + bd - dt, bz, bw - 2 * dt, dt, bh, 0);
          add(dn + " đáy", "drawer", bx + dt, y0 + 0, bz + 8, bw - 2 * dt, bd - dt, bt, 0);
        }
        doorLo = capZ + t; doorLoOuter = false;
      }

      // đợt ngang chia đều
      if (c.shelves > 0) {
        double lo = (c.drawers > 0) ? doorLo : zb, hi = mainTop;
        double space = hi - lo - c.shelves * t;
        double step = space / (c.shelves + 1);
        for (int s = 0; s < c.shelves; ++s) {
          double z = lo + step * (s + 1) + s * t;
          int sp = add(tag + " Đợt " + std::to_string(s + 1), "carcass", x0[i], 0, z, w[i], innerDepth, t, w[i]);
          if (p.cam) for (double y : {37.0, innerDepth - 37}) {
            hole(sp, x0[i] + 34, y, z + t / 2, 2, 15, 12.5, "cam");
            hole(sp, x0[i] + w[i] - 34, y, z + t / 2, 2, 15, 12.5, "cam");
            hole(divs[i], x0[i], y, z + t / 2, 0, 5, 10, "bolt");
            hole(divs[i + 1], x0[i] + w[i], y, z + t / 2, 0, 5, 10, "bolt");
          }
        }
      }

      // cánh khoang chính
      auto doors = [&](int count, double openLo, bool loOuter, double openHi, bool hiOuter, const std::string& zone) {
        if (count <= 0) return;
        double fLo = zLeafLo(openLo, loOuter), fHi = zLeafHi(openHi, hiOuter);
        double fh = fHi - fLo;
        double lw = ((fx1 - fx0) - (count - 1) * p.gap) / count;
        int hn = hingesFor(fh);
        for (int k = 0; k < count; ++k) {
          double lx = fx0 + k * (lw + p.gap);
          bool left = (k == 0) || (k % 2 == 0 && k != count - 1);
          if (count > 1 && k == count - 1) left = false;
          int id = add(tag + " Cánh " + zone + std::to_string(k + 1), "front", lx, frontY(), fLo, lw, p.doorThick, fh,
                       2 * lw + 2 * fh);
          // bản lề: lỗ chén Ø35 cách mép 22.5, cách đầu 100
          double hx = left ? lx + 22.5 : lx + lw - 22.5;
          int plate = left ? divs[i] : divs[i + 1];
          double px = left ? x0[i] : x0[i] + w[i];
          for (int h = 0; h < hn; ++h) {
            double z = fLo + 100 + h * (fh - 200) / (hn - 1);
            hole(id, hx, frontY() + p.doorThick / 2, z, 1, 35, 12.5, "hinge_cup");
            for (double dy : {37.0, 69.0}) hole(plate, px, dy, z, 0, 5, 12, "hinge_plate");
            ++r.hingeCount;
          }
        }
      };
      doors(c.doors, doorLo, doorLoOuter, mainTop, loftZ < 0, "");  // nếu có hộc trên: mép trên là ván ngăn
      if (loftZ > 0 && p.loftDoors > 0)
        doors(p.loftDoors, loftZ + t, false, zt, true, "trên ");
    }
    for (auto& a : r.panels) finalizePanel(a);
  }
};

}  // namespace

Result solve(const Params& p) {
  if (p.kind != "cabinet") return solvePart(p);
  Result r;
  Builder b(p, r);
  b.run();
  if (!r.ok) r.panels.clear();
  return r;
}

}  // namespace hsx
