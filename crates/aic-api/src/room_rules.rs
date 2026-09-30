//! Luật theo phòng (D33): loại phòng (Bếp / WC / Phòng ngủ / Khách / Thờ) → vật liệu, chân,
//! khoét hậu … áp khi tạo tủ trong phòng đó. Loại phòng đặt trong cài đặt dự án (undo được);
//! chưa đặt thì đoán theo tên phòng ("WC1" → WC, "Bếp" → Bếp).

use crate::Engine;
use aic_domain::{CabinetKind, ObjectId};
use aic_project::CoreError;
use serde_json::{json, Value};

pub const ROOM_TYPES: &[(&str, &str)] = &[("BEP", "Bếp"), ("WC", "WC / nhà tắm"), ("PN", "Phòng ngủ"), ("KHACH", "Phòng khách"), ("THO", "Phòng thờ"), ("KHAC", "Khác")];

/// Đoán loại phòng theo tên (không dấu, không phân biệt hoa thường).
pub fn guess(room: &str) -> Option<&'static str> {
    let r = room.to_lowercase();
    let has = |p: &str| r.contains(p);
    if has("wc") || has("tắm") || has("vệ sinh") || has("tam") && has("nha") {
        Some("WC")
    } else if has("bếp") || has("bep") || has("kitchen") {
        Some("BEP")
    } else if has("thờ") || has("tho") {
        Some("THO")
    } else if has("pn") || has("ngủ") || has("ngu") {
        Some("PN")
    } else if has("khách") || has("khach") {
        Some("KHACH")
    } else {
        None
    }
}

impl Engine {
    pub(crate) fn room_type(&self, room: &str) -> Option<String> {
        if room.trim().is_empty() {
            return None;
        }
        self.doc.settings.room_types.get(room).cloned().or_else(|| guess(room).map(String::from))
    }

    pub(crate) fn room_types_info(&self, extra: Option<&str>) -> Value {
        let mut rooms: Vec<String> = self.doc.objects.values().filter_map(|o| o.as_cabinet()).map(|c| c.room.clone()).filter(|r| !r.is_empty()).collect();
        rooms.extend(self.doc.settings.room_types.keys().cloned());
        rooms.extend(extra.map(str::trim).filter(|r| !r.is_empty()).map(String::from));
        rooms.sort();
        rooms.dedup();
        json!({
            "types": ROOM_TYPES.iter().map(|(k, l)| json!({ "value": k, "label": l })).collect::<Vec<_>>(),
            "rooms": rooms.iter().map(|r| json!({ "room": r, "type": self.room_type(r), "explicit": self.doc.settings.room_types.contains_key(r) })).collect::<Vec<_>>(),
        })
    }

    /// Luật của loại phòng cho một tủ vừa tạo; trả về danh sách luật đã áp.
    pub(crate) fn apply_room_rules(&mut self, cab: ObjectId) -> Result<Vec<&'static str>, CoreError> {
        let Some(c) = self.doc.objects.get(&cab).and_then(|o| o.as_cabinet()).cloned() else { return Ok(Vec::new()) };
        let Some(t) = self.room_type(&c.room) else { return Ok(Vec::new()) };
        let floor_kind = matches!(c.kind, CabinetKind::Base | CabinetKind::Drawer);
        // Bếp / Thờ: chỉ khi đã đặt loại phòng (giữ vật liệu người dùng chọn cho phòng tên "Bếp").
        let explicit = self.doc.settings.room_types.contains_key(&c.room);
        let mut applied = Vec::new();
        match t.as_str() {
            "WC" => {
                self.apply_material_set(vec![cab], None, "Bếp chống ẩm (MFC lõi xanh + Acrylic)")?;
                applied.push("Vật liệu chống ẩm");
                if floor_kind {
                    self.set_parameter_pub(cab, "base_type", "LEGS")?;
                    applied.push("Chân nhựa");
                    if c.back_panel {
                        let pipe = aic_domain::structure::BackCutout { kind: aic_domain::structure::CutoutKind::Pipe, anchor: aic_domain::structure::HAnchor::Center, x: 0.0, y: 250.0, w: 60.0, h: 60.0, r: 30.0 };
                        self.edit_cabinet(cab, "Khoét hậu", move |c| {
                            if c.rules.back.cutouts.is_empty() {
                                c.rules.back.cutouts.push(pipe);
                            }
                            Ok(())
                        })?;
                        applied.push("Khoét ống Ø60");
                    }
                }
            }
            "BEP" if explicit => {
                self.apply_material_set(vec![cab], None, "Bếp chống ẩm (MFC lõi xanh + Acrylic)")?;
                applied.push("Vật liệu chống ẩm");
            }
            "THO" if explicit => {
                self.set_parameter_pub(cab, "tr_cornice", "FRONT")?;
                applied.push("Phào nóc");
            }
            _ => {}
        }
        Ok(applied)
    }
}
