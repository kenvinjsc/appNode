//! Bảng "Thuộc tính kết cấu": tabs of construction options for one cabinet, built by
//! the core (the UI only renders fields), and per-tab presets ("mẫu nhỏ") kept in
//! the shared library so a set of options can be reused on any cabinet / project.

use crate::library::GroupPreset;
use crate::zones::bad;
use crate::Engine;
use aic_domain::ObjectId;
use aic_project::CoreError;
use serde_json::{json, Map, Value};

fn num(key: &str, label: &str, v: f64) -> Value {
    json!({ "key": key, "label": label, "kind": "number", "value": v })
}
fn flag(key: &str, label: &str, v: bool) -> Value {
    json!({ "key": key, "label": label, "kind": "bool", "value": v })
}
fn text(key: &str, label: &str, v: &str, hint: &str) -> Value {
    json!({ "key": key, "label": label, "kind": "text", "value": v, "hint": hint })
}
fn section(label: &str) -> Value {
    json!({ "key": "", "label": label, "kind": "section" })
}
fn select(key: &str, label: &str, v: &str, opts: &[(&str, &str)]) -> Value {
    json!({ "key": key, "label": label, "kind": "select", "value": v, "options": opts.iter().map(|(v, l)| json!({ "value": v, "label": l })).collect::<Vec<_>>() })
}

impl Engine {
    /// Tabs (key, title, fields) of the construction options of a cabinet.
    pub(crate) fn structure_tabs(&self, cab: ObjectId) -> Result<Vec<(String, String, Vec<Value>)>, CoreError> {
        let def = self.cabinet_def(cab)?;
        let p = |n: &str| self.doc.param_value(cab, n).unwrap_or(0.0);
        let r = &def.rules;
        let st = |s: aic_domain::JoinStyle| format!("{s:?}").to_uppercase();
        let rail = |rs: &aic_domain::structure::RailSet| if rs.size > 0.0 { rs.size } else { p("rail_width") };
        let tr = &r.top_rails;
        Ok(vec![
            ("general".into(), "Thông số chung".into(), vec![num("width", "Rộng", p("width")), num("height", "Cao", p("height")), num("depth", "Sâu", p("depth")), num("thickness", "Dày ván", p("thickness"))]),
            (
                "joints".into(),
                "Liên kết".into(),
                vec![
                    select("top_style", "Nóc", &st(def.top_style), &[("OVERLAY", "Nóc phủ hồi"), ("INSET", "Nóc lọt giữa hồi"), ("RAILS", "Thanh giằng")]),
                    select("bottom_style", "Đáy", &st(def.bottom_style), &[("INSET", "Đáy lọt giữa hồi"), ("OVERLAY", "Đáy phủ hồi")]),
                    flag("back_panel", "Có tấm hậu", def.back_panel),
                ],
            ),
            (
                "back".into(),
                "Hậu".into(),
                vec![
                    num("back_groove", "Độ sâu rãnh hậu (C)", p("back_groove")),
                    num("back_thickness", "Độ dày tấm hậu (B)", p("back_thickness")),
                    num("back_offset", "Lùi hậu (I)", p("back_offset")),
                    num("back_clearance", "Khe hở", r.back.clearance),
                    num("back_gap_left", "Hở trái", r.back.gaps[0]),
                    num("back_gap_right", "Hở phải", r.back.gaps[1]),
                    num("back_gap_top", "Hở trên", r.back.gaps[2]),
                    num("back_gap_bottom", "Hở dưới", r.back.gaps[3]),
                    flag("back_split", "Chia dọc", r.back.split),
                    text("back_split_formula", "Công thức chia", &r.back.split_formula, "600 = mỗi tấm ≤ 600 mm · 3x = chia 3"),
                    flag("top_covers_back", "Nóc trùm hậu", r.back.top_covers == Some(true)),
                    flag("bottom_covers_back", "Đáy trùm hậu", r.back.bottom_covers == Some(true)),
                ],
            ),
            (
                "top_rails".into(),
                "Thanh giằng (trên)".into(),
                vec![
                    section("Thanh bạ nóc trên phía trước"),
                    num("rt_front_count", "Số thanh", tr.front.count as f64),
                    num("rt_front_size", "Kích thước", rail(&tr.front)),
                    flag("rt_front_horizontal", "Ngang", tr.front.horizontal),
                    num("rt_front_offset", "Âm mặt", tr.front.offset),
                    section("Thanh bạ nóc trên phía sau"),
                    num("rt_back_count", "Số thanh", tr.back.count as f64),
                    num("rt_back_size", "Kích thước", rail(&tr.back)),
                    flag("rt_back_horizontal", "Ngang", tr.back.horizontal),
                    num("rt_back_offset", "Cách hậu", tr.back.offset),
                    section("Thanh bạ nóc trên bổ sung"),
                    num("rt_extra_count", "Số lượng", tr.extra.count as f64),
                    num("rt_extra_size", "Kích thước", rail(&tr.extra)),
                    flag("rt_extra_horizontal", "Ngang", tr.extra.horizontal),
                ],
            ),
            ("plinth".into(), "Len chân".into(), vec![num("plinth_height", "Cao chân", p("plinth_height")), num("plinth_setback", "Chân giật vào", r.plinth_setback)]),
            ("shelves".into(), "Lùi đợt".into(), vec![num("shelf_setback", "Kệ di động lùi trước", p("shelf_setback"))]),
        ])
    }

    pub(crate) fn get_structure(&self, cab: ObjectId) -> Result<Value, CoreError> {
        let def = self.cabinet_def(cab)?;
        let tabs: Vec<Value> = self
            .structure_tabs(cab)?
            .into_iter()
            .map(|(k, title, fields)| {
                let presets: Vec<&str> = self.library.groups.iter().filter(|g| g.group == k).map(|g| g.name.as_str()).collect();
                json!({ "key": k, "title": title, "fields": fields, "presets": presets })
            })
            .collect();
        let rails_active = def.top_style == aic_domain::JoinStyle::Rails;
        Ok(json!({ "cabinet": cab, "name": def.name, "tabs": tabs, "rails_active": rails_active }))
    }

    /// Lưu mẫu tab: the tab's current values under `name` in the shared library.
    pub(crate) fn save_group_preset(&mut self, cab: ObjectId, group: &str, name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(bad("template", "name required"));
        }
        let (_, _, fields) = self.structure_tabs(cab)?.into_iter().find(|t| t.0 == group).ok_or_else(|| bad("group", "unknown tab"))?;
        let values: Map<String, Value> = fields.into_iter().filter(|f| f["kind"] != "section").map(|f| (f["key"].as_str().unwrap_or("").to_string(), f["value"].clone())).collect();
        self.library.groups.retain(|g| !(g.group == group && g.name == name));
        self.library.groups.push(GroupPreset { group: group.into(), name: name.into(), values });
        self.save_library_pub()
    }

    /// Áp mẫu tab lên các tủ: one undo step, all or nothing.
    pub(crate) fn apply_group_preset(&mut self, ids: &[ObjectId], group: &str, name: &str) -> Result<(), CoreError> {
        let g = self.library.groups.iter().find(|g| g.group == group && g.name == name).cloned().ok_or_else(|| bad("group", "unknown preset"))?;
        let mark = self.history.mark();
        for &id in ids {
            for (k, v) in &g.values {
                let s = match v {
                    Value::Bool(b) => if *b { "on".to_string() } else { "off".to_string() },
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    _ => continue,
                };
                if let Err(e) = self.set_parameter_pub(id, k, &s) {
                    self.history.rollback(&mut self.doc, mark);
                    return Err(e);
                }
            }
        }
        self.history.squash(mark, "Áp mẫu kết cấu");
        Ok(())
    }

    pub(crate) fn delete_group_preset(&mut self, group: &str, name: &str) -> Result<(), CoreError> {
        self.library.groups.retain(|g| !(g.group == group && g.name == name));
        self.save_library_pub()
    }
}
