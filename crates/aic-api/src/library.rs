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
    /// Bộ vật liệu: thùng / cánh / hậu + chỉ dán cánh, thùng.
    #[serde(default)]
    pub material_sets: Vec<MaterialSet>,
    /// Nguồn thư viện nhóm (D31) và cách xử lý trùng tên.
    #[serde(default)]
    pub sources: Vec<crate::library_sources::LibrarySource>,
    #[serde(default)]
    pub conflict: crate::library_sources::Conflict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialSet {
    pub name: String,
    pub carcass: String,
    pub front: String,
    pub back: String,
    /// Mã chỉ cánh / mặt ngăn (None = giữ nguyên).
    #[serde(default)]
    pub edge_front: Option<String>,
    /// Mã chỉ thùng + kệ (None = giữ nguyên).
    #[serde(default)]
    pub edge_carcass: Option<String>,
}

/// Bộ vật liệu dựng sẵn (luôn có, không lưu vào thư viện).
pub fn builtin_material_sets() -> Vec<MaterialSet> {
    let set = |name: &str, carcass: &str, front: &str, back: &str, ef: &str, ec: &str| MaterialSet {
        name: name.into(),
        carcass: carcass.into(),
        front: front.into(),
        back: back.into(),
        edge_front: Some(ef.into()),
        edge_carcass: Some(ec.into()),
    };
    vec![
        set("Bếp chống ẩm (MFC lõi xanh + Acrylic)", "MFCMR18-WHITE", "ACR18-WHITE", "HDFMR8-WHITE", "ABS-1", "PVC-1"),
        set("Tủ áo MFC vân sồi", "MFC17-WHITE", "MFC18-OAK", "MDF8-WHITE", "PVC-1", "PVC-1"),
        set("Cao cấp Veneer tần bì", "MDF18-WHITE", "VEN18-ASH", "MDF8-WHITE", "ABS-2", "ABS-1"),
        set("Tiết kiệm MDF trắng", "MDF17-WHITE", "MDF17-WHITE", "MDF8-WHITE", "DON-1", "DON-1"),
    ]
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
        self.merged = Default::default();
        self.merge_sources();
    }

    pub(crate) fn save_library_pub(&self) -> Result<(), CoreError> {
        self.save_library()
    }

    fn save_library(&self) -> Result<(), CoreError> {
        let Some(p) = &self.library_path else { return Ok(()) };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| bad("library", e.to_string()))?;
        }
        // Chỉ mục của máy (mục từ nguồn nhóm không chép vào file máy).
        let s = serde_json::to_string_pretty(&self.local_library_value()?).map_err(|e| bad("library", e.to_string()))?;
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

impl Engine {
    fn material_set(&self, name: &str) -> Option<MaterialSet> {
        self.library.material_sets.iter().find(|m| m.name == name).cloned().or_else(|| builtin_material_sets().into_iter().find(|m| m.name == name))
    }

    pub(crate) fn material_sets_info(&self) -> Value {
        let builtin: Vec<Value> = builtin_material_sets().into_iter().map(|m| json!({ "set": m, "builtin": true })).collect();
        let user: Vec<Value> = self.library.material_sets.iter().map(|m| json!({ "set": m, "builtin": false })).collect();
        json!({ "sets": builtin.into_iter().chain(user).collect::<Vec<_>>() })
    }

    /// Lưu bộ vật liệu từ vật liệu thùng / cánh / hậu + chỉ dán hiện tại của một tủ.
    pub(crate) fn save_material_set(&mut self, cab: ObjectId, name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() || builtin_material_sets().iter().any(|m| m.name == name) {
            return Err(bad("preset", "name required / reserved"));
        }
        let def = self.cabinet_def(cab)?;
        let code = |g: &str| def.edge_rule.groups.get(g).map(|x| x.band_code.clone()).or_else(|| Some(def.edge_rule.band_code.clone()));
        let set = MaterialSet {
            name: name.into(),
            carcass: def.carcass_material.0.clone(),
            front: def.front_material.0.clone(),
            back: def.back_material.0.clone(),
            edge_front: code("front"),
            edge_carcass: code("carcass"),
        };
        self.library.material_sets.retain(|m| m.name != name);
        self.library.material_sets.push(set);
        self.save_library_pub()
    }

    /// Áp bộ vật liệu cho các tủ (`ids`) hoặc mọi tủ của một phòng: một bước undo.
    pub(crate) fn apply_material_set(&mut self, ids: Vec<ObjectId>, room: Option<String>, name: &str) -> Result<usize, CoreError> {
        let set = self.material_set(name).ok_or_else(|| bad("preset", "unknown material set"))?;
        for m in [&set.carcass, &set.front, &set.back] {
            if !self.doc.materials.iter().any(|x| &x.id.0 == m) {
                return Err(CoreError::UnknownMaterial { material: m.clone() });
            }
        }
        let mut cabs: Vec<ObjectId> = ids.into_iter().filter(|i| self.doc.objects.get(i).is_some_and(|o| o.as_cabinet().is_some())).collect();
        if let Some(room) = room {
            cabs.extend(self.doc.objects.iter().filter_map(|(id, o)| o.as_cabinet().filter(|c| c.room == room).map(|_| *id)));
        }
        cabs.sort();
        cabs.dedup();
        if cabs.is_empty() {
            return Err(bad("preset", "no cabinet"));
        }
        let mark = self.history.mark();
        let res = (|| -> Result<(), CoreError> {
            for &c in &cabs {
                self.set_material(c, &set.carcass, Some("carcass"))?;
                self.set_material(c, &set.front, Some("front"))?;
                self.set_material(c, &set.back, Some("back"))?;
                if let Some(e) = &set.edge_front {
                    self.set_parameter_pub(c, "edge_g_front_code", e)?;
                }
                if let Some(e) = &set.edge_carcass {
                    self.set_parameter_pub(c, "edge_g_carcass_code", e)?;
                    self.set_parameter_pub(c, "edge_g_shelf_code", e)?;
                }
            }
            Ok(())
        })();
        if let Err(e) = res {
            self.history.rollback(&mut self.doc, mark);
            return Err(e);
        }
        self.history.squash(mark, "Áp bộ vật liệu");
        Ok(cabs.len())
    }

    pub(crate) fn delete_material_set(&mut self, name: &str) -> Result<(), CoreError> {
        self.library.material_sets.retain(|m| m.name != name);
        self.save_library_pub()
    }
}
