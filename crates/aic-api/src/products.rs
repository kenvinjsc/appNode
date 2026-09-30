//! Mẫu sản phẩm dựng sẵn (D30, §7 báo cáo đề xuất): mỗi mẫu dựng bằng chuỗi request có sẵn
//! (tạo tủ, chuẩn xưởng, chia khoang, cánh / ngăn kéo, bộ vật liệu) và gộp thành một bước undo.

use crate::protocol::{CabinetOverrides, Request, SplitZone, ZoneAddPanels};
use crate::Engine;
use aic_domain::zone::{DoorKind, HingeSide, Lock, Mount, SplitKind, Uid};
use aic_domain::{CabinetKind, ObjectId};
use aic_project::CoreError;
use serde_json::{json, Value};

/// (mã, tên, phòng gợi ý, mô tả).
pub const PRODUCTS: &[(&str, &str, &str, &str)] = &[
    ("KITCHEN_BASE_800", "Tủ bếp dưới 2 cánh 800", "Bếp", "800 × 810 × 560 · chân nhựa 100 + len · 1 kệ · cánh đôi · MFC lõi xanh / Acrylic"),
    ("KITCHEN_WALL_800", "Tủ bếp trên 800", "Bếp", "800 × 700 × 350 · treo tường · 1 kệ · cánh đôi · tay nắm dưới"),
    ("KITCHEN_CORNER_L", "Tủ bếp góc L 1100", "Bếp", "1100 × 810 × 560 · tấm mù + cánh 450 · 1 kệ mỗi khoang"),
    ("KITCHEN_OVEN_TALL", "Tủ lò 600 kịch trần", "Bếp", "600 × 2300 × 580 · 2 ngăn kéo dưới 720 · khoang lò 600 · cánh lật trên"),
];

const KITCHEN_SET: &str = "Bếp chống ẩm (MFC lõi xanh + Acrylic)";

pub struct Place {
    pub position: Option<[f64; 3]>,
    pub room: Option<String>,
    pub floor: Option<String>,
    pub after: Option<ObjectId>,
}

impl Engine {
    pub(crate) fn products_info(&self) -> Value {
        json!({ "products": PRODUCTS.iter().map(|(k, n, r, d)| json!({ "key": k, "name": n, "room": r, "summary": d })).collect::<Vec<_>>() })
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

    fn build_product(&mut self, key: &str, p: &Place) -> Result<ObjectId, CoreError> {
        let cab = match key {
            "KITCHEN_BASE_800" => {
                let o = CabinetOverrides { width: Some(800.0), height: Some(810.0), depth: Some(560.0), plinth_height: Some(100.0), doors: Some(0), shelves: Some(0), ..Default::default() };
                let cab = self.create(CabinetKind::Base, o, "BếpDưới", p, "Bếp")?;
                self.set(cab, &[("base_type", "LEGS_PLINTH"), ("s_joint_type", "CAM_DOWEL"), ("back_groove", "8")])?;
                let root = self.root_zone(cab)?;
                self.shelves(cab, vec![root], 1)?;
                self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                cab
            }
            "KITCHEN_WALL_800" => {
                let o = CabinetOverrides { width: Some(800.0), height: Some(700.0), depth: Some(350.0), plinth_height: Some(0.0), doors: Some(0), shelves: Some(0), ..Default::default() };
                let cab = self.create(CabinetKind::Wall, o, "BếpTrên", p, "Bếp")?;
                self.set(cab, &[("base_type", "HANGING"), ("hang_rail", "on"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.root_zone(cab)?;
                self.shelves(cab, vec![root], 1)?;
                self.doors(cab, vec![root], DoorKind::Double, 2, HingeSide::Left)?;
                cab
            }
            "KITCHEN_CORNER_L" => {
                let r = self.handle(Request::CreateCorner {
                    hand: "LEFT".into(),
                    kind: None,
                    wall: false,
                    width: Some(1100.0),
                    height: Some(810.0),
                    depth: Some(560.0),
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
                let o = CabinetOverrides { width: Some(600.0), height: Some(2300.0), depth: Some(580.0), plinth_height: Some(100.0), doors: Some(0), shelves: Some(0), drawers: Some(0), top_style: Some("OVERLAY".into()), ..Default::default() };
                let cab = self.create(CabinetKind::Base, o, "TủLò", p, "Bếp")?;
                self.set(cab, &[("base_type", "LEGS_PLINTH"), ("s_joint_type", "CAM_DOWEL")])?;
                let root = self.root_zone(cab)?;
                // Dưới 720 (2 ngăn kéo) · khoang lò 600 · phần còn lại cánh lật.
                self.split_zone(SplitZone { cabinet: cab, zone: root, kind: SplitKind::ShelfFixed, formula: "720,600".into(), from_end: false, thickness: None })?;
                let leaves: Vec<Uid> = self.leaves(cab).into_iter().map(|z| z.0).collect();
                if leaves.len() == 3 {
                    self.handle(Request::ZoneAddDrawers { cabinet: cab, zones: vec![leaves[0]], count: 2, cols: 1, mount: Mount::Overlay, thickness: None, with_box: true, inner: false, false_front: false })?;
                    self.doors(cab, vec![leaves[2]], DoorKind::Single, 1, HingeSide::Top)?;
                    // Khoang lò: thanh đỡ + khe thoát nhiệt khoét hậu (D34).
                    self.zone_add_link(cab, vec![leaves[1]], aic_domain::zone::LinkKind::ApplianceBay, 0.0, "OVEN-600".into())?;
                }
                cab
            }
            _ => return Err(crate::zones::bad("product", "unknown product")),
        };
        self.apply_material_set(vec![cab], None, KITCHEN_SET)?;
        Ok(cab)
    }

    /// Chèn một mẫu sản phẩm: một bước undo, lỗi thì trả lại như cũ.
    pub(crate) fn insert_product(&mut self, key: &str, mut p: Place) -> Result<ObjectId, CoreError> {
        // Đặt cạnh tủ `after`: cùng phòng / tầng với tủ đó.
        if let Some(c) = p.after.and_then(|a| self.doc.objects.get(&a)).and_then(|o| o.as_cabinet()) {
            p.room = p.room.or_else(|| Some(c.room.clone()));
            p.floor = p.floor.or_else(|| Some(c.floor.clone()));
        }
        let label = PRODUCTS.iter().find(|x| x.0 == key).map(|x| x.1).unwrap_or("Mẫu sản phẩm");
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

