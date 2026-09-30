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

/// Nhóm mẫu "Chuẩn xưởng": giá trị của mọi tab kết cấu.
pub(crate) const SHOP_GROUP: &str = "all";

/// Đặt một field `s_*` của chuẩn xưởng (kiểu lấy theo giá trị hiện có).
pub(crate) fn set_shop_field(shop: &mut aic_domain::structure::ShopRules, key: &str, value: &str) -> Result<(), CoreError> {
    let field = key.strip_prefix("s_").ok_or_else(|| bad(key, "not a shop field"))?;
    let mut map = serde_json::to_value(&*shop).map_err(|e| bad(key, e.to_string()))?;
    let obj = map.as_object_mut().ok_or_else(|| bad(key, "shop"))?;
    let cur = obj.get(field).ok_or_else(|| bad(key, "unknown field"))?;
    let v = value.trim();
    let new = match cur {
        Value::Bool(_) => Value::Bool(matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes")),
        Value::Number(_) => {
            let n: f64 = v.replace(',', ".").parse().map_err(|_| bad(key, "number"))?;
            if !n.is_finite() || n < 0.0 {
                return Err(bad(key, "must be ≥ 0"));
            }
            json!(n)
        }
        _ => Value::String(if matches!(field, "pin_row" | "handle_pos" | "handle_type" | "slide_type" | "joint_type") { v.to_ascii_uppercase() } else { v.to_string() }),
    };
    obj.insert(field.to_string(), new);
    *shop = serde_json::from_value(map).map_err(|_| bad(key, "invalid value"))?;
    Ok(())
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
        let sh = &r.shop;
        let pin_row = if sh.pin_row == aic_domain::structure::PinRow::Row32 { "ROW_32" } else { "AT_SHELF" };
        let handle_pos = match sh.handle_pos {
            aic_domain::structure::HandlePos::Auto => "AUTO",
            aic_domain::structure::HandlePos::Center => "CENTER",
            aic_domain::structure::HandlePos::Top => "TOP",
            aic_domain::structure::HandlePos::Bottom => "BOTTOM",
        };
        let handle_type = match sh.handle_type {
            aic_domain::structure::HandleType::Bar => "BAR",
            aic_domain::structure::HandleType::Knob => "KNOB",
            aic_domain::structure::HandleType::PushOpen => "PUSH_OPEN",
            aic_domain::structure::HandleType::None => "NONE",
        };
        let joint_type = match sh.joint_type {
            aic_domain::structure::JointType::Dowel => "DOWEL",
            aic_domain::structure::JointType::CamDowel => "CAM_DOWEL",
            aic_domain::structure::JointType::Screw => "SCREW",
            aic_domain::structure::JointType::Bracket => "BRACKET",
        };
        let slide_type = match sh.slide_type {
            aic_domain::structure::SlideType::Ball => "BALL",
            aic_domain::structure::SlideType::Undermount => "UNDERMOUNT",
            aic_domain::structure::SlideType::Tandem => "TANDEM",
        };
        Ok(vec![
            ("general".into(), "Thông số chung".into(), vec![num("width", "Rộng", p("width")), num("height", "Cao", p("height")), num("depth", "Sâu", p("depth")), num("thickness", "Dày ván", p("thickness"))]),
            (
                "joints".into(),
                "Liên kết".into(),
                vec![
                    select("top_style", "Nóc", &st(def.top_style), &[("OVERLAY", "Nóc phủ hồi"), ("INSET", "Nóc lọt giữa hồi"), ("RAILS", "Thanh giằng")]),
                    select("bottom_style", "Đáy", &st(def.bottom_style), &[("INSET", "Đáy lọt giữa hồi"), ("OVERLAY", "Đáy phủ hồi")]),
                    flag("back_panel", "Có tấm hậu", def.back_panel),
                    section("Liên kết thùng (hồi ↔ nóc / đáy / vách)"),
                    select("s_joint_type", "Kiểu liên kết", joint_type, &[("DOWEL", "Chốt gỗ"), ("CAM_DOWEL", "Cam (minifix) + chốt gỗ"), ("SCREW", "Vít xuyên"), ("BRACKET", "Ke góc")]),
                    num("s_joint_end", "Lỗ đầu cách mép", sh.joint_end),
                    num("s_joint_pitch", "Khoảng cách lỗ tối đa", sh.joint_pitch),
                    num("s_dowel_d", "Đường kính chốt gỗ", sh.dowel_d),
                    num("s_dowel_face_depth", "Sâu lỗ mặt", sh.dowel_face_depth),
                    num("s_dowel_edge_depth", "Sâu lỗ cạnh", sh.dowel_edge_depth),
                    num("s_cam_d", "Đường kính cam", sh.cam_d),
                    num("s_cam_depth", "Sâu lỗ cam", sh.cam_depth),
                    num("s_cam_offset", "Tâm cam cách mặt hồi", sh.cam_offset),
                    num("s_bolt_d", "Đường kính lỗ chốt cam (cạnh)", sh.bolt_d),
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
            (
                "shelves".into(),
                "Kệ & chốt tầng".into(),
                vec![
                    num("shelf_setback", "Kệ di động lùi trước", p("shelf_setback")),
                    num("s_shelf_clear", "Hở kệ mỗi bên", sh.shelf_clear),
                    select("s_pin_row", "Lỗ chốt tầng", pin_row, &[("AT_SHELF", "Chỉ tại vị trí kệ"), ("ROW_32", "Hàng lỗ hệ 32")]),
                    num("s_pin_edge", "Lỗ cách mép trước kệ", sh.pin_edge),
                    num("s_pin_edge_back", "Lỗ cách mép sau", sh.pin_edge_back),
                    num("s_pin_below", "Lỗ dưới mặt kệ", sh.pin_below),
                    num("s_pin_d", "Đường kính lỗ", sh.pin_d),
                    num("s_pin_depth", "Sâu lỗ", sh.pin_depth),
                    section("Hàng lỗ hệ 32"),
                    num("s_pin_pitch", "Bước lỗ", sh.pin_pitch),
                    num("s_pin_start", "Lỗ đầu cách đáy khoang", sh.pin_start),
                    num("s_pin_end", "Lỗ cuối cách nóc khoang", sh.pin_end),
                    flag("s_pin_snap", "Kéo kệ bắt vào lỗ", sh.pin_snap),
                ],
            ),
            (
                "doors".into(),
                "Cánh & tay nắm".into(),
                vec![
                    text("s_hinge_table", "Số bản lề theo cao cánh", &sh.hinge_table, "900=2, 1600=3, 4 → ≤900: 2 · ≤1600: 3 · còn lại: 4"),
                    num("s_hinge_edge", "Tâm chén cách mép cánh", sh.hinge_edge),
                    num("s_hinge_end", "Chén đầu cách đầu cánh", sh.hinge_end),
                    num("s_cup_d", "Đường kính chén", sh.cup_d),
                    num("s_cup_depth", "Sâu chén", sh.cup_depth),
                    flag("s_hinge_plate", "Khoan đế bản lề trên hồi", sh.hinge_plate),
                    section("Tay nắm"),
                    select("s_handle_type", "Loại", handle_type, &[("BAR", "Tay nắm thanh"), ("KNOB", "Núm"), ("PUSH_OPEN", "Nhấn mở (push-open)"), ("NONE", "Không tay nắm")]),
                    num("s_handle_len", "Dài tay nắm", sh.handle_len),
                    num("s_handle_pitch", "Bước lỗ", sh.handle_pitch),
                    flag("s_handle_drill", "Khoan lỗ tay nắm", sh.handle_drill),
                    select("s_handle_pos", "Vị trí trên cánh", handle_pos, &[("AUTO", "Theo loại tủ (bếp dưới: trên, bếp trên: dưới)"), ("CENTER", "Giữa"), ("TOP", "Trên"), ("BOTTOM", "Dưới")]),
                    num("s_handle_from_end", "Cách đầu cánh (trên / dưới)", sh.handle_from_end),
                    num("s_handle_edge", "Cách mép mở", sh.handle_edge),
                    section("Cửa lùa"),
                    num("s_slide_overlap", "Chồng cánh lùa", sh.slide_overlap),
                ],
            ),
            (
                "drawers".into(),
                "Ngăn kéo".into(),
                vec![
                    select("s_slide_type", "Loại ray", slide_type, &[("BALL", "Ray bi 3 tầng"), ("UNDERMOUNT", "Ray âm giảm chấn"), ("TANDEM", "Hộp kim loại (tandem)")]),
                    num("s_box_top_gap", "Hộc thấp hơn ô (trên)", sh.box_top_gap),
                    num("s_box_bottom_gap", "Đáy hộc cách đáy ô", sh.box_bottom_gap),
                    num("s_box_min", "Cao hộc tối thiểu", sh.box_min),
                    num("s_box_max", "Cao hộc tối đa", sh.box_max),
                    num("s_slide_margin", "Ray ngắn hơn sâu khoang", sh.slide_margin),
                ],
            ),
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
        let standards: Vec<&str> = self.library.groups.iter().filter(|g| g.group == SHOP_GROUP).map(|g| g.name.as_str()).collect();
        Ok(json!({ "cabinet": cab, "name": def.name, "tabs": tabs, "rails_active": rails_active, "standards": standards }))
    }

    /// Lưu mẫu tab: the tab's current values under `name` in the shared library.
    pub(crate) fn save_group_preset(&mut self, cab: ObjectId, group: &str, name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(bad("template", "name required"));
        }
        // "all" = Chuẩn xưởng: every tab except the cabinet's size (thickness kept).
        let fields: Vec<Value> = if group == SHOP_GROUP {
            self.structure_tabs(cab)?
                .into_iter()
                .flat_map(|t| t.2)
                .filter(|f| !matches!(f["key"].as_str(), Some("width" | "height" | "depth")))
                .collect()
        } else {
            self.structure_tabs(cab)?.into_iter().find(|t| t.0 == group).ok_or_else(|| bad("group", "unknown tab"))?.2
        };
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
