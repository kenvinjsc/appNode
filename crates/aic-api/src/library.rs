//! Thư viện mẫu dùng chung mọi dự án: template tủ, rule preset và mẫu vùng
//! (nội dung một khoang). Stored as a JSON file chosen by the host (dev server /
//! desktop shell); without a path it lives in memory for the session.

use crate::zones::bad;
use crate::Engine;
use aic_domain::zone::{Front, Uid, Zone};
use aic_domain::ObjectId;
use aic_project::{CabinetTemplate, CoreError, RulePreset};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Mẫu vùng: the content of one zone (splits with bays, fronts, links).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZonePreset {
    pub name: String,
    pub zone: Zone,
    /// Size of the zone it was saved from (W, H, D), for information.
    pub size: [f64; 3],
}

/// Mẫu một tab của bảng Thuộc tính kết cấu (hậu, giằng, len chân …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupPreset {
    pub group: String,
    pub name: String,
    pub values: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub groups: Vec<GroupPreset>,
    #[serde(default)]
    pub templates: Vec<CabinetTemplate>,
    #[serde(default)]
    pub presets: Vec<RulePreset>,
    #[serde(default)]
    pub zones: Vec<ZonePreset>,
}

/// Default library file: `$AIC_LIBRARY`, else `~/.aic-cad/library.json`.
pub fn default_library_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AIC_LIBRARY") {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()?;
    Some(PathBuf::from(home).join(".aic-cad").join("library.json"))
}

fn upsert<T>(list: &mut Vec<T>, item: T, name: impl Fn(&T) -> &str) {
    let n = name(&item).to_string();
    list.retain(|x| name(x) != n);
    list.push(item);
}

/// Short Vietnamese summary of a zone's content ("2 vách · 3 ngăn kéo · cánh").
pub fn zone_summary(z: &Zone) -> String {
    let (mut splits, mut doors, mut drawers, mut links) = (0usize, 0usize, 0u32, 0usize);
    fn walk(z: &Zone, s: &mut usize, d: &mut usize, w: &mut u32, l: &mut usize) {
        if let Some(sp) = &z.split {
            *s += sp.panels.len();
            for c in &sp.children {
                walk(c, s, d, w, l);
            }
        }
        match &z.front {
            Some(Front::Doors(_)) => *d += 1,
            Some(Front::Drawers(x)) => *w += x.count * x.cols.max(1),
            None => {}
        }
        *l += z.links.len();
    }
    walk(z, &mut splits, &mut doors, &mut drawers, &mut links);
    let mut parts = Vec::new();
    if splits > 0 {
        parts.push(format!("{splits} tấm chia"));
    }
    if doors > 0 {
        parts.push(format!("{doors} bộ cánh"));
    }
    if drawers > 0 {
        parts.push(format!("{drawers} ngăn kéo"));
    }
    if links > 0 {
        parts.push(format!("{links} thanh treo"));
    }
    if parts.is_empty() {
        "trống".into()
    } else {
        parts.join(" · ")
    }
}

impl Engine {
    /// Use a library file (loaded now if it exists; written on every change).
    pub fn set_library_path(&mut self, path: Option<PathBuf>) {
        self.library = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        self.library_path = path;
    }

    pub(crate) fn save_library_pub(&self) -> Result<(), CoreError> {
        self.save_library()
    }

    fn save_library(&self) -> Result<(), CoreError> {
        let Some(p) = &self.library_path else { return Ok(()) };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| bad("library", e.to_string()))?;
        }
        let s = serde_json::to_string_pretty(&self.library).map_err(|e| bad("library", e.to_string()))?;
        std::fs::write(p, s).map_err(|e| bad("library", e.to_string()))
    }

    pub(crate) fn library_add_template(&mut self, t: CabinetTemplate) -> Result<(), CoreError> {
        upsert(&mut self.library.templates, t, |x| &x.name);
        self.save_library()
    }

    pub(crate) fn library_add_preset(&mut self, p: RulePreset) -> Result<(), CoreError> {
        upsert(&mut self.library.presets, p, |x| &x.name);
        self.save_library()
    }

    /// Remove a library entry; `kind` = template | preset | zone. Returns whether it existed.
    pub(crate) fn library_remove(&mut self, kind: &str, name: &str) -> Result<bool, CoreError> {
        let before = self.library.templates.len() + self.library.presets.len() + self.library.zones.len();
        match kind {
            "template" => self.library.templates.retain(|t| t.name != name),
            "preset" => self.library.presets.retain(|t| t.name != name),
            _ => self.library.zones.retain(|t| t.name != name),
        }
        let removed = before != self.library.templates.len() + self.library.presets.len() + self.library.zones.len();
        if removed {
            self.save_library()?;
        }
        Ok(removed)
    }

    /// Lưu mẫu vùng: the content of a zone, reusable in any cabinet / project.
    pub(crate) fn save_zone_preset(&mut self, cab: ObjectId, zone: Uid, name: &str) -> Result<Value, CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(bad("template", "name required"));
        }
        let def = self.cabinet_def(cab)?;
        let z = def.zones.zone(zone).cloned().ok_or_else(|| bad("zone", "zone not found"))?;
        if z.split.is_none() && z.front.is_none() && z.links.is_empty() {
            return Err(bad("zone_preset", "empty zone"));
        }
        let size = self.doc.cabinet_layout(cab).and_then(|l| l.zones.iter().find(|b| b.id == zone).map(|b| b.size)).unwrap_or([0.0; 3]);
        upsert(&mut self.library.zones, ZonePreset { name: name.into(), zone: z, size }, |x| &x.name);
        self.save_library()?;
        Ok(json!({ "name": name }))
    }

    /// Áp mẫu vùng vào các vùng đang ghim (thay nội dung cũ). One undo step; refused
    /// when the content does not fit (a LOCK bay larger than the zone…).
    pub(crate) fn apply_zone_preset(&mut self, cab: ObjectId, zones: &[Uid], name: &str) -> Result<(), CoreError> {
        let p = self.library.zones.iter().find(|z| z.name == name).cloned().ok_or_else(|| bad("zone_preset", "unknown preset"))?;
        if zones.is_empty() {
            return Err(bad("zone", "pin a zone first"));
        }
        self.edit_cabinet_checked(cab, "Áp mẫu vùng", |c| {
            for z in zones {
                c.zones.graft(*z, &p.zone).map_err(|e| bad("zone", e))?;
            }
            Ok(())
        })
    }

    pub(crate) fn library_info(&self) -> Value {
        json!({
            "path": self.library_path.as_ref().map(|p| p.display().to_string()),
            "zones": self.library.zones.iter().map(|z| json!({ "name": z.name, "size": z.size, "summary": zone_summary(&z.zone) })).collect::<Vec<_>>(),
        })
    }
}
