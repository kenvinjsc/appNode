#include "layout.hpp"
#include <cstdlib>

namespace hsx {
namespace {

struct Parser {
  const std::string& s;
  size_t i = 0;
  std::string err;
  explicit Parser(const std::string& t) : s(t) {}

  bool fail(const std::string& m) { if (err.empty()) err = m + " (vị trí " + std::to_string(i) + ")"; return false; }

  bool number(double& v) {
    const char* b = s.c_str() + i; char* e = nullptr;
    v = std::strtod(b, &e);
    if (e == b) return fail("Cần một số");
    i += (size_t)(e - b);
    return true;
  }

  bool leaf(LNode& n) {
    if (i >= s.size() || s[i] != '[') return fail("Cần '[' sau L");
    ++i;
    n.split = false; n.leaf = Column{};
    while (true) {
      if (i < s.size() && s[i] == ']') { ++i; return true; }
      if (i >= s.size()) return fail("Thiếu ']'");
      const char k = s[i++];
      if (i >= s.size() || s[i] != '=') return fail("Cần '=' sau khóa khoang");
      ++i;
      double v; if (!number(v)) return false;
      switch (k) {
        case 'd': n.leaf.drawers = (int)v; break;
        case 'p': n.leaf.drawerPitch = v; break;
        case 's': n.leaf.shelves = (int)v; break;
        case 'l': n.leaf.doors = (int)v; break;
        case 'm': n.leaf.doorStyle = (int)v - 1; break;
        case 'h': n.leaf.hinges = (int)v; break;
        case 'f': n.leaf.fixH = v; break;
        case 'g': n.leaf.doorGap = v; break;
        case 'x': n.leaf.ductW = v; break;
        case 'r': n.leaf.hingeSide = (int)v; break;
        default: return fail(std::string("Khóa khoang không hợp lệ: ") + k);
      }
      if (i < s.size() && s[i] == ',') ++i;
    }
  }

  bool node(LNode& n, int depth) {
    if (depth > 12) return fail("Bố cục lồng quá sâu");
    if (i >= s.size()) return fail("Bố cục trống");
    const char c = s[i++];
    if (c == 'L') return leaf(n);
    if (c != 'V' && c != 'H') return fail(std::string("Nút không hợp lệ: ") + c);
    if (i >= s.size() || s[i] != '(') return fail("Cần '(' sau V/H");
    ++i;
    n.split = true; n.dir = c; n.size.clear(); n.kids.clear();
    while (true) {
      double sz; if (!number(sz)) return false;
      if (sz < 0) return fail("Kích thước khoang âm");
      if (i >= s.size() || s[i] != ':') return fail("Cần ':' sau kích thước");
      ++i;
      LNode kid; if (!node(kid, depth + 1)) return false;
      n.size.push_back(sz); n.kids.push_back(kid);
      if (i < s.size() && s[i] == ',') { ++i; continue; }
      if (i < s.size() && s[i] == ')') { ++i; break; }
      return fail("Cần ',' hoặc ')'");
    }
    if (n.kids.size() > 40) return fail("Quá nhiều khoang trong một nhóm chia");
    return true;
  }
};

}  // namespace

bool parseLayout(const std::string& text, LNode& out, std::string& err) {
  Parser ps(text);
  if (!ps.node(out, 0)) { err = "Bố cục không hợp lệ: " + ps.err; return false; }
  if (ps.i != text.size()) { err = "Bố cục không hợp lệ: thừa ký tự sau nút gốc"; return false; }
  return true;
}

LNode layoutFromColumns(const Params& p) {
  std::vector<Column> cols = p.cols;
  if (cols.empty()) cols.push_back(Column{});
  LNode root; root.split = true; root.dir = 'V';
  for (const Column& c : cols) {
    LNode k;
    if (p.loftHeight > 0) {                 // mỗi cột: khoang chính + hộc trên (đợt ngăn nằm riêng từng cột)
      k.split = true; k.dir = 'H';
      LNode main; main.leaf = c; main.leaf.width = 0;
      LNode loft; loft.leaf.doors = p.loftDoors;
      k.size = {0, p.loftHeight}; k.kids = {main, loft};
    } else { k.leaf = c; k.leaf.width = 0; }
    root.size.push_back(c.width); root.kids.push_back(k);
  }
  return root;
}

}  // namespace hsx
