// Tiện ích hình học 2D dùng chung cho các chi tiết (header-only). Đơn vị mm, góc độ.
#pragma once
#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

namespace hsx {
namespace geom {

const double PI = 3.14159265358979323846;
using Poly = std::vector<double>;  // x0,y0,x1,y1,...

inline void pt(Poly& p, double a, double b) { p.push_back(a); p.push_back(b); }

// Số đoạn theo sai số cung (sag): dây cung lệch tối đa kSag mm so với đường cong thật.
const double kSag = 0.3;
inline int segsFor(double angleDeg, double radius) {
  radius = std::max(radius, 1.0);
  const double step = 2.0 * std::acos(std::max(0.0, 1.0 - kSag / radius));
  const int n = (int)std::ceil(std::fabs(angleDeg) * PI / 180.0 / std::max(step, 1e-3));
  return std::min(256, std::max(2, n));
}

// Cung quanh (cx,cy) bán kính rx/ry từ góc a0 đến a1 (độ, a1 có thể nhỏ hơn a0 = đi ngược chiều).
inline void arc(Poly& p, double cx, double cy, double rx, double ry, double a0, double a1, bool skipFirst = false) {
  const int n = std::max(2, segsFor(a1 - a0, std::max(rx, ry)));
  for (int i = skipFirst ? 1 : 0; i <= n; ++i) {
    double a = (a0 + (a1 - a0) * i / n) * PI / 180.0;
    pt(p, cx + rx * std::cos(a), cy + ry * std::sin(a));
  }
}

struct V2 { double x, y; };
inline V2 operator+(V2 a, V2 b) { return {a.x + b.x, a.y + b.y}; }
inline V2 operator-(V2 a, V2 b) { return {a.x - b.x, a.y - b.y}; }
inline V2 operator*(V2 a, double k) { return {a.x * k, a.y * k}; }
inline double len(V2 a) { return std::hypot(a.x, a.y); }
inline V2 unit(V2 a) { double l = len(a); return l > 1e-12 ? V2{a.x / l, a.y / l} : V2{0, 0}; }
inline double cross(V2 a, V2 b) { return a.x * b.y - a.y * b.x; }
inline double deg(V2 a) { return std::atan2(a.y, a.x) * 180.0 / PI; }

// Cách xử lý một đỉnh đa giác.
enum Kind { SQUARE = 0, ROUND = 1, CONCAVE = 2, CHAMFER = 3, DOGBONE = 4 };
struct Vtx { V2 p; Kind kind = SQUARE; double size = 0; };

// Dựng đa giác kín từ các đỉnh thô + cách xử lý từng đỉnh.
//  ROUND   : bo tròn tiếp tuyến bán kính size (cung lồi ở đỉnh lồi, cung lõm ở đỉnh lõm)
//  CONCAVE : cung tâm tại đỉnh (cắt khuyết vào góc)
//  CHAMFER : vát chéo cách đỉnh size
//  DOGBONE : khử dao: vòng tròn bán kính size tâm tại đỉnh, chỉ áp cho đỉnh lõm (reflex)
// Chiều đi ngược kim đồng hồ = vật liệu bên trái (biên ngoài); lỗ đi thuận kim đồng hồ.
inline Poly build(const std::vector<Vtx>& vs) {
  Poly out;
  const size_t n = vs.size();
  for (size_t i = 0; i < n; ++i) {
    const V2 P = vs[i].p, A = vs[(i + n - 1) % n].p, B = vs[(i + 1) % n].p;
    const V2 u1 = unit(P - A), u2 = unit(B - P);
    double l1 = len(P - A), l2 = len(B - P);
    double turn = cross(u1, u2);                      // >0 rẽ trái, <0 rẽ phải
    double s = std::min(vs[i].size, 0.5 * std::min(l1, l2));
    Kind k = vs[i].kind;
    if (s < 1e-6 || std::fabs(turn) < 1e-9) k = SQUARE;
    if (k == DOGBONE && turn >= 0) k = SQUARE;        // chỉ đỉnh lõm khi đi ngược kim đồng hồ
    if (k == SQUARE) { pt(out, P.x, P.y); continue; }
    const V2 T1 = P - u1 * s, T2 = P + u2 * s;
    const double sg = turn > 0 ? 1.0 : -1.0;
    if (k == CHAMFER) { pt(out, T1.x, T1.y); pt(out, T2.x, T2.y); }
    else if (k == ROUND) {
      const V2 nrm = {-u1.y * sg, u1.x * sg};         // pháp tuyến về phía tâm
      const V2 C = T1 + nrm * s;
      const double a0 = deg(T1 - C);
      arc(out, C.x, C.y, s, s, a0, a0 + sg * 90.0);
    } else if (k == CONCAVE) {
      const double a0 = deg(T1 - P);
      arc(out, P.x, P.y, s, s, a0, a0 - sg * 90.0);
    } else if (k == DOGBONE) {                        // cung 270° đi qua phía vật liệu
      const V2 b = unit(u1 - u2);
      const double tA = deg(u1 * -1.0), tB = deg(u2), tb = deg(b);
      auto norm = [](double a) { while (a < 0) a += 360; while (a >= 360) a -= 360; return a; };
      double ccw = norm(tB - tA), mid = norm(tb - tA);
      double sweep = (mid <= ccw) ? ccw : ccw - 360.0;  // chọn chiều có chứa phương b
      arc(out, P.x, P.y, s, s, tA, tA + sweep);
    }
  }
  return out;
}

inline double area(const Poly& p) {
  double a = 0; const size_t n = p.size() / 2;
  for (size_t i = 0; i < n; ++i) { size_t j = (i + 1) % n; a += p[2 * i] * p[2 * j + 1] - p[2 * j] * p[2 * i + 1]; }
  return a / 2;
}

}  // namespace geom
}  // namespace hsx
