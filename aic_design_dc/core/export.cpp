#include "cabinet.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <map>
#include <sstream>
#include <tuple>

namespace hsx {

std::vector<CutRow> cutlist(const Result& r) {
  std::map<std::tuple<std::string, long, long, long>, CutRow> m;
  for (auto& p : r.panels) {
    auto key = std::make_tuple(p.role, std::lround(p.L * 10), std::lround(p.W * 10), std::lround(p.T * 10));
    auto it = m.find(key);
    if (it == m.end()) m[key] = CutRow{p.role, p.L, p.W, p.T, 1, p.banding};
    else { it->second.qty++; it->second.banding += p.banding; }
  }
  std::vector<CutRow> out;
  for (auto& kv : m) out.push_back(kv.second);
  return out;
}

static void line(FILE* f, const char* layer, double x1, double y1, double x2, double y2) {
  std::fprintf(f, "0\nLINE\n8\n%s\n10\n%.3f\n20\n%.3f\n30\n0\n11\n%.3f\n21\n%.3f\n31\n0\n", layer, x1, y1, x2, y2);
}

bool exportDxf(const Result& r, const std::string& path, double SL, double SW, double kerf) {
  std::vector<const Panel*> ps;
  for (auto& p : r.panels) ps.push_back(&p);
  std::sort(ps.begin(), ps.end(), [](const Panel* a, const Panel* b) { return a->L * a->W > b->L * b->W; });
  FILE* f = std::fopen(path.c_str(), "w");
  if (!f) return false;
  std::fprintf(f, "0\nSECTION\n2\nENTITIES\n");
  // xếp kiểu "shelf": hàng ngang theo chiều dài ván
  double ox = 0, oy = 0, rowH = 0, overY = 0; int sheet = 0;
  for (const Panel* p : ps) {
    double L = p->L, W = p->W;
    const char* layer = "PANEL";
    if (L > SL || W > SW) {  // quá khổ tấm (vd. hậu 9.5 phải cắt từ tấm lớn/ghép): đặt riêng, gắn layer OVERSIZE
      layer = "OVERSIZE"; 
      double by = -(W + 200) - overY; overY += W + 200;
      line(f, layer, 0, by, L, by); line(f, layer, L, by, L, by + W);
      line(f, layer, L, by + W, 0, by + W); line(f, layer, 0, by + W, 0, by);
      continue;
    }
    if (ox + L > SL) { ox = 0; oy += rowH + kerf; rowH = 0; }
    if (oy + W > SW) { ++sheet; ox = 0; oy = 0; rowH = 0; }
    double bx = ox, by = oy + sheet * (SW + 200);
    line(f, "PANEL", bx, by, bx + L, by);
    line(f, "PANEL", bx + L, by, bx + L, by + W);
    line(f, "PANEL", bx + L, by + W, bx, by + W);
    line(f, "PANEL", bx, by + W, bx, by);
    for (auto& h : p->holes) {
      if (h.axis != p->tAxis) continue;  // chỉ lỗ khoan mặt ván
      double u = h.p[p->lAxis] - p->o[p->lAxis], v = h.p[p->wAxis] - p->o[p->wAxis];
      std::fprintf(f, "0\nCIRCLE\n8\nDRILL_%s\n10\n%.3f\n20\n%.3f\n30\n0\n40\n%.3f\n", h.kind, bx + u, by + v, h.dia / 2);
    }
    ox += L + kerf; rowH = std::max(rowH, W);
  }
  std::fprintf(f, "0\nENDSEC\n0\nEOF\n");
  std::fclose(f);
  return true;
}

bool parseParams(const std::string& text, Params& o, std::string& err) {
  std::istringstream in(text);
  std::string ln;
  auto b = [](const std::string& v) { return v == "1" || v == "true"; };
  while (std::getline(in, ln)) {
    size_t e = ln.find('=');
    if (e == std::string::npos) continue;
    std::string k = ln.substr(0, e), v = ln.substr(e + 1);
    while (!v.empty() && (v.back() == '\r' || v.back() == ' ')) v.pop_back();
    double d = std::atof(v.c_str());
    if (k == "width") o.width = d; else if (k == "depth") o.depth = d; else if (k == "height") o.height = d;
    else if (k == "thick") o.thick = d; else if (k == "back_thick") o.backThick = d;
    else if (k == "plinth") o.plinth = d; else if (k == "has_back") o.hasBack = b(v);
    else if (k == "back_style") o.back = v == "inset" ? BackStyle::Inset : BackStyle::Overlay;
    else if (k == "door_style") o.door = v == "inset" ? DoorStyle::Inset : v == "half" ? DoorStyle::Half : DoorStyle::Overlay;
    else if (k == "door_thick") o.doorThick = d; else if (k == "gap") o.gap = d;
    else if (k == "front_return") o.frontReturn = d; else if (k == "loft_height") o.loftHeight = d;
    else if (k == "loft_doors") o.loftDoors = (int)d; else if (k == "cam") o.cam = b(v);
    else if (k == "col") {  // width,drawers,drawerPitch,shelves,doors
      Column c; double a[5] = {0, 0, 0, 0, 0}; int i = 0;
      std::istringstream cs(v); std::string tok;
      while (i < 5 && std::getline(cs, tok, ',')) a[i++] = std::atof(tok.c_str());
      c.width = a[0]; c.drawers = (int)a[1]; c.drawerPitch = a[2]; c.shelves = (int)a[3]; c.doors = (int)a[4];
      o.cols.push_back(c);
    } else { err = "Khóa không hợp lệ: " + k; return false; }
  }
  return true;
}

static void esc(std::string& out, const std::string& s) {
  for (char c : s) { if (c == '"' || c == '\\') out += '\\'; out += c; }
}

std::string toJson(const Result& r) {
  std::string o; char buf[256];
  if (!r.ok) { o = "{\"ok\":false,\"error\":\""; esc(o, r.error); return o + "\"}"; }
  o = "{\"ok\":true,\"hinges\":" + std::to_string(r.hingeCount) + ",\"panels\":[";
  for (size_t i = 0; i < r.panels.size(); ++i) {
    const Panel& p = r.panels[i];
    if (i) o += ',';
    o += "{\"name\":\""; esc(o, p.name); o += "\",\"role\":\"" + p.role + "\",";
    std::snprintf(buf, sizeof buf, "\"o\":[%.3f,%.3f,%.3f],\"s\":[%.3f,%.3f,%.3f],\"banding\":%.1f,\"holes\":[",
                  p.o[0], p.o[1], p.o[2], p.s[0], p.s[1], p.s[2], p.banding);
    o += buf;
    for (size_t j = 0; j < p.holes.size(); ++j) {
      const Hole& h = p.holes[j];
      if (j) o += ',';
      std::snprintf(buf, sizeof buf, "{\"p\":[%.3f,%.3f,%.3f],\"axis\":%d,\"dia\":%.1f,\"depth\":%.1f,\"kind\":\"%s\"}",
                    h.p[0], h.p[1], h.p[2], h.axis, h.dia, h.depth, h.kind);
      o += buf;
    }
    o += "]}";
  }
  o += "],\"cutlist\":[";
  auto cl = cutlist(r);
  for (size_t i = 0; i < cl.size(); ++i) {
    std::snprintf(buf, sizeof buf, "%s{\"role\":\"%s\",\"L\":%.1f,\"W\":%.1f,\"T\":%.1f,\"qty\":%d,\"banding\":%.0f}",
                  i ? "," : "", cl[i].role.c_str(), cl[i].L, cl[i].W, cl[i].T, cl[i].qty, cl[i].banding);
    o += buf;
  }
  return o + "]}";
}

}  // namespace hsx
