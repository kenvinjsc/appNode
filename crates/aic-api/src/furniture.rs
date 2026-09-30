//! Sản phẩm ngoài tủ hộp (D19+): tạo, thuộc tính (tab riêng trong Thuộc tính kết cấu).
//! Hình học do `aic_domain::layout::products` dựng; ở đây chỉ là request / thuộc tính.

use crate::structure_api::{flag, num, section, select, text};
use crate::zones::bad;
use crate::{protocol, Engine, Request};
use aic_domain::product::{BedSpec, CladdingSpec, DeskSpec, Product};
use aic_domain::ObjectId;
use aic_project::CoreError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Đặt field `field` của một spec (kiểu theo giá trị hiện có; enum viết hoa).
pub(crate) fn set_json_field<T: Serialize + DeserializeOwned>(spec: &mut T, key: &str, field: &str, value: &str) -> Result<(), CoreError> {
    let mut map = serde_json::to_value(&*spec).map_err(|e| bad(key, e.to_string()))?;
    let obj = map.as_object_mut().ok_or_else(|| bad(key, "spec"))?;
    let cur = obj.get(field).ok_or_else(|| bad(key, "unknown field"))?;
    let v = value.trim();
    let new = match cur {
        Value::Bool(_) => Value::Bool(matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes")),
        Value::Number(_) => {
            let n: f64 = v.replace(',', ".").parse().map_err(|_| bad(key, "number"))?;
            if !n.is_finite() || n < 0.0 {
                return Err(bad(key, "must be ≥ 0"));
            }
            if cur.is_u64() {
                json!(n.round() as u64)
            } else {
                json!(n)
            }
        }
        _ => Value::String(v.to_ascii_uppercase()),
    };
    obj.insert(field.to_string(), new);
    *spec = serde_json::from_value(map).map_err(|_| bad(key, "invalid value"))?;
    Ok(())
}

fn enum_str<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

impl Engine {
    /// Tạo sản phẩm: một tủ nền (không hậu, không kệ / cánh) mang `rules.product`, một undo.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_furniture(&mut self, kind: &str, size: [Option<f64>; 3], position: Option<[f64; 3]>, name: Option<String>, room: Option<String>, floor: Option<String>, options: BTreeMap<String, String>) -> Result<Value, CoreError> {
        let (product, def_size, def_name) = match kind.to_ascii_uppercase().as_str() {
            "BED" => (Product::Bed(BedSpec::default()), [1600.0, 1000.0, 2000.0], "Giường"),
            "DESK" => (Product::Desk(DeskSpec::default()), [1200.0, 750.0, 600.0], "Bàn"),
            "CLADDING" => (Product::Cladding(CladdingSpec::default()), [3000.0, 2700.0, 60.0], "VáchỐp"),
            _ => return Err(bad("kind", "BED | DESK | CLADDING")),
        };
        let [w, h, d] = [0, 1, 2].map(|i| size[i].unwrap_or(def_size[i]));
        if def_name == "Giường" && !(800.0..=2200.0).contains(&w) {
            return Err(CoreError::ConstraintViolated { constraint: "BED_WIDTH".into(), message: format!("mattress width {w} ∉ [800, 2200]") });
        }
        let mark = self.history.mark();
        let res = (|| -> Result<Value, CoreError> {
            let overrides = protocol::CabinetOverrides { width: Some(w), height: Some(h), depth: Some(d), doors: Some(0), shelves: Some(0), ..Default::default() };
            let r = self.handle(Request::CreateCabinet { kind: aic_domain::CabinetKind::OpenShelf, position, parent: None, overrides, name: Some(name.unwrap_or_else(|| def_name.into())), room, floor, after: None })?;
            let cab: ObjectId = serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
            self.edit_cabinet(cab, def_name, move |c| {
                c.rules.product = Some(product);
                c.back_panel = false;
                Ok(())
            })?;
            for (k, v) in &options {
                self.set_zone_property(cab, k, v)?;
            }
            Ok(json!({ "id": cab }))
        })();
        match res {
            Ok(v) => {
                self.history.squash(mark, def_name);
                Ok(v)
            }
            Err(e) => {
                self.history.rollback(&mut self.doc, mark);
                Err(e)
            }
        }
    }

    /// Thuộc tính sản phẩm (`bed_*` …), một undo.
    pub(crate) fn set_product_property(&mut self, id: ObjectId, key: &str, value: &str) -> Result<(), CoreError> {
        let (key, value) = (key.to_string(), value.to_string());
        self.edit_cabinet_checked(id, "Thuộc tính sản phẩm", move |c| {
            match (c.rules.product.as_mut(), key.split_once('_')) {
                (Some(Product::Bed(b)), Some(("bed", f))) => {
                    set_json_field(b, &key, f, &value)?;
                    b.frame_h = b.frame_h.clamp(200.0, 700.0);
                    b.side_rail_h = b.side_rail_h.clamp(100.0, 400.0);
                    b.slat_count = b.slat_count.clamp(6, 30);
                    b.legs = if b.legs >= 6 { 6 } else { 4 };
                    b.drawer_count = b.drawer_count.clamp(1, 4);
                    Ok(())
                }
                (Some(Product::Desk(d)), Some(("desk", f))) => {
                    set_json_field(d, &key, f, &value)?;
                    d.top_t = d.top_t.clamp(12.0, 60.0);
                    d.unit_drawers = d.unit_drawers.clamp(1, 6);
                    d.hutch_shelves = d.hutch_shelves.min(6);
                    Ok(())
                }
                (Some(Product::Cladding(c)), Some(("cl", f))) => {
                    set_json_field(c, &key, f, &value)?;
                    for (name, formula) in [("cl_cols", &c.cols), ("cl_rows", &c.rows)] {
                        if !formula.trim().is_empty() {
                            aic_domain::zone::parse_split_formula(formula).map_err(|e| bad(name, e))?;
                        }
                    }
                    Ok(())
                }
                _ => Err(bad(&key, "not a property of this product")),
            }
        })
    }

    /// Tab thuộc tính của sản phẩm (đặt đầu bảng Thuộc tính kết cấu).
    pub(crate) fn product_tabs(p: &Product) -> Vec<(String, String, Vec<Value>)> {
        let sup = &[("PANEL", "Chân tấm"), ("DRAWER_UNIT", "Hộc tủ ngăn kéo"), ("LEG", "Chân sắt")];
        match p {
            Product::Cladding(c) => vec![(
                "cladding".into(),
                "Vách ốp".into(),
                vec![
                    section("Chia tấm ốp (công thức như Chia khoang)"),
                    flag("cl_boards", "Tấm ốp", c.boards),
                    text("cl_cols", "Chia cột", &c.cols, "/5 = 5 tấm đều · 600,* · 3*800"),
                    text("cl_rows", "Chia hàng", &c.rows, "trống = 1 hàng · 1200,*"),
                    select("cl_joint_type", "Mối ghép", &enum_str(&c.joint_type), &[("SHADOW_GAP", "Khe bóng"), ("V_GROOVE", "Soi V"), ("NONE", "Ghép sát")]),
                    num("cl_joint_gap", "Khe (mm)", c.joint_gap),
                    flag("cl_frame", "Khung xương sau tấm", c.frame),
                    section("Lam gỗ"),
                    flag("cl_batten", "Lam", c.batten),
                    num("cl_batten_w", "Rộng lam", c.batten_w),
                    num("cl_batten_gap", "Khe lam", c.batten_gap),
                    num("cl_batten_t", "Dày lam", c.batten_t),
                    flag("cl_batten_vertical", "Lam dọc", c.batten_vertical),
                ],
            )],
            Product::Desk(d) => vec![(
                "desk".into(),
                "Bàn".into(),
                vec![
                    section("Mặt bàn (Rộng × Sâu × Cao của sản phẩm)"),
                    num("desk_top_t", "Dày mặt bàn (17–40)", d.top_t),
                    num("desk_top_overhang", "Nhô hai bên", d.top_overhang),
                    section("Đỡ mặt bàn"),
                    select("desk_support_left", "Bên trái", &enum_str(&d.support_left), sup),
                    select("desk_support_right", "Bên phải", &enum_str(&d.support_right), sup),
                    num("desk_unit_w", "Rộng hộc (khóa)", d.unit_w),
                    num("desk_unit_drawers", "Số ngăn kéo mỗi hộc", d.unit_drawers as f64),
                    section("Yếm, hộc phím"),
                    flag("desk_modesty", "Yếm", d.modesty),
                    num("desk_modesty_h", "Cao yếm", d.modesty_h),
                    num("desk_modesty_setback", "Yếm lùi từ mép sau", d.modesty_setback),
                    flag("desk_keyboard_tray", "Hộc bàn phím", d.keyboard_tray),
                    section("Kệ trên, khoét dây, gương"),
                    num("desk_hutch_h", "Cao kệ trên (0 = không)", d.hutch_h),
                    num("desk_hutch_shelves", "Số kệ", d.hutch_shelves as f64),
                    num("desk_hutch_d", "Sâu kệ trên", d.hutch_d),
                    num("desk_cable_d", "Lỗ luồn dây Ø (0 = không)", d.cable_d),
                    num("desk_cable_x", "Tâm lỗ cách mép phải", d.cable_x),
                    num("desk_cable_y", "Tâm lỗ cách mép sau", d.cable_y),
                    num("desk_mirror_w", "Gương rộng (0 = không)", d.mirror_w),
                    num("desk_mirror_h", "Gương cao", d.mirror_h),
                ],
            )],
            Product::Bed(b) => vec![(
                "bed".into(),
                "Giường".into(),
                vec![
                    section("Kích thước nệm: Rộng × Sâu của sản phẩm (lọt nệm)"),
                    num("bed_frame_h", "Cao mặt dát (350–450)", b.frame_h),
                    section("Đầu giường (cao = Cao sản phẩm)"),
                    select("bed_headboard_style", "Kiểu", &enum_str(&b.headboard_style), &[("FLAT", "Phẳng"), ("UPHOLSTERED", "Bọc nệm"), ("SLATTED", "Nan đứng")]),
                    num("bed_headboard_t", "Dày (0 = theo ván)", b.headboard_t),
                    section("Thành & dát"),
                    num("bed_side_rail_h", "Cao vai / đuôi", b.side_rail_h),
                    num("bed_lip", "Vai cao hơn dát", b.lip),
                    select("bed_slats", "Dát", &enum_str(&b.slats), &[("SLATS", "Dát nan"), ("BOARD", "Dát tấm")]),
                    num("bed_slat_count", "Số nan", b.slat_count as f64),
                    num("bed_slat_t", "Dày dát", b.slat_t),
                    flag("bed_center_beam", "Đà giữa (rộng ≥ 1400)", b.center_beam),
                    select("bed_legs", "Số chân", &b.legs.to_string(), &[("4", "4 chân"), ("6", "6 chân")]),
                    section("Hộc kéo"),
                    select("bed_storage", "Kiểu", &enum_str(&b.storage), &[("NONE", "Không"), ("DRAWERS_2_SIDES", "Hộc kéo 2 bên"), ("DRAWERS_FOOT", "Hộc kéo đuôi"), ("GAS_LIFT", "Nâng hơi")]),
                    num("bed_drawer_count", "Số hộc mỗi bên (1–4)", b.drawer_count as f64),
                ],
            )],
        }
    }
}
