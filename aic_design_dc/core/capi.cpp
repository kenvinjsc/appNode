// C ABI ổn định để Ruby (Fiddle) gọi — không phụ thuộc phiên bản Ruby/SketchUp.
#include "cabinet.hpp"
#include "nest.hpp"
#include <atomic>
#include <cstring>
#include <map>
#include <memory>
#include <mutex>
#include <thread>

#if defined(_WIN32)
#define HSX_API extern "C" __declspec(dllexport)
#else
#define HSX_API extern "C" __attribute__((visibility("default")))
#endif

namespace {
// Ruby gọi 2 lần (lấy kích thước, rồi lấy dữ liệu) -> nhớ kết quả lần gần nhất để chỉ giải một lần.
std::mutex g_mu;
std::string g_key, g_json;

const std::string& solveJson(const char* params) {
  std::string key = params ? params : "";
  if (key == g_key && !g_json.empty()) return g_json;
  hsx::Params p; std::string err;
  if (!hsx::parseParams(key, p, err)) g_json = "{\"ok\":false,\"error\":\"" + err + "\"}";
  else g_json = hsx::toJson(hsx::solve(p));
  g_key = key;
  return g_json;
}
}  // namespace

// Trả về độ dài JSON; ghi vào out nếu cap đủ (luôn kết thúc bằng '\0'). Gọi 2 lần: out=NULL để lấy kích thước.
HSX_API int hsx_solve(const char* params, char* out, int cap) {
  std::lock_guard<std::mutex> lk(g_mu);
  const std::string& json = solveJson(params);
  if (out && cap > (int)json.size()) std::memcpy(out, json.c_str(), json.size() + 1);
  return (int)json.size();
}

HSX_API int hsx_export_dxf(const char* params, const char* path) {
  hsx::Params p; std::string err;
  if (!hsx::parseParams(params ? params : "", p, err)) return 0;
  auto r = hsx::solve(p);
  return r.ok && hsx::exportDxf(r, path) ? 1 : 0;
}

// ===================== Gom chi tiết + xếp ván: chạy ở luồng nền =====================
// Luồng chính (SketchUp UI) chỉ gọi các hàm nhẹ: start (tạo luồng rồi trả về ngay), status (đọc nguyên tử),
// cancel, và lấy kết quả sau khi xong. Việc nặng (giải từng tủ, gom, MaxRects) chạy ở luồng riêng.
namespace {
struct NestHandle {
  std::atomic<int> state{0};          // 0 đang chạy, 1 xong, 2 lỗi, 3 đã hủy
  std::atomic<double> progress{0};
  std::atomic<bool> cancel{false};
  hsx::NestResult res;                // chỉ đọc khi state != 0
};
std::mutex g_nm;
std::map<int, std::shared_ptr<NestHandle>> g_handles;
int g_nextHandle = 1;

std::shared_ptr<NestHandle> getHandle(int h) {
  std::lock_guard<std::mutex> lk(g_nm);
  auto it = g_handles.find(h);
  return it == g_handles.end() ? nullptr : it->second;
}
int copyOut(const std::string& s, char* out, int cap) {
  if (out && cap > (int)s.size()) std::memcpy(out, s.c_str(), s.size() + 1);
  return (int)s.size();
}
}  // namespace

// Bắt đầu; trả về handle > 0. Luồng giữ shared_ptr nên an toàn dù handle bị giải phóng giữa chừng.
HSX_API int hsx_nest_start(const char* job) {
  auto h = std::make_shared<NestHandle>();
  const std::string text = job ? job : "";
  int id;
  { std::lock_guard<std::mutex> lk(g_nm); id = g_nextHandle++; g_handles[id] = h; }
  try {
    std::thread([h, text]() {
      try {
        hsx::NestJob j; std::string err;
        if (!hsx::parseJob(text, j, err)) { h->res.ok = false; h->res.error = err; h->state.store(2, std::memory_order_release); return; }
        h->res = hsx::runNest(j, &h->cancel, &h->progress);
        const int st = h->res.ok ? 1 : (h->cancel.load() ? 3 : 2);
        h->state.store(st, std::memory_order_release);
      } catch (...) {
        h->res.ok = false; h->res.error = "Lỗi nội bộ khi xếp ván";
        h->state.store(2, std::memory_order_release);
      }
    }).detach();
  } catch (...) {
    std::lock_guard<std::mutex> lk(g_nm); g_handles.erase(id);
    return 0;
  }
  return id;
}

// 0 chạy, 1 xong, 2 lỗi, 3 hủy, -1 handle sai. progress (0..1) có thể NULL.
HSX_API int hsx_nest_status(int h, double* progress) {
  auto p = getHandle(h);
  if (!p) return -1;
  if (progress) *progress = p->progress.load(std::memory_order_relaxed);
  return p->state.load(std::memory_order_acquire);
}

HSX_API void hsx_nest_cancel(int h) {
  if (auto p = getHandle(h)) p->cancel.store(true);
}

// Tóm tắt JSON (không kèm toạ độ); -1 nếu chưa xong.
HSX_API int hsx_nest_result(int h, char* out, int cap) {
  auto p = getHandle(h);
  if (!p || p->state.load(std::memory_order_acquire) == 0) return -1;
  return copyOut(hsx::nestSummaryJson(p->res), out, cap);
}

// Toạ độ một tấm ván (lấy theo yêu cầu để không đẩy JSON lớn qua luồng chính).
HSX_API int hsx_nest_sheet(int h, int mat, int sheet, char* out, int cap) {
  auto p = getHandle(h);
  if (!p || p->state.load(std::memory_order_acquire) != 1) return -1;
  return copyOut(hsx::nestSheetJson(p->res, mat, sheet), out, cap);
}

HSX_API int hsx_nest_export_dxf(int h, const char* path) {
  auto p = getHandle(h);
  if (!p || p->state.load(std::memory_order_acquire) != 1) return 0;
  return hsx::exportNestDxf(p->res, path ? path : "") ? 1 : 0;
}

HSX_API void hsx_nest_free(int h) {
  std::shared_ptr<NestHandle> p;
  { std::lock_guard<std::mutex> lk(g_nm); auto it = g_handles.find(h); if (it == g_handles.end()) return; p = it->second; g_handles.erase(it); }
  p->cancel.store(true);   // luồng (nếu còn chạy) tự dừng và tự giải phóng
}
