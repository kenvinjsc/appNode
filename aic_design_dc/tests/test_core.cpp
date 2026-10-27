#include "../core/cabinet.hpp"
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
  for (auto& pn : r.panels) for (auto& h : pn.holes) CHECK(std::string(h.kind) != "cam" && std::string(h.kind) != "bolt");
  { auto pc = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay); pc.cam = true; CHECK(hsx::solve(pc).panels.size() == r.panels.size()); }
  CHECK(hsx::exportDxf(r, "/tmp/hsx_test.dxf"));
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
    auto q = polyOk(s, "shelf_round"); CHECK(hsx::exportDxf(q, "/tmp/hsx_part.dxf")); }
  for (const char* pf : {"cove", "round", "bevel", "step", "rect"}) {
    hsx::Params m; m.kind = "moulding"; m.profile = pf; m.length = 2000; polyOk(m, pf);
  }
  { hsx::Params m; m.kind = "moulding"; m.profile = "zzz"; CHECK(!hsx::solve(m).ok); }
  { hsx::Params b; b.kind = "beam"; b.length = 1200; auto q = polyOk(b, "beam"); CHECK(q.panels[0].s[0] == 1200); }
  { hsx::Params b; b.kind = "nope"; CHECK(!hsx::solve(b).ok); }
  // hiệu năng: 1000 lần giải + JSON phải rất nhanh (tương tác kéo giãn không giật)
  { auto t0 = std::chrono::steady_clock::now(); size_t sz = 0;
    for (int i = 0; i < 1000; ++i) sz += hsx::toJson(hsx::solve(video(1000 + i % 600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay))).size();
    double ms = std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - t0).count();
    std::printf("perf: %.3f ms / lần (%zu bytes JSON)\n", ms / 1000, sz / 1000);
    CHECK(ms / 1000 < 5.0); }
  std::printf("OK: %zu panels, %d hinges, %zu cutlist rows\n", r.panels.size(), r.hingeCount, hsx::cutlist(r).size());
}
