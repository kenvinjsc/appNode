//! Thư viện nhóm (D31): ngoài thư viện máy (`~/.aic-cad/library.json`) còn các nguồn "Công ty"
//! (thư mục mạng / thư mục đồng bộ chứa `library.json`). Mục của nguồn được hợp nhất vào thư
//! viện đang dùng; trùng tên thì giữ bản máy (KEEP_LOCAL) hoặc dùng bản nguồn (USE_REMOTE).
//! Thư viện máy chỉ lưu mục của máy (không chép mục nguồn vào). Đẩy mục lên nguồn ghi được.
//! Ngoài dự án, không phải Command (không undo).

use crate::library::Library;
use crate::zones::bad;
use crate::Engine;
use aic_project::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibrarySource {
    pub name: String,
    /// Thư mục chứa `library.json` hoặc đường dẫn file.
    pub path: String,
    /// Chỉ đọc (thợ): không đẩy mục lên được.
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Conflict {
    #[default]
    KeepLocal,
    UseRemote,
}

/// Các danh sách mục trong thư viện (khóa = group? + name).
const KINDS: [&str; 5] = ["groups", "templates", "presets", "zones", "material_sets"];

fn key(kind: &str, item: &Value) -> String {
    let name = item["name"].as_str().unwrap_or("");
    if kind == "groups" { format!("{}/{name}", item["group"].as_str().unwrap_or("")) } else { name.to_string() }
}

fn file_of(src: &LibrarySource) -> PathBuf {
    let p = PathBuf::from(&src.path);
    if p.extension().is_some_and(|e| e == "json") { p } else { p.join("library.json") }
}

fn read_lib(src: &LibrarySource) -> Result<Library, String> {
    let f = file_of(src);
    match std::fs::read_to_string(&f) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("{}: {e}", f.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Library::default()),
        Err(e) => Err(format!("{}: {e}", f.display())),
    }
}

/// Trạng thái hợp nhất: mục từ nguồn (để không lưu vào thư viện máy) và bản máy bị thay.
#[derive(Debug, Clone, Default)]
pub struct Merged {
    /// (kind, key) → (nguồn, bản nguồn).
    pub remote: Vec<(String, String, String, Value)>,
    /// (kind, key) → bản máy bị USE_REMOTE thay (khôi phục khi lưu).
    pub shadowed: Vec<(String, String, Value)>,
    pub errors: Vec<String>,
    /// Số mục có trong từng nguồn.
    pub counts: Vec<(String, usize)>,
}

impl Engine {
    /// Đọc lại mọi nguồn và hợp nhất vào `self.library` (thư viện máy vừa nạp).
    pub(crate) fn merge_sources(&mut self) {
        let mut merged = Merged::default();
        let Ok(mut lib) = serde_json::to_value(&self.library) else { return };
        for src in self.library.sources.clone() {
            let remote = match read_lib(&src) {
                Ok(r) => r,
                Err(e) => {
                    merged.errors.push(e);
                    continue;
                }
            };
            let Ok(rv) = serde_json::to_value(&remote) else { continue };
            merged.counts.push((src.name.clone(), KINDS.iter().map(|k| rv[*k].as_array().map_or(0, |a| a.len())).sum()));
            for kind in KINDS {
                let Some(items) = rv[kind].as_array() else { continue };
                for item in items {
                    let k = key(kind, item);
                    let list = lib[kind].as_array_mut().expect("library list");
                    // Nguồn trước đã góp mục này: nguồn đầu tiên thắng.
                    if merged.remote.iter().any(|r| r.0 == kind && r.1 == k) {
                        continue;
                    }
                    match list.iter().position(|x| key(kind, x) == k) {
                        None => list.push(item.clone()),
                        Some(i) if self.library.conflict == Conflict::UseRemote => {
                            merged.shadowed.push((kind.into(), k.clone(), list[i].clone()));
                            list[i] = item.clone();
                        }
                        Some(_) => continue,
                    }
                    merged.remote.push((kind.into(), k, src.name.clone(), item.clone()));
                }
            }
        }
        if let Ok(l) = serde_json::from_value(lib) {
            self.library = l;
        }
        self.merged = merged;
    }

    /// Thư viện để ghi file máy: bỏ mục nguồn chưa sửa, trả lại bản máy bị thay.
    pub(crate) fn local_library_value(&self) -> Result<Value, CoreError> {
        let mut lib = serde_json::to_value(&self.library).map_err(|e| bad("library", e.to_string()))?;
        for kind in KINDS {
            let Some(list) = lib[kind].as_array_mut() else { continue };
            let mut out = Vec::new();
            for item in list.drain(..) {
                let k = key(kind, &item);
                let from_remote = self.merged.remote.iter().any(|r| r.0 == kind && r.1 == k && r.3 == item);
                if !from_remote {
                    out.push(item);
                } else if let Some(s) = self.merged.shadowed.iter().find(|s| s.0 == kind && s.1 == k) {
                    out.push(s.2.clone());
                }
            }
            *list = out;
        }
        Ok(lib)
    }

    pub(crate) fn library_sources_info(&self) -> Value {
        let count = |name: &str| self.merged.counts.iter().find(|c| c.0 == name).map_or(0, |c| c.1);
        json!({
            "sources": self.library.sources.iter().map(|s| json!({ "name": s.name, "path": s.path, "readonly": s.readonly, "items": count(&s.name) })).collect::<Vec<_>>(),
            "conflict": self.library.conflict,
            "errors": self.merged.errors,
            "remote_items": self.merged.remote.iter().map(|r| json!({ "kind": r.0, "key": r.1, "source": r.2 })).collect::<Vec<_>>(),
        })
    }

    pub(crate) fn set_library_sources(&mut self, sources: Vec<LibrarySource>, conflict: Conflict) -> Result<Value, CoreError> {
        if sources.iter().any(|s| s.name.trim().is_empty() || s.path.trim().is_empty()) {
            return Err(bad("sources", "name / path required"));
        }
        // Lưu thư viện máy với cấu hình nguồn mới, rồi nạp lại và hợp nhất.
        let mut local = self.local_library_value()?;
        local["sources"] = serde_json::to_value(&sources).map_err(|e| bad("sources", e.to_string()))?;
        local["conflict"] = serde_json::to_value(conflict).map_err(|e| bad("conflict", e.to_string()))?;
        self.library = serde_json::from_value(local).map_err(|e| bad("library", e.to_string()))?;
        self.merged = Merged::default();
        self.save_library_pub()?;
        self.merge_sources();
        Ok(self.library_sources_info())
    }

    /// Nạp lại các nguồn (lấy mục máy khác vừa đẩy lên).
    pub(crate) fn reload_library(&mut self) -> Result<Value, CoreError> {
        let local = self.local_library_value()?;
        self.library = serde_json::from_value(local).map_err(|e| bad("library", e.to_string()))?;
        self.merged = Merged::default();
        self.merge_sources();
        Ok(self.library_sources_info())
    }

    /// Đẩy một mục thư viện (chuẩn xưởng, template, mẫu vùng, bộ vật liệu …) lên nguồn ghi được.
    pub(crate) fn publish_library_item(&mut self, source: &str, kind: &str, name: &str, group: Option<&str>) -> Result<Value, CoreError> {
        if !KINDS.contains(&kind) {
            return Err(bad("kind", "groups | templates | presets | zones | material_sets"));
        }
        let src = self.library.sources.iter().find(|s| s.name == source).cloned().ok_or_else(|| bad("source", "unknown source"))?;
        if src.readonly {
            return Err(CoreError::ConstraintViolated { constraint: "LIBRARY_READONLY".into(), message: source.into() });
        }
        let lib = serde_json::to_value(&self.library).map_err(|e| bad("library", e.to_string()))?;
        let want = if kind == "groups" { format!("{}/{name}", group.unwrap_or("")) } else { name.to_string() };
        let item = lib[kind].as_array().and_then(|l| l.iter().find(|x| key(kind, x) == want)).cloned().ok_or_else(|| bad("name", "unknown item"))?;
        let remote = read_lib(&src).map_err(|e| CoreError::ConstraintViolated { constraint: "LIBRARY_SOURCE".into(), message: e })?;
        let mut rv = serde_json::to_value(&remote).map_err(|e| bad("library", e.to_string()))?;
        let list = rv[kind].as_array_mut().expect("library list");
        list.retain(|x| key(kind, x) != want);
        list.push(item);
        let f = file_of(&src);
        let err = |e: std::io::Error| CoreError::ConstraintViolated { constraint: "LIBRARY_SOURCE".into(), message: format!("{}: {e}", f.display()) };
        if let Some(dir) = f.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        std::fs::write(&f, serde_json::to_string_pretty(&rv).unwrap_or_default()).map_err(err)?;
        self.reload_library()
    }
}
