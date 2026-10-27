// C ABI ổn định để Ruby (Fiddle) gọi — không phụ thuộc phiên bản Ruby/SketchUp.
#include "cabinet.hpp"
#include <cstring>
#include <mutex>

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
