// Liên kết cam chốt (tự dò mối nối chữ T giữa các tấm thân tủ) và dán cạnh theo độ phơi lộ.
#include "cabinet.hpp"
#include <algorithm>
#include <cmath>

namespace hsx {
namespace {
const double EPS = 1e-6;

double overlap1(double a0, double a1, double b0, double b1) { return std::min(a1, b1) - std::max(a0, b0); }

bool isCarcass(const Panel& p) { return p.role == "carcass" && p.prismAxis < 0; }

void addHole(Panel& pn, const double c[3], int axis, double dia, double depth, const char* kind, int side) {
  Hole h{{c[0], c[1], c[2]}, axis, dia, depth, kind, side};
  pn.holes.push_back(h);
}
}  // namespace

// Mối nối chữ T: đầu (cạnh) tấm A tựa lên mặt tấm B (B mỏng theo trục k, A không mỏng theo k).
//  - A: cam Ø15×12.5 khoan từ mặt nhỏ, cách đầu camBack; lỗ ty Ø8 xuyên đầu tấm
//  - B: lỗ ty vít Ø5×10 trên mặt tiếp xúc
//  - giữa các cam: chốt gỗ Ø8 (A sâu 15, B sâu 12)
void addJoinery(Result& r, const Params& p) {
  auto& P = r.panels;
  for (size_t ia = 0; ia < P.size(); ++ia) {
    for (size_t ib = 0; ib < P.size(); ++ib) {
      if (ia == ib || !isCarcass(P[ia]) || !isCarcass(P[ib])) continue;
      Panel &A = P[ia], &B = P[ib];
      for (int k = 0; k < 3; ++k) {
        if (B.tAxis != k || A.tAxis == k) continue;
        int sign = 0;                       // +1: A nằm phía -k của B và chạm mặt nhỏ của B
        if (std::fabs(A.o[k] + A.s[k] - B.o[k]) < EPS) sign = +1;
        else if (std::fabs(B.o[k] + B.s[k] - A.o[k]) < EPS) sign = -1;
        if (!sign) continue;
        const int th = A.tAxis;             // trục mỏng của A (nằm trong mặt tiếp xúc)
        const int ln = 3 - k - th;          // trục dọc đường nối
        const double lo = std::max(A.o[ln], B.o[ln]), hi = std::min(A.o[ln] + A.s[ln], B.o[ln] + B.s[ln]);
        const double tov = overlap1(A.o[th], A.o[th] + A.s[th], B.o[th], B.o[th] + B.s[th]);
        if (hi - lo < 2 * p.camEdge || tov < 5) continue;   // mối nối quá ngắn/mỏng
        const double c = sign > 0 ? B.o[k] : B.o[k] + B.s[k];   // toạ độ mặt tiếp xúc
        const double thMid = std::max(A.o[th], B.o[th]) + tov / 2;
        const double span = (hi - lo) - 2 * p.camEdge;
        const int n = std::max(2, 1 + (int)std::ceil(span / std::max(50.0, p.camSpacing) - 1e-9));
        const int sideA = sign > 0 ? +1 : -1;  // đầu A ở phía +k nếu A nằm dưới B
        const int sideB = sign > 0 ? -1 : +1;  // mặt B tiếp xúc
        std::vector<double> pos;
        for (int i = 0; i < n; ++i) pos.push_back(lo + p.camEdge + span * i / (n - 1));
        for (double s : pos) {
          double pt[3];
          pt[k] = c - sign * p.camBack; pt[ln] = s; pt[th] = A.o[th];
          addHole(A, pt, th, 15, 12.5, "cam", -1);                  // cam, khoan từ mặt nhỏ của A
          pt[k] = c; pt[ln] = s; pt[th] = thMid;
          addHole(A, pt, k, 8, p.camBack, "channel", sideA);        // lỗ ty xuyên đầu A tới tâm cam
          addHole(B, pt, k, 5, 10, "bolt", sideB);                  // ty vít trên mặt B
        }
        for (size_t i = 0; i + 1 < pos.size(); ++i) {                // chốt gỗ ở giữa hai cam liên tiếp
          double pt[3]; pt[k] = c; pt[ln] = (pos[i] + pos[i + 1]) / 2; pt[th] = thMid;
          addHole(A, pt, k, 8, 15, "dowel", sideA);
          addHole(B, pt, k, 8, 12, "dowel", sideB);
        }
      }
    }
  }
}

// Dán cạnh: mặt cánh/hộc kéo dán 4 cạnh (bandFrontThick); tấm thân tủ dán các cạnh không bị tấm khác che
// (và không chạm sàn) với bandThick; hậu, ruột hộc kéo, kính không dán.
void applyBanding(Result& r, const Params& p) {
  auto& P = r.panels;
  for (auto& A : P) {
    for (int i = 0; i < 6; ++i) A.band[i] = 0;
    if (A.prismAxis >= 0) continue;                       // tấm biên dạng: dán theo bandEdges ở board.cpp
    const bool front = A.role == "front", carc = A.role == "carcass";
    if (!front && !carc) continue;
    for (int a = 0; a < 3; ++a) {
      if (a == A.tAxis) continue;                         // mặt dày (mặt ván) không dán
      for (int side = 0; side < 2; ++side) {
        bool covered = false;
        if (carc) {
          const double plane = A.o[a] + (side ? A.s[a] : 0);
          if (a == 2 && side == 0 && plane < EPS) covered = true;   // chạm sàn
          const int b = (a + 1) % 3, c = (a + 2) % 3;
          const double faceArea = A.s[b] * A.s[c];
          double cov = 0;
          for (const auto& O : P) {
            if (&O == &A || !(O.role == "carcass" || O.role == "back") || O.prismAxis >= 0) continue;
            const double touch = side ? O.o[a] : O.o[a] + O.s[a];
            if (std::fabs(touch - plane) > EPS) continue;
            const double ob = overlap1(A.o[b], A.o[b] + A.s[b], O.o[b], O.o[b] + O.s[b]);
            const double oc = overlap1(A.o[c], A.o[c] + A.s[c], O.o[c], O.o[c] + O.s[c]);
            if (ob > EPS && oc > EPS) cov += ob * oc;
          }
          if (cov >= 0.9 * faceArea) covered = true;
        }
        if (!covered) A.band[2 * a + side] = front ? p.bandFrontThick : p.bandThick;
      }
    }
    double len = 0;                                        // tổng chiều dài các cạnh dán
    for (int a = 0; a < 3; ++a) {
      if (a == A.tAxis) continue;
      const int o = 3 - a - A.tAxis;
      for (int side = 0; side < 2; ++side) if (A.band[2 * a + side] > 0) len += A.s[o];
    }
    A.banding = len;
  }
}

}  // namespace hsx
