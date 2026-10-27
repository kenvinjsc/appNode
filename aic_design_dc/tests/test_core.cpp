#include "../core/cabinet.hpp"
#include <cstdio>
#include <cstdlib>
#include <string>
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
  // resize giữ nguyên số bộ phận, chỉ đổi kích thước
  CHECK(hsx::solve(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay)).panels.size() ==
        hsx::solve(video(1207, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay)).panels.size());
  // lỗi đầu vào
  hsx::Params bad; bad.width = 30; CHECK(!hsx::solve(bad).ok);
  // xuất DXF
  auto r = hsx::solve(video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay));
  for (auto& pn : r.panels) for (auto& h : pn.holes) CHECK(std::string(h.kind) != "cam" && std::string(h.kind) != "bolt");
  { auto pc = video(1600, hsx::DoorStyle::Overlay, hsx::BackStyle::Overlay); pc.cam = true; CHECK(hsx::solve(pc).panels.size() == r.panels.size()); }
  CHECK(hsx::exportDxf(r, "/tmp/hsx_test.dxf"));
  std::printf("OK: %zu panels, %d hinges, %zu cutlist rows\n", r.panels.size(), r.hingeCount, hsx::cutlist(r).size());
}
