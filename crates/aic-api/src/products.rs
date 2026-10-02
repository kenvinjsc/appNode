//! Mẫu sản phẩm dựng sẵn (D30, §7 báo cáo đề xuất): mỗi mẫu dựng bằng chuỗi request có sẵn
//! (tạo tủ, chuẩn xưởng, chia khoang, cánh / ngăn kéo, bộ vật liệu) và gộp thành một bước undo.

use crate::protocol::{CabinetOverrides, Request, SplitZone, ZoneAddPanels};
use crate::Engine;
use aic_domain::zone::{DoorKind, HingeSide, Lock, Mount, SplitKind, Uid};
use aic_domain::{CabinetKind, ObjectId};
use aic_project::CoreError;
use serde_json::{json, Value};

/// Tham số hiển thị của mẫu (form chèn): `kind` = number / bool / select.
pub struct Param {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: &'static str,
    pub default: &'static str,
    pub options: &'static [(&'static str, &'static str)],
}

const fn num(key: &'static str, label: &'static str, default: &'static str) -> Param {
    Param { key, label, kind: "number", default, options: &[] }
}
const fn flag(key: &'static str, label: &'static str, default: &'static str) -> Param {
    Param { key, label, kind: "bool", default, options: &[] }
}

pub struct Product {
    pub key: &'static str,
    pub name: &'static str,
    /// Nhóm (phòng) trong lưới template.
    pub room: &'static str,
    pub summary: &'static str,
    /// W × H × D mặc định.
    pub size: [f64; 3],
    pub params: &'static [Param],
}

const CEILING: &[Param] = &[flag("ceiling", "Kịch trần", "off"), num("ceiling_h", "Cao trần", "2700"), num("ceiling_gap", "Che trần", "50")];

/// Bộ template dựng sẵn (§7 báo cáo đề xuất).
pub const PRODUCTS: &[Product] = &[
    Product { key: "KITCHEN_BASE_800", name: "Tủ bếp dưới 2 cánh 800", room: "Bếp", summary: "800 × 810 × 560 · chân nhựa 100 + len · 1 kệ · cánh đôi · MFC lõi xanh / Acrylic", size: [800.0, 810.0, 560.0], params: &[] },
    Product { key: "KITCHEN_WALL_800", name: "Tủ bếp trên 800", room: "Bếp", summary: "800 × 700 × 350 · treo tường · 1 kệ · cánh đôi · tay nắm dưới", size: [800.0, 700.0, 350.0], params: &[Param { key: "lift", label: "Cánh", kind: "select", default: "DOUBLE", options: &[("DOUBLE", "Cánh đôi"), ("LIFT_UP", "Cánh lật HK")] }] },
    Product { key: "KITCHEN_CORNER_L", name: "Tủ bếp góc L 1100", room: "Bếp", summary: "1100 × 810 × 560 · tấm mù + cánh 450 · 1 kệ mỗi khoang", size: [1100.0, 810.0, 560.0], params: &[] },
    Product { key: "KITCHEN_OVEN_TALL", name: "Tủ lò 600 kịch trần", room: "Bếp", summary: "600 × 2300 × 580 · 2 ngăn kéo dưới 720 · khoang lò 600 · cánh lật trên", size: [600.0, 2300.0, 580.0], params: &[] },
    Product { key: "WARDROBE_2D_1000", name: "Tủ áo 2 cánh 1000", room: "Phòng ngủ", summary: "1000 × 2200 × 580 · vách giữa: trái thanh treo, phải kệ hàng lỗ 32 · cánh đôi", size: [1000.0, 2200.0, 580.0], params: &[num("shelves", "Số kệ bên phải", "4")] },
    Product { key: "WARDROBE_4D_1800", name: "Tủ áo 4 cánh 1800", room: "Phòng ngủ", summary: "1800 × 2400 × 600 · 2 khoang 900 · tầng trên 400 · treo + 3 ngăn kéo trong", size: [1800.0, 2400.0, 600.0], params: CEILING },
    Product { key: "WARDROBE_3BAY_CEILING", name: "Tủ áo 3 khoang kịch trần", room: "Phòng ngủ", summary: "2400 × 2600 × 600 · tủ trên 400 · 800,*,800: treo dài, kệ + 3 ngăn kéo, treo 2 tầng", size: [2400.0, 2600.0, 600.0], params: &[flag("ceiling", "Kịch trần", "on"), num("ceiling_h", "Cao trần", "2700"), num("ceiling_gap", "Che trần", "100")] },
    Product { key: "WARDROBE_SLIDING_2000", name: "Tủ áo cánh lùa 2000", room: "Phòng ngủ", summary: "2000 × 2400 × 620 · 2 khoang: treo + kệ / 2 ngăn kéo trong · chồng 35", size: [2000.0, 2400.0, 620.0], params: &[Param { key: "leaves", label: "Số cánh lùa", kind: "select", default: "2", options: &[("2", "2 cánh"), ("3", "3 cánh")] }] },
    Product { key: "BED_1600", name: "Giường 1600 × 2000", room: "Phòng ngủ", summary: "Nệm 1600 × 2000 · dát nan 14 · đà giữa · 6 chân", size: [1600.0, 1000.0, 2000.0], params: &[] },
    Product { key: "BED_1600_DRAWERS", name: "Giường hộc kéo 1600", room: "Phòng ngủ", summary: "Nệm 1600 × 2000 · cao dát 450 · 2 hộc mỗi bên", size: [1600.0, 1000.0, 2000.0], params: &[num("drawers", "Số hộc mỗi bên", "2")] },
    Product { key: "TV_HANGING_1800", name: "Kệ TV treo 1800", room: "Phòng khách", summary: "1800 × 350 × 400 · treo · /3: 2 ngăn kéo push-open + khoang mở · khoét dây Ø60", size: [1800.0, 350.0, 400.0], params: &[] },
    Product { key: "SHOE_1200", name: "Tủ giày 1200", room: "Phòng khách", summary: "1200 × 1000 × 350 · /2 · mỗi khoang 4 kệ nghiêng 15° · khoét thông gió", size: [1200.0, 1000.0, 350.0], params: &[num("tilt", "Nghiêng kệ (°)", "15")] },
    Product { key: "DESK_1200", name: "Bàn học 1200", room: "Phòng ngủ", summary: "1200 × 750 × 600 · mặt 25 · hộc phải 3 ngăn · kệ trên 2 tầng · khoét dây", size: [1200.0, 750.0, 600.0], params: &[flag("hutch", "Kệ trên", "on")] },
    Product { key: "ISLAND_1800", name: "Bàn đảo 1800", room: "Bếp", summary: "1800 × 900 × 900 · mở 2 mặt: trước 1 cánh + 3 ngăn kéo, sau kệ mở · mặt đá nhô 300 phía ghế · ốp hông", size: [1800.0, 900.0, 900.0], params: &[num("seat", "Mặt đá nhô phía ghế", "300")] },
    Product { key: "WASHER_700", name: "Tủ máy giặt 700", room: "WC", summary: "700 × 2000 × 620 · khoang máy giặt 600 dưới · 2 kệ + cánh đôi trên · chống ẩm", size: [700.0, 2000.0, 620.0], params: &[] },
    Product { key: "ALTAR_1270", name: "Tủ thờ 1270", room: "Phòng thờ", summary: "1270 × 810 × 610 · 2 hộc kéo trên · khoang trang trí + đèn LED · phào nóc", size: [1270.0, 810.0, 610.0], params: &[] },
    Product { key: "LAVABO_800", name: "Tủ lavabo treo 800", room: "WC", summary: "800 × 500 × 480 · treo · cánh đôi + mặt ngăn giả · khoét ống Ø60 · chống ẩm", size: [800.0, 500.0, 480.0], params: &[] },
];

const KITCHEN_SET: &str = "Bếp chống ẩm (MFC lõi xanh + Acrylic)";

pub struct Place {
    pub position: Option<[f64; 3]>,
    pub room: Option<String>,
    pub floor: Option<String>,
    pub after: Option<ObjectId>,
    /// W × H × D (None = theo mẫu).
    pub size: [Option<f64>; 3],
    /// Tham số hiển thị (`Product.params`).
    pub params: std::collections::BTreeMap<String, String>,
}

impl Engine {
    pub(crate) fn products_info(&self) -> Value {
        json!({ "products": PRODUCTS.iter().map(|p| json!({
            "key": p.key, "name": p.name, "room": p.room, "summary": p.summary, "size": p.size,
            "params": p.params.iter().map(|x| json!({ "key": x.key, "label": x.label, "kind": x.kind, "default": x.default,
                "options": x.options.iter().map(|(v, l)| json!({ "value": v, "label": l })).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>() })
    }

    fn create(&mut self, kind: CabinetKind, o: CabinetOverrides, name: &str, p: &Place, room: &str) -> Result<ObjectId, CoreError> {
        let r = self.handle(Request::CreateCabinet {
            kind,
            position: p.position,
            parent: None,
            overrides: o,
            name: Some(name.into()),
            room: p.room.clone().or_else(|| Some(room.into())),
            floor: p.floor.clone(),
            after: p.after,
        })?;
        serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })
    }

    fn set(&mut self, id: ObjectId, pairs: &[(&str, &str)]) -> Result<(), CoreError> {
        for (k, v) in pairs {
            self.set_parameter_pub(id, k, v)?;
        }
        Ok(())
    }

    /// Khoang lá (chưa chia) của tủ, theo thứ tự từ dưới lên / trái sang.
    fn leaves(&self, cab: ObjectId) -> Vec<(Uid, [f64; 3])> {
        let mut v: Vec<(Uid, [f64; 3])> = self.doc.cabinet_layout(cab).map(|l| l.zones.iter().filter(|z| z.leaf).map(|z| (z.id, z.min)).collect()).unwrap_or_default();
        v.sort_by(|a, b| a.1[1].total_cmp(&b.1[1]).then(a.1[0].total_cmp(&b.1[0])));
        v
    }

    fn root_zone(&self, cab: ObjectId) -> Result<Uid, CoreError> {
        Ok(self.cabinet_def(cab)?.zones.root.id)
    }

    fn shelves(&mut self, cab: ObjectId, zones: Vec<Uid>, count: u32) -> Result<(), CoreError> {
        self.zone_add_panels(ZoneAddPanels { cabinet: cab, zones, kind: SplitKind::ShelfAdjustable, count, thickness: None, lock: Lock::Even, value: 0.0, tilt_deg: None })?;
        Ok(())
    }

    fn doors(&mut self, cab: ObjectId, zones: Vec<Uid>, kind: DoorKind, cols: u32, hinge: HingeSide) -> Result<(), CoreError> {
        self.handle(Request::ZoneAddDoors { cabinet: cab, zones, kind, cols, rows: 1, mount: Mount::Overlay, hinge, thickness: None, stop: None })?;
        Ok(())
    }

    /// Khoang con của một khoang đã chia (theo thứ tự trục chia).
    fn kids(&self, cab: ObjectId, zone: Uid) -> Result<Vec<Uid>, CoreError> {
        let def = self.cabinet_def(cab)?;
        Ok(def.zones.zone(zone).and_then(|z| z.split.as_ref()).map(|s| s.children.iter().map(|c| c.id).collect()).unwrap_or_default())
    }

    fn split(&mut self, cab: ObjectId, zone: Uid, kind: SplitKind, formula: &str, from_end: bool) -> Result<Vec<Uid>, CoreError> {
        self.split_zone(SplitZone { cabinet: cab, zone, kind, formula: formula.into(), from_end, thickness: None })?;
        self.kids(cab, zone)
    }

    fn rail(&mut self, cab: ObjectId, zone: Uid) -> Result<(), CoreError> {
        self.zone_add_link(cab, vec![zone], aic_domain::zone::LinkKind::OvalRail, 60.0, String::new())?;
        Ok(())
    }

    fn drawers(&mut self, cab: ObjectId, zone: Uid, count: u32, inner: bool) -> Result<(), CoreError> {
        self.handle(Request::ZoneAddDrawers { cabinet: cab, zones: vec![zone], count, cols: 1, mount: Mount::Overlay, thickness: None, with_box: true, inner, false_front: false })?;
        Ok(())
    }

    /// Tủ mới không khoang dựng sẵn (tủ áo mặc định có vách giữa).
    fn clear_zones(&mut self, cab: ObjectId) -> Result<Uid, CoreError> {
        self.edit_cabinet(cab, "Xóa khoang", |c| {
            c.zones = Default::default();
            Ok(())
        })?;
        self.root_zone(cab)
    }

    fn build_product(&mut self, key: &str, p: &Place) -> Result<ObjectId, CoreError> {
        let def = PRODUCTS.iter().find(|x| x.key == key).ok_or_else(|| crate::zones::bad("product", "unknown product"))?;
        let [w, h, d] = [0, 1, 2].map(|i| p.size[i].unwrap_or(def.size[i]));
        let param = |k: &str| -> String { p.params.get(k).cloned().unwrap_or_else(|| def.params.iter().find(|x| x.key == k).map(|x| x.default.to_string()).unwrap_or_default()) };
        let on = |k: &str| matches!(param(k).to_ascii_lowercase().as_str(), "on" | "1" | "true" | "yes");
        let numv = |k: &str, dflt: f64| param(k).replace(',', ".").parse::<f64>().unwrap_or(dflt);
        // Kịch trần: cao tủ = cao trần − che trần.
        let h = if on("ceiling") { numv("ceiling_h", 2700.0) - numv("ceiling_gap", 50.0) } else { h };
        let size = |plinth: f64| CabinetOverrides { width: Some(w), height: Some(h), depth: Some(d), plinth_height: Some(plinth), doors: Some(0), shelves: Some(0), drawers: Some(0), ..Default::default() };
        let mut kitchen_set = true;
        let cab = match key {
            "KITCHEN_BASE_800" => {
                let cab = self.create(CabinetKind::Base, size(100.0), "BếpDưới", p, "Bếp")?;
                self.set(cab, &[("base_type", "LEGS_PLINTH"), ("s_joint_type", "CAM_DOWEL"), ("back_groove", "8")])?;
                let root = self.root_zone(cab)?;
                self.shelves(cab, vec![root], 1)?;
                self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                cab
            }
            "KITCHEN_WALL_800" => {
                let cab = self.create(CabinetKind::Wall, size(0.0), "BếpTrên", p, "Bếp")?;
                self.set(cab, &[("base_type", "HANGING"), ("hang_rail", "on"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.root_zone(cab)?;
                self.shelves(cab, vec![root], 1)?;
                if param("lift") == "LIFT_UP" {
                    self.doors(cab, vec![root], DoorKind::LiftUp, 1, HingeSide::Top)?;
                } else {
                    self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                }
                cab
            }
            "KITCHEN_CORNER_L" => {
                let r = self.handle(Request::CreateCorner {
                    hand: "LEFT".into(),
                    kind: None,
                    wall: false,
                    width: Some(w),
                    height: Some(h),
                    depth: Some(d),
                    door_width: Some(450.0),
                    position: p.position,
                    room: p.room.clone().or_else(|| Some("Bếp".into())),
                    floor: p.floor.clone(),
                    after: p.after,
                })?;
                let cab: ObjectId = serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
                self.set(cab, &[("plinth_height", "100"), ("base_type", "LEGS_PLINTH"), ("s_joint_type", "CAM_DOWEL")])?;
                let leaves: Vec<Uid> = self.leaves(cab).into_iter().map(|z| z.0).collect();
                self.shelves(cab, leaves, 1)?;
                cab
            }
            "KITCHEN_OVEN_TALL" => {
                let mut o = size(100.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Base, o, "TủLò", p, "Bếp")?;
                self.set(cab, &[("base_type", "LEGS_PLINTH"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.root_zone(cab)?;
                // Dưới 720 (2 ngăn kéo) · khoang lò 600 · phần còn lại cánh lật.
                let leaves = self.split(cab, root, SplitKind::ShelfFixed, "720,600", false)?;
                if leaves.len() == 3 {
                    self.drawers(cab, leaves[0], 2, false)?;
                    self.doors(cab, vec![leaves[2]], DoorKind::Single, 1, HingeSide::Top)?;
                    // Khoang lò: thanh đỡ + khe thoát nhiệt khoét hậu (D34).
                    self.zone_add_link(cab, vec![leaves[1]], aic_domain::zone::LinkKind::ApplianceBay, 0.0, "OVEN-600".into())?;
                }
                cab
            }
            "WARDROBE_2D_1000" => {
                kitchen_set = false;
                let mut o = size(80.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Wardrobe, o, "TủÁo", p, "")?;
                self.set(cab, &[("s_pin_row", "ROW_32"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.clear_zones(cab)?;
                let halves = self.split(cab, root, SplitKind::Divider, "50%", false)?;
                if let [l, r] = halves[..] {
                    self.rail(cab, l)?;
                    self.shelves(cab, vec![r], numv("shelves", 4.0).clamp(0.0, 10.0) as u32)?;
                }
                self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                cab
            }
            "WARDROBE_4D_1800" | "WARDROBE_3BAY_CEILING" => {
                kitchen_set = false;
                let mut o = size(80.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Wardrobe, o, "TủÁo", p, "")?;
                self.set(cab, &[("s_pin_row", "ROW_32"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.clear_zones(cab)?;
                // Tủ trên 400 (chia ảo, cánh riêng) + tủ dưới.
                let tb = self.split(cab, root, SplitKind::ShelfFixed, "400", true)?;
                let (low, top) = (tb[0], tb[1]);
                if key == "WARDROBE_4D_1800" {
                    // 2 khoang đều: mỗi khoang tầng dưới = 3 ngăn kéo trong (600) + treo.
                    let bays = self.split(cab, low, SplitKind::Divider, "/2", false)?;
                    let tops = self.split(cab, top, SplitKind::VirtualV, "/2", false)?;
                    for b in bays {
                        let parts = self.split(cab, b, SplitKind::VirtualH, "600", false)?;
                        self.drawers(cab, parts[0], 3, true)?;
                        self.rail(cab, parts[1])?;
                        self.doors(cab, vec![b], DoorKind::Double, 2, HingeSide::Left)?;
                    }
                    for t in tops {
                        self.shelves(cab, vec![t], 1)?;
                        self.doors(cab, vec![t], DoorKind::Double, 2, HingeSide::Left)?;
                    }
                } else {
                    // 800,*,800: treo dài · kệ + 3 ngăn kéo trong · treo 2 tầng.
                    let bays = self.split(cab, low, SplitKind::Divider, "800,*,800", false)?;
                    let tops = self.split(cab, top, SplitKind::VirtualV, "800,*,800", false)?;
                    if let [a, m, c] = bays[..] {
                        self.rail(cab, a)?;
                        let mid = self.split(cab, m, SplitKind::VirtualH, "600", false)?;
                        self.drawers(cab, mid[0], 3, true)?;
                        self.shelves(cab, vec![mid[1]], 2)?;
                        let two = self.split(cab, c, SplitKind::ShelfFixed, "/2", false)?;
                        for z in two {
                            self.rail(cab, z)?;
                        }
                        for z in [a, m, c] {
                            self.doors(cab, vec![z], DoorKind::Double, 2, HingeSide::Left)?;
                        }
                    }
                    for t in tops {
                        self.doors(cab, vec![t], DoorKind::Double, 2, HingeSide::Left)?;
                    }
                }
                cab
            }
            "WARDROBE_SLIDING_2000" => {
                kitchen_set = false;
                let mut o = size(80.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Wardrobe, o, "TủÁoLùa", p, "")?;
                self.set(cab, &[("s_pin_row", "ROW_32"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.clear_zones(cab)?;
                let halves = self.split(cab, root, SplitKind::Divider, "/2", false)?;
                if let [l, r] = halves[..] {
                    self.rail(cab, l)?;
                    let parts = self.split(cab, r, SplitKind::VirtualH, "500", false)?;
                    // Ngăn kéo trong lùi sau cánh lùa.
                    self.drawers(cab, parts[0], 2, true)?;
                    self.shelves(cab, vec![parts[1]], 3)?;
                }
                let leaves = param("leaves").parse::<u32>().unwrap_or(2).clamp(2, 3);
                self.doors(cab, vec![root], DoorKind::Sliding, leaves, HingeSide::Left)?;
                self.edit_cabinet(cab, "Hệ ray cánh lùa", |c| {
                    if let Some(aic_domain::zone::Front::Doors(dd)) = c.zones.zone_mut(root).and_then(|z| z.front.as_mut()) {
                        dd.sliding = Some(aic_domain::zone::SlidingSpec { overlap: 35.0, ..Default::default() });
                    }
                    Ok(())
                })?;
                cab
            }
            "BED_1600" | "BED_1600_DRAWERS" => {
                kitchen_set = false;
                let mut opts = std::collections::BTreeMap::new();
                if key == "BED_1600_DRAWERS" {
                    opts.insert("bed_frame_h".to_string(), "450".to_string());
                    opts.insert("bed_storage".to_string(), "DRAWERS_2_SIDES".to_string());
                    opts.insert("bed_drawer_count".to_string(), param("drawers"));
                }
                let r = self.create_furniture("BED", [Some(w), Some(h), Some(d)], p.position, None, p.room.clone(), p.floor.clone(), opts)?;
                serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?
            }
            "DESK_1200" => {
                kitchen_set = false;
                let mut opts = std::collections::BTreeMap::new();
                if on("hutch") {
                    opts.insert("desk_hutch_h".to_string(), "600".to_string());
                    opts.insert("desk_hutch_shelves".to_string(), "2".to_string());
                }
                let r = self.create_furniture("DESK", [Some(w), Some(h), Some(d)], p.position, None, p.room.clone(), p.floor.clone(), opts)?;
                serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?
            }
            "TV_HANGING_1800" => {
                kitchen_set = false;
                let mut o = size(0.0);
                o.top_style = Some("OVERLAY".into());
                o.bottom_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Base, o, "KệTV", p, "Phòng khách")?;
                self.set(cab, &[("base_type", "HANGING"), ("s_handle_type", "PUSH_OPEN")])?;
                let root = self.root_zone(cab)?;
                let bays = self.split(cab, root, SplitKind::Divider, "/3", false)?;
                if let [a, _, c] = bays[..] {
                    self.drawers(cab, a, 1, false)?;
                    self.drawers(cab, c, 1, false)?;
                }
                self.handle(Request::SetBackCutouts { cabinet: cab, cutouts: vec![aic_domain::structure::BackCutout { kind: aic_domain::structure::CutoutKind::Pipe, anchor: aic_domain::structure::HAnchor::Center, x: 0.0, y: 120.0, w: 60.0, h: 60.0, r: 30.0 }] })?;
                cab
            }
            "SHOE_1200" => {
                kitchen_set = false;
                let mut o = size(80.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Base, o, "TủGiày", p, "Phòng khách")?;
                let root = self.root_zone(cab)?;
                let halves = self.split(cab, root, SplitKind::Divider, "/2", false)?;
                let tilt = numv("tilt", 15.0).clamp(0.0, 45.0);
                self.zone_add_panels(ZoneAddPanels { cabinet: cab, zones: halves, kind: SplitKind::ShelfFixed, count: 4, thickness: None, lock: Lock::Even, value: 0.0, tilt_deg: Some([tilt, 0.0]) })?;
                self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                self.handle(Request::SetBackCutouts { cabinet: cab, cutouts: vec![aic_domain::structure::BackCutout { kind: aic_domain::structure::CutoutKind::Vent, anchor: aic_domain::structure::HAnchor::Center, x: 0.0, y: h - 80.0 - 2.0 * 17.2 - 60.0, w: (w - 200.0).clamp(100.0, 600.0), h: 40.0, r: 10.0 }] })?;
                cab
            }
            "ISLAND_1800" => {
                let mut opts = std::collections::BTreeMap::new();
                opts.insert("island_seat_overhang".to_string(), param("seat"));
                opts.insert("tr_end_left".to_string(), "on".to_string());
                opts.insert("tr_end_right".to_string(), "on".to_string());
                opts.insert("base_type".to_string(), "LEGS_PLINTH".to_string());
                let r = self.create_furniture("ISLAND", [Some(w), Some(h), Some(d)], p.position, None, p.room.clone().or_else(|| Some("Bếp".into())), p.floor.clone(), opts)?;
                let cab: ObjectId = serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
                let root = self.root_zone(cab)?;
                let rf = self.kids(cab, root)?;
                if let [rear, front] = rf[..] {
                    let cols = self.split(cab, front, SplitKind::Divider, "*,600", false)?;
                    if let [a, b] = cols[..] {
                        self.doors(cab, vec![a], DoorKind::Single, 1, HingeSide::Left)?;
                        self.drawers(cab, b, 3, false)?;
                    }
                    self.shelves(cab, vec![rear], 1)?;
                }
                cab
            }
            "WASHER_700" => {
                let cab = self.create(CabinetKind::Base, size(0.0), "TủMáyGiặt", p, "WC")?;
                self.set(cab, &[("base_type", "LEGS")])?;
                let root = self.root_zone(cab)?;
                // Khoang máy giặt dưới (≥ 620 × 870 × 600 lọt lòng), phía trên kệ + cánh đôi.
                let parts = self.split(cab, root, SplitKind::ShelfFixed, "890", false)?;
                if let [low, top] = parts[..] {
                    self.zone_add_link(cab, vec![low], aic_domain::zone::LinkKind::ApplianceBay, 0.0, "WASHER-600".into())?;
                    self.shelves(cab, vec![top], 2)?;
                    self.doors(cab, vec![top], DoorKind::Double, 2, HingeSide::Left)?;
                }
                cab
            }
            "ALTAR_1270" => {
                kitchen_set = false;
                let mut o = size(80.0);
                o.top_style = Some("OVERLAY".into());
                let cab = self.create(CabinetKind::Base, o, "TủThờ", p, "Phòng thờ")?;
                self.set(cab, &[("tr_cornice", "3_SIDES"), ("tr_cornice_h", "80")])?;
                let root = self.root_zone(cab)?;
                // 2 hộc kéo trên cao 200, dưới khoang trang trí mở + đèn LED.
                let parts = self.split(cab, root, SplitKind::ShelfFixed, "200", true)?;
                if let [low, top] = parts[..] {
                    self.handle(Request::ZoneAddDrawers { cabinet: cab, zones: vec![top], count: 1, cols: 2, mount: Mount::Overlay, thickness: None, with_box: true, inner: false, false_front: false })?;
                    self.zone_add_link(cab, vec![low], aic_domain::zone::LinkKind::Accessory, 0.0, "LED-STRIP".into())?;
                }
                cab
            }
            "LAVABO_800" => {
                let cab = self.create(CabinetKind::Base, size(0.0), "TủLavabo", p, "WC")?;
                self.set(cab, &[("base_type", "HANGING"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.root_zone(cab)?;
                let parts = self.split(cab, root, SplitKind::VirtualH, "150", true)?;
                self.handle(Request::ZoneAddDrawers { cabinet: cab, zones: vec![parts[1]], count: 1, cols: 1, mount: Mount::Overlay, thickness: None, with_box: false, inner: false, false_front: true })?;
                self.doors(cab, vec![parts[0]], DoorKind::Double, 2, HingeSide::Left)?;
                self.handle(Request::SetBackCutouts { cabinet: cab, cutouts: vec![aic_domain::structure::BackCutout { kind: aic_domain::structure::CutoutKind::Pipe, anchor: aic_domain::structure::HAnchor::Center, x: 0.0, y: 200.0, w: 60.0, h: 60.0, r: 30.0 }] })?;
                cab
            }
            _ => return Err(crate::zones::bad("product", "unknown product")),
        };
        if kitchen_set {
            self.apply_material_set(vec![cab], None, KITCHEN_SET)?;
        }
        Ok(cab)
    }

    /// Chèn một mẫu sản phẩm: một bước undo, lỗi thì trả lại như cũ.
    pub(crate) fn insert_product(&mut self, key: &str, mut p: Place) -> Result<ObjectId, CoreError> {
        // Đặt cạnh tủ `after`: cùng phòng / tầng với tủ đó.
        if let Some(c) = p.after.and_then(|a| self.doc.objects.get(&a)).and_then(|o| o.as_cabinet()) {
            p.room = p.room.or_else(|| Some(c.room.clone()));
            p.floor = p.floor.or_else(|| Some(c.floor.clone()));
        }
        let label = PRODUCTS.iter().find(|x| x.key == key).map(|x| x.name).unwrap_or("Mẫu sản phẩm");
        let mark = self.history.mark();
        match self.build_product(key, &p) {
            Ok(id) => {
                self.history.squash(mark, label);
                Ok(id)
            }
            Err(e) => {
                self.history.rollback(&mut self.doc, mark);
                Err(e)
            }
        }
    }
}

