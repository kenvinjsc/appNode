// Bố cục tủ dạng CÂY KHOANG (giống quy trình trong video): khung rỗng -> chia đợt/vách -> đặt ngăn kéo/cánh vào từng khoang.
//
//   node := L[d=3,p=180,s=0,l=2,m=1,h=0]          khoang lá: d ngăn kéo, p cao ngăn, s đợt, l cánh (-1 = tự theo rộng tối đa),
//                                                   m kiểu cánh (1 trùm ngoài,2 trùm nửa,3 lọt lòng), h số bản lề (0 = tự)
//         | V(size:node,size:node,...)            chia VÁCH đứng: các khoang từ trái sang phải
//         | H(size:node,size:node,...)            chia ĐỢT ngang: các khoang từ dưới lên
//   size = mm thông thủy của khoang (0 = tự chia đều phần còn lại)
#pragma once
#include <string>
#include <vector>
#include "cabinet.hpp"

namespace hsx {

struct LNode {
  bool split = false;
  char dir = 'V';
  std::vector<double> size;
  std::vector<LNode> kids;
  Column leaf;          // nội dung khoang lá (width không dùng)
};

bool parseLayout(const std::string& text, LNode& out, std::string& err);
LNode layoutFromColumns(const Params& p);   // tủ kiểu cũ: các cột (+ hộc trên) -> cây tương đương

}  // namespace hsx
