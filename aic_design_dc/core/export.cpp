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
  using Key = std::tuple<std::string, long, long, long, long, long, long, long>;
  std::map<Key, CutRow> m;
  for (auto& p : r.panels) {
    CutRow row{p.role, p.L, p.W, p.T, 1, p.banding, 0, 0, {0, 0, 0, 0}};
    row.band[0] = p.band[2 * p.wAxis];     row.band[1] = p.band[2 * p.wAxis + 1];   // cạnh dài ở W nhỏ / lớn
    row.band[2] = p.band[2 * p.lAxis];     row.band[3] = p.band[2 * p.lAxis + 1];   // cạnh ngắn ở L nhỏ / lớn
    row.cutW = p.W - row.band[0] - row.band[1];
    row.cutL = p.L - row.band[2] - row.band[3];
    auto q = [](double v) { return std::lround(v * 10); };
    Key key{p.role, q(p.L), q(p.W), q(p.T), q(row.band[0]) * 1000 + q(row.band[1]), q(row.band[2]) * 1000 + q(row.band[3]), 0, 0};
    auto it = m.find(key);
    if (it == m.end()) m[key] = row;
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
    if (p->prismAxis >= 0 && p->prismAxis == p->tAxis && p->poly.size() >= 6) {  // biên dạng thật (vd. đợt bo góc)
      const int ia = (p->prismAxis + 1) % 3;
      const size_t n = p->poly.size() / 2;
      auto map = [&](size_t i, double& x, double& y) {
        double a = p->poly[2 * i], b = p->poly[2 * i + 1];
        double wl = (ia == p->lAxis) ? a : b, ww = (ia == p->lAxis) ? b : a;
        x = bx + wl - p->o[p->lAxis]; y = by + ww - p->o[p->wAxis];
      };
      for (size_t i = 0; i < n; ++i) {
        double x1, y1, x2, y2; map(i, x1, y1); map((i + 1) % n, x2, y2);
        line(f, "PANEL", x1, y1, x2, y2);
      }
    } else {
      line(f, "PANEL", bx, by, bx + L, by);
      line(f, "PANEL", bx + L, by, bx + L, by + W);
      line(f, "PANEL", bx + L, by + W, bx, by + W);
      line(f, "PANEL", bx, by + W, bx, by);
    }
    if (p->prismAxis >= 0 && p->prismAxis == p->tAxis) {      // rãnh/khoét: vẽ đường bao theo độ sâu
      const int ia = (p->prismAxis + 1) % 3;
      for (auto& pk : p->pockets) {
        const size_t n = pk.loop.size() / 2;
        char lay[64]; std::snprintf(lay, sizeof lay, "%s_%.1f", pk.kind.c_str(), pk.depth);
        for (size_t i = 0; i < n; ++i) {
          size_t j = (i + 1) % n; double q[4];
          for (int t = 0; t < 2; ++t) {
            size_t k = t ? j : i; double a = pk.loop[2 * k], b = pk.loop[2 * k + 1];
            double wl = (ia == p->lAxis) ? a : b, ww = (ia == p->lAxis) ? b : a;
            q[2 * t] = bx + wl - p->o[p->lAxis]; q[2 * t + 1] = by + ww - p->o[p->wAxis];
          }
          line(f, lay, q[0], q[1], q[2], q[3]);
        }
      }
    }
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
    else if (k == "back_style") o.back = v == "inset" ? BackStyle::Inset : v == "flush" ? BackStyle::Flush : BackStyle::Overlay;
    else if (k == "door_style") o.door = v == "inset" ? DoorStyle::Inset : v == "half" ? DoorStyle::Half : DoorStyle::Overlay;
    else if (k == "door_thick") o.doorThick = d; else if (k == "gap") o.gap = d;
    else if (k == "front_return") o.frontReturn = d; else if (k == "loft_height") o.loftHeight = d;
    else if (k == "loft_doors") o.loftDoors = (int)d; else if (k == "cam") o.cam = b(v);
    else if (k == "cam_spacing") o.camSpacing = d; else if (k == "cam_edge") o.camEdge = d; else if (k == "cam_back") o.camBack = d;
    else if (k == "band") o.band = b(v); else if (k == "band_thick") o.bandThick = d;
    else if (k == "band_front_thick") o.bandFrontThick = d; else if (k == "band_edges") o.bandEdges = v;
    else if (k == "cab_type") o.cabType = v; else if (k == "rail_w") o.railW = d; else if (k == "rail_count") o.railCount = (int)d;
    else if (k == "drawer_slide") o.drawerSlide = v; else if (k == "slide_len") o.slideLen = d;
    else if (k == "top_cover") o.topCover = d; else if (k == "top_cover_overhang") o.topCoverOverhang = d;
    else if (k == "end_panel") o.endPanel = v; else if (k == "end_panel_w") o.endPanelW = d;
    else if (k == "hang_z") o.hangZ = d; else if (k == "side_on_plinth") o.sideOnPlinth = b(v);
    else if (k == "plinth_front") o.plinthFront = b(v); else if (k == "plinth_setback") o.plinthSetback = d; else if (k == "door_max_w") o.doorMaxW = d; else if (k == "layout") o.layout = v;
    else if (k == "sheet_w") o.sheetW = d; else if (k == "kind") o.kind = v;
    else if (k == "radius") o.radius = d; else if (k == "angle") o.angle = d;
    else if (k == "part_height") o.partHeight = d; else if (k == "part_thick") o.partThick = d;
    else if (k == "corner_fl") o.cornerFL = d; else if (k == "corner_fr") o.cornerFR = d;
    else if (k == "corner_br") o.cornerBR = d; else if (k == "corner_bl") o.cornerBL = d;
    else if (k == "length") o.length = d; else if (k == "size_a") o.sizeA = d; else if (k == "size_b") o.sizeB = d;
    else if (k == "profile") o.profile = v;
    else if (k == "corner_style") o.cornerStyle = v; else if (k == "joint_type") o.jointType = v;
    else if (k == "joint_mode") o.jointMode = v; else if (k == "joint_edges") o.jointEdges = v;
    else if (k == "joint_count") o.jointCount = (int)d; else if (k == "joint_len") o.jointLen = d;
    else if (k == "joint_depth") o.jointDepth = d; else if (k == "joint_margin") o.jointMargin = d;
    else if (k == "joint_angle") o.jointAngle = d; else if (k == "dogbone") o.dogbone = d;
    else if (k == "frame_border") o.frameBorder = d; else if (k == "frame_r") o.frameR = d;
    else if (k == "frame_style") o.frameStyle = v; else if (k == "rabbet_w") o.rabbetW = d; else if (k == "rabbet_d") o.rabbetD = d;
    else if (k == "groove_count") o.grooveCount = (int)d; else if (k == "groove_w") o.grooveW = d;
    else if (k == "groove_depth") o.grooveDepth = d; else if (k == "groove_offset") o.grooveOffset = d;
    else if (k == "groove_spacing") o.grooveSpacing = d; else if (k == "groove_inset") o.grooveInset = d;
    else if (k == "groove_face") o.grooveFace = v;
    else if (k == "kerf_w") o.kerfW = d; else if (k == "kerf_remain") o.kerfRemain = d;
    else if (k == "bend_radius") o.bendRadius = d; else if (k == "kerf_zone") o.kerfZone = d; else if (k == "kerf_start") o.kerfStart = d;
    else if (k == "stile") o.stile = d; else if (k == "rail") o.rail = d; else if (k == "frame_thick") o.frameThick = d;
    else if (k == "panel_thick") o.panelThick = d; else if (k == "panel_type") o.panelType = v;
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
  o.reserve(r.panels.size() * 400 + 256);
  if (!r.ok) { o = "{\"ok\":false,\"error\":\""; esc(o, r.error); return o + "\"}"; }
  o = "{\"ok\":true,\"hinges\":" + std::to_string(r.hingeCount) + ",\"info\":\"";
  esc(o, r.info);
  o += "\",\"panels\":[";
  for (size_t i = 0; i < r.panels.size(); ++i) {
    const Panel& p = r.panels[i];
    if (i) o += ',';
    o += "{\"id\":\""; esc(o, p.id); o += "\",\"name\":\""; esc(o, p.name); o += "\",\"role\":\"" + p.role + "\",";
    if (p.prismAxis >= 0) {
      o += "\"axis\":" + std::to_string(p.prismAxis) + ",\"poly\":[";
      for (size_t j = 0; j < p.poly.size(); ++j) { std::snprintf(buf, sizeof buf, "%s%.3f", j ? "," : "", p.poly[j]); o += buf; }
      o += "],";
    }
    {
      bool any = false; for (double v : p.band) any = any || v > 0;
      if (any) { std::snprintf(buf, sizeof buf, "\"band\":[%.2f,%.2f,%.2f,%.2f,%.2f,%.2f],", p.band[0], p.band[1], p.band[2], p.band[3], p.band[4], p.band[5]); o += buf; }
    }
    if (!p.pockets.empty()) {
      o += "\"pockets\":[";
      for (size_t j = 0; j < p.pockets.size(); ++j) {
        const auto& pk = p.pockets[j];
        std::snprintf(buf, sizeof buf, "%s{\"top\":%s,\"depth\":%.3f,\"kind\":\"%s\",\"loop\":[", j ? "," : "", pk.top ? "true" : "false", pk.depth, pk.kind.c_str());
        o += buf;
        for (size_t m = 0; m < pk.loop.size(); ++m) { std::snprintf(buf, sizeof buf, "%s%.3f", m ? "," : "", pk.loop[m]); o += buf; }
        o += "]}";
      }
      o += "],";
    }
    std::snprintf(buf, sizeof buf, "\"o\":[%.3f,%.3f,%.3f],\"s\":[%.3f,%.3f,%.3f],\"banding\":%.1f,\"holes\":[",
                  p.o[0], p.o[1], p.o[2], p.s[0], p.s[1], p.s[2], p.banding);
    o += buf;
    for (size_t j = 0; j < p.holes.size(); ++j) {
      const Hole& h = p.holes[j];
      if (j) o += ',';
      std::snprintf(buf, sizeof buf, "{\"p\":[%.3f,%.3f,%.3f],\"axis\":%d,\"dia\":%.1f,\"depth\":%.1f,\"kind\":\"%s\",\"side\":%d}",
                    h.p[0], h.p[1], h.p[2], h.axis, h.dia, h.depth, h.kind, h.side);
      o += buf;
    }
    o += "]}";
  }
  o += "],\"openings\":[";
  for (size_t i = 0; i < r.openings.size(); ++i) {
    const Opening& op = r.openings[i];
    std::snprintf(buf, sizeof buf, "%s{\"t\":\"%s\",\"x0\":%.2f,\"x1\":%.2f,\"z0\":%.2f,\"z1\":%.2f,\"d\":%d,\"s\":%d,\"l\":%d,\"p\":[", i ? "," : "",
                  op.tag.c_str(), op.x0, op.x1, op.z0, op.z1, op.drawers, op.shelves, op.doors);
    o += buf;
    for (size_t j = 0; j < op.path.size(); ++j) o += (j ? "," : "") + std::to_string(op.path[j]);
    o += "]}";
  }
  o += "],\"cutlist\":[";
  auto cl = cutlist(r);
  for (size_t i = 0; i < cl.size(); ++i) {
    std::snprintf(buf, sizeof buf,
                  "%s{\"role\":\"%s\",\"L\":%.1f,\"W\":%.1f,\"T\":%.1f,\"qty\":%d,\"banding\":%.0f,\"cutL\":%.1f,\"cutW\":%.1f,\"band\":[%.2f,%.2f,%.2f,%.2f]}",
                  i ? "," : "", cl[i].role.c_str(), cl[i].L, cl[i].W, cl[i].T, cl[i].qty, cl[i].banding, cl[i].cutL, cl[i].cutW,
                  cl[i].band[0], cl[i].band[1], cl[i].band[2], cl[i].band[3]);
    o += buf;
  }
  return o + "]}";
}

}  // namespace hsx
