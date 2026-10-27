// C ABI ổn định để Ruby (Fiddle) gọi — không phụ thuộc phiên bản Ruby/SketchUp.
#include "cabinet.hpp"
#include <cstring>

#if defined(_WIN32)
#define HSX_API extern "C" __declspec(dllexport)
#else
#define HSX_API extern "C" __attribute__((visibility("default")))
#endif

// Trả về độ dài JSON; ghi vào out nếu cap đủ (luôn kết thúc bằng '\0'). Gọi 2 lần: out=NULL để lấy kích thước.
HSX_API int hsx_solve(const char* params, char* out, int cap) {
  hsx::Params p; std::string err;
  std::string json;
  if (!hsx::parseParams(params ? params : "", p, err)) json = "{\"ok\":false,\"error\":\"" + err + "\"}";
  else json = hsx::toJson(hsx::solve(p));
  if (out && cap > (int)json.size()) std::memcpy(out, json.c_str(), json.size() + 1);
  return (int)json.size();
}

HSX_API int hsx_export_dxf(const char* params, const char* path) {
  hsx::Params p; std::string err;
  if (!hsx::parseParams(params ? params : "", p, err)) return 0;
  auto r = hsx::solve(p);
  return r.ok && hsx::exportDxf(r, path) ? 1 : 0;
}
