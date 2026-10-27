#include "nest.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <map>
#include <sstream>
#include <tuple>

namespace hsx {
namespace {

struct Rect { double x, y, w, h; };

// MaxRects (Best Short Side Fit) cho một tấm ván.
struct Bin {
  std::vector<Rect> free;
  double maxW = 0, maxH = 0;      // ước lượng nhanh chỗ trống lớn nhất (để bỏ qua tấm đầy)
  explicit Bin(double w, double h) { free.push_back({0, 0, w, h}); refresh(); }
  void refresh() { maxW = maxH = 0; for (auto& f : free) { maxW = std::max(maxW, f.w); maxH = std::max(maxH, f.h); } }

  bool find(double w, double h, Rect& out, double& s1, double& s2) const {
    bool ok = false; s1 = s2 = 1e18;
    for (auto& f : free) {
      if (w <= f.w + 1e-9 && h <= f.h + 1e-9) {
        double a = std::min(f.w - w, f.h - h), b = std::max(f.w - w, f.h - h);
        if (a < s1 - 1e-9 || (std::fabs(a - s1) <= 1e-9 && b < s2)) { s1 = a; s2 = b; out = {f.x, f.y, w, h}; ok = true; }
      }
    }
    return ok;
  }
  void place(const Rect& r) {
    std::vector<Rect> nf;
    for (auto& f : free) {
      if (r.x >= f.x + f.w || r.x + r.w <= f.x || r.y >= f.y + f.h || r.y + r.h <= f.y) { nf.push_back(f); continue; }
      if (r.x > f.x) nf.push_back({f.x, f.y, r.x - f.x, f.h});
      if (r.x + r.w < f.x + f.w) nf.push_back({r.x + r.w, f.y, f.x + f.w - r.x - r.w, f.h});
      if (r.y > f.y) nf.push_back({f.x, f.y, f.w, r.y - f.y});
      if (r.y + r.h < f.y + f.h) nf.push_back({f.x, r.y + r.h, f.w, f.y + f.h - r.y - r.h});
    }
    // bỏ ô trống nằm gọn trong ô khác
    std::vector<Rect> pr;
    for (size_t i = 0; i < nf.size(); ++i) {
      bool in = false;
      for (size_t j = 0; j < nf.size() && !in; ++j) {
        if (i == j) continue;
        const Rect &a = nf[i], &b = nf[j];
        if (a.x >= b.x - 1e-9 && a.y >= b.y - 1e-9 && a.x + a.w <= b.x + b.w + 1e-9 && a.y + a.h <= b.y + b.h + 1e-9 &&
            (i > j || a.w != b.w || a.h != b.h || a.x != b.x || a.y != b.y)) in = true;
      }
      if (!in && nf[i].w > 1e-6 && nf[i].h > 1e-6) pr.push_back(nf[i]);
    }
    free.swap(pr);
    refresh();
  }
};

double q1(double v) { return std::round(v * 10.0) / 10.0; }

std::string fmtThick(double t) {
  char b[32]; std::snprintf(b, sizeof b, "%.1fmm", t); return b;
}

}  // namespace

bool parseJob(const std::string& text, NestJob& job, std::string& err) {
  std::vector<std::string> sections(1);
  std::istringstream in(text);
  std::string ln;
  while (std::getline(in, ln)) {
    while (!ln.empty() && (ln.back() == '\r' || ln.back() == ' ')) ln.pop_back();
    if (ln == "---") { sections.emplace_back(); continue; }
    sections.back() += ln + "\n";
  }
  {  // tuỳ chọn
    std::istringstream os(sections[0]);
    while (std::getline(os, ln)) {
      size_t e = ln.find('=');
      if (e == std::string::npos) continue;
      std::string k = ln.substr(0, e); double d = std::atof(ln.c_str() + e + 1);
      if (k == "sheet_l") job.opts.sheetL = d; else if (k == "sheet_w") job.opts.sheetW = d;
      else if (k == "kerf") job.opts.kerf = d; else if (k == "trim") job.opts.trim = d;
      else if (k == "rotate") job.opts.rotate = d != 0;
      else { err = "Tuỳ chọn không hợp lệ: " + k; return false; }
    }
  }
  if (job.opts.sheetL < 100 || job.opts.sheetW < 100 || job.opts.kerf < 0 || job.opts.trim < 0 ||
      job.opts.sheetL - 2 * job.opts.trim < 50 || job.opts.sheetW - 2 * job.opts.trim < 50) { err = "Khổ ván/lề không hợp lệ"; return false; }
  for (size_t i = 1; i < sections.size(); ++i) {
    std::string body, cnt;
    std::istringstream ss(sections[i]);
    int count = 1;
    while (std::getline(ss, ln)) {
      if (ln.rfind("count=", 0) == 0) count = std::max(1, std::atoi(ln.c_str() + 6)); else body += ln + "\n";
    }
    if (body.find_first_not_of(" \n") == std::string::npos) continue;
    Params p; std::string e;
    if (!parseParams(body, p, e)) { err = "Phần " + std::to_string(i) + ": " + e; return false; }
    job.cabs.emplace_back(count, p);
  }
  return true;
}

NestResult runNest(const NestJob& job, const std::atomic<bool>* cancel, std::atomic<double>* progress) {
  NestResult R; R.opts = job.opts;
  auto fail = [&](const std::string& m) { R.ok = false; R.error = m; return R; };
  auto cancelled = [&]() { return cancel && cancel->load(std::memory_order_relaxed); };
  auto prog = [&](double v) { if (progress) progress->store(v, std::memory_order_relaxed); };
  if (job.cabs.empty()) return fail("Không có chi tiết nào để xếp");

  // 1) gom chi tiết: cùng (vật liệu, kích thước cắt, chỉ dán) -> cộng dồn số lượng
  using Key = std::tuple<long, long, long, long, long>;
  std::map<Key, int> idx;
  for (size_t c = 0; c < job.cabs.size(); ++c) {
    if (cancelled()) return fail("Đã hủy");
    Result sol = solve(job.cabs[c].second);
    if (!sol.ok) return fail("Chi tiết #" + std::to_string(c + 1) + ": " + sol.error);
    for (auto& pn : sol.panels) {
      if (!(pn.role == "carcass" || pn.role == "back" || pn.role == "front" || pn.role == "drawer")) continue;
      const double bw0 = pn.band[2 * pn.wAxis], bw1 = pn.band[2 * pn.wAxis + 1];
      const double bl0 = pn.band[2 * pn.lAxis], bl1 = pn.band[2 * pn.lAxis + 1];
      const double cutL = q1(pn.L - bl0 - bl1), cutW = q1(pn.W - bw0 - bw1);
      auto q = [](double v) { return std::lround(v * 10); };
      Key key{q(pn.T), q(cutL), q(cutW), q(bw0) * 1000 + q(bw1), q(bl0) * 1000 + q(bl1)};
      auto it = idx.find(key);
      if (it == idx.end()) {
        NestItem ni; ni.name = pn.name; ni.mat = fmtThick(pn.T); ni.thick = pn.T; ni.w = cutL; ni.h = cutW; ni.qty = 0; ni.role = pn.role;
        ni.band[0] = bw0; ni.band[1] = bw1; ni.band[2] = bl0; ni.band[3] = bl1;
        it = idx.emplace(key, (int)R.items.size()).first; R.items.push_back(ni);
      }
      R.items[it->second].qty += job.cabs[c].first;
      R.partCount += job.cabs[c].first;
    }
    prog(0.2 * (c + 1) / job.cabs.size());
  }

  // 2) xếp ván theo từng vật liệu
  std::map<std::string, std::vector<int>> byMat;
  for (size_t i = 0; i < R.items.size(); ++i) byMat[R.items[i].mat].push_back((int)i);
  const double UL = job.opts.sheetL - 2 * job.opts.trim, UW = job.opts.sheetW - 2 * job.opts.trim, K = job.opts.kerf;
  size_t totalUnits = 0, done = 0;
  for (auto& kv : byMat) for (int i : kv.second) totalUnits += R.items[i].qty;
  for (auto& kv : byMat) {
    MatOut mo; mo.mat = kv.first; mo.thick = R.items[kv.second[0]].thick;
    std::vector<int> order = kv.second;
    std::sort(order.begin(), order.end(), [&](int a, int b) {
      const auto &x = R.items[a], &y = R.items[b];
      if (std::fabs(x.w - y.w) > 1e-9) return x.w > y.w;
      return x.h > y.h;
    });
    std::vector<Bin> bins;
    for (int ii : order) {
      const NestItem& it = R.items[ii];
      for (int u = 0; u < it.qty; ++u) {
        if (cancelled()) return fail("Đã hủy");
        ++done; if ((done & 63) == 0) prog(0.2 + 0.8 * done / std::max<size_t>(1, totalUnits));
        double pw = it.w, ph = it.h; bool rot = false;
        const bool fitN = pw <= UL + 1e-9 && ph <= UW + 1e-9;
        const bool fitR = job.opts.rotate && ph <= UL + 1e-9 && pw <= UW + 1e-9;
        if (!fitN && !fitR) { if (u == 0) mo.oversize.push_back(it.name); continue; }
        mo.parts++; mo.partsArea += it.w * it.h;
        Rect best{}; int bs = -1; bool bestRot = false;
        for (size_t sh = 0; sh < bins.size() && bs < 0; ++sh) {   // tấm đầu tiên còn chỗ (lấp đầy tấm cũ trước)
          double s1a, s2a, s1b = 1e18, s2b = 1e18; Rect ra{}, rb{};
          const bool a = fitN && bins[sh].maxW >= pw + K - 1e-9 && bins[sh].maxH >= ph + K - 1e-9 && bins[sh].find(pw + K, ph + K, ra, s1a, s2a);
          const bool b = fitR && bins[sh].maxW >= ph + K - 1e-9 && bins[sh].maxH >= pw + K - 1e-9 && bins[sh].find(ph + K, pw + K, rb, s1b, s2b);
          if (a && (!b || s1a < s1b || (s1a == s1b && s2a <= s2b))) { best = ra; bs = (int)sh; bestRot = false; }
          else if (b) { best = rb; bs = (int)sh; bestRot = true; }
        }
        if (bs < 0) {                                           // mở tấm mới
          bins.emplace_back(UL + K, UW + K); mo.sheets.emplace_back();
          bs = (int)bins.size() - 1;
          double s1, s2;
          if (fitN) { bins[bs].find(pw + K, ph + K, best, s1, s2); bestRot = false; }
          else { bins[bs].find(ph + K, pw + K, best, s1, s2); bestRot = true; }
        }
        bins[bs].place(best);
        rot = bestRot;
        Placement pl{ii, best.x + job.opts.trim, best.y + job.opts.trim, rot ? ph : pw, rot ? pw : ph, rot};
        mo.sheets[bs].items.push_back(pl); mo.sheets[bs].used += it.w * it.h;
      }
    }
    mo.sheetArea = mo.sheets.size() * job.opts.sheetL * job.opts.sheetW;
    R.totalSheets += (int)mo.sheets.size();
    R.mats.push_back(std::move(mo));
  }
  double pa = 0, sa = 0;
  for (auto& m : R.mats) { pa += m.partsArea; sa += m.sheetArea; }
  R.wastePct = sa > 0 ? 100.0 * (1.0 - pa / sa) : 0;
  prog(1.0);
  return R;
}

static void esc(std::string& o, const std::string& s) { for (char c : s) { if (c == '"' || c == '\\') o += '\\'; o += c; } }

std::string nestSummaryJson(const NestResult& r) {
  char b[256]; std::string o;
  if (!r.ok) { o = "{\"ok\":false,\"error\":\""; esc(o, r.error); return o + "\"}"; }
  std::snprintf(b, sizeof b, "{\"ok\":true,\"opts\":{\"sheetL\":%.0f,\"sheetW\":%.0f,\"kerf\":%.1f,\"trim\":%.1f,\"rotate\":%s},\"partCount\":%d,\"totalSheets\":%d,\"wastePct\":%.2f,\"items\":[",
                r.opts.sheetL, r.opts.sheetW, r.opts.kerf, r.opts.trim, r.opts.rotate ? "true" : "false", r.partCount, r.totalSheets, r.wastePct);
  o = b;
  for (size_t i = 0; i < r.items.size(); ++i) {
    const auto& it = r.items[i];
    o += i ? ",{\"name\":\"" : "{\"name\":\"";
    esc(o, it.name);
    std::snprintf(b, sizeof b, "\",\"mat\":\"%s\",\"role\":\"%s\",\"w\":%.1f,\"h\":%.1f,\"qty\":%d,\"band\":[%.2f,%.2f,%.2f,%.2f]}",
                  it.mat.c_str(), it.role.c_str(), it.w, it.h, it.qty, it.band[0], it.band[1], it.band[2], it.band[3]);
    o += b;
  }
  o += "],\"mats\":[";
  for (size_t i = 0; i < r.mats.size(); ++i) {
    const auto& m = r.mats[i];
    const double w = m.sheetArea > 0 ? 100.0 * (1.0 - m.partsArea / m.sheetArea) : 0;
    std::snprintf(b, sizeof b, "%s{\"mat\":\"%s\",\"thick\":%.1f,\"sheets\":%d,\"parts\":%d,\"partsArea\":%.0f,\"sheetArea\":%.0f,\"wastePct\":%.2f,\"oversize\":[",
                  i ? "," : "", m.mat.c_str(), m.thick, (int)m.sheets.size(), m.parts, m.partsArea, m.sheetArea, w);
    o += b;
    for (size_t j = 0; j < m.oversize.size(); ++j) { o += j ? ",\"" : "\""; esc(o, m.oversize[j]); o += "\""; }
    o += "]}";
  }
  return o + "]}";
}

std::string nestSheetJson(const NestResult& r, int mat, int sheet) {
  if (!r.ok || mat < 0 || mat >= (int)r.mats.size() || sheet < 0 || sheet >= (int)r.mats[mat].sheets.size()) return "{\"ok\":false}";
  const auto& s = r.mats[mat].sheets[sheet];
  char b[200];
  std::snprintf(b, sizeof b, "{\"ok\":true,\"mat\":\"%s\",\"sheet\":%d,\"L\":%.0f,\"W\":%.0f,\"used\":%.0f,\"items\":[",
                r.mats[mat].mat.c_str(), sheet, r.opts.sheetL, r.opts.sheetW, s.used);
  std::string o = b;
  for (size_t i = 0; i < s.items.size(); ++i) {
    const auto& p = s.items[i];
    std::snprintf(b, sizeof b, "%s{\"i\":%d,\"x\":%.1f,\"y\":%.1f,\"w\":%.1f,\"h\":%.1f,\"r\":%d}", i ? "," : "", p.item, p.x, p.y, p.w, p.h, p.rot ? 1 : 0);
    o += b;
  }
  return o + "]}";
}

bool exportNestDxf(const NestResult& r, const std::string& path) {
  if (!r.ok) return false;
  FILE* f = std::fopen(path.c_str(), "w");
  if (!f) return false;
  std::fprintf(f, "0\nSECTION\n2\nENTITIES\n");
  auto line = [&](const char* layer, double x1, double y1, double x2, double y2) {
    std::fprintf(f, "0\nLINE\n8\n%s\n10\n%.3f\n20\n%.3f\n30\n0\n11\n%.3f\n21\n%.3f\n31\n0\n", layer, x1, y1, x2, y2);
  };
  auto rect = [&](const char* layer, double x, double y, double w, double h) {
    line(layer, x, y, x + w, y); line(layer, x + w, y, x + w, y + h); line(layer, x + w, y + h, x, y + h); line(layer, x, y + h, x, y);
  };
  double oy = 0;
  for (auto& m : r.mats) {
    for (size_t s = 0; s < m.sheets.size(); ++s) {
      rect("SHEET", 0, oy, r.opts.sheetL, r.opts.sheetW);
      std::fprintf(f, "0\nTEXT\n8\nLABEL\n10\n0\n20\n%.3f\n30\n0\n40\n30\n1\n%s tam %zu\n", oy + r.opts.sheetW + 20, m.mat.c_str(), s + 1);
      for (auto& p : m.sheets[s].items) {
        rect("PARTS", p.x, oy + p.y, p.w, p.h);
        std::fprintf(f, "0\nTEXT\n8\nLABEL\n10\n%.3f\n20\n%.3f\n30\n0\n40\n14\n1\n%.0fx%.0f\n", p.x + 5, oy + p.y + 5, p.w, p.h);
      }
      oy += r.opts.sheetW + 300;
    }
  }
  std::fprintf(f, "0\nENDSEC\n0\nEOF\n");
  std::fclose(f);
  return true;
}

}  // namespace hsx
