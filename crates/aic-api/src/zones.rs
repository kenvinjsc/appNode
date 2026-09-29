//! Zone-based cabinet editing (Tạo tấm / Chỉnh tấm). Every edit clones the
//! cabinet definition, changes it with pure domain functions and applies it
//! with a single undoable `Command::SetCabinet`.

use crate::protocol::{PartModPatch, ZoneAddPanels};
use crate::Engine;
use aic_domain::zone::{DoorKind, DoorSpec, DrawerSpec, Front, HingeSide, LinkKind, Lock, Mount, SplitKind, StopRail, StopRailSpec, Uid};
use aic_domain::{Cabinet, DomainObject, ObjectId};
use aic_project::{Command, CoreError};
use serde_json::{json, Value};

pub(crate) fn bad(name: &str, reason: impl Into<String>) -> CoreError {
    CoreError::InvalidParameter { name: name.into(), reason: reason.into() }
}

fn num(name: &str, v: &str) -> Result<f64, CoreError> {
    v.trim().trim_start_matches('=').trim().replace(',', ".").parse::<f64>().map_err(|_| bad(name, "expected a number"))
}

fn count(name: &str, v: &str) -> Result<u32, CoreError> {
    let n = num(name, v)?;
    if !(0.0..=50.0).contains(&n) || n.fract() != 0.0 {
        return Err(bad(name, "expected a whole number 0..50"));
    }
    Ok(n as u32)
}

fn flag(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes" | "có")
}

/// Kind of generated part behind a part key.
pub(crate) enum PartRef {
    Carcass,
    Split(Uid),
    Door(Uid),
    Drawer(Uid),
    Link(Uid),
}

pub(crate) fn parse_key(k: &str) -> PartRef {
    let mut it = k.split(':');
    let kind = it.next().unwrap_or("");
    let uid: Uid = it.next().and_then(|u| u.parse().ok()).unwrap_or(0);
    match kind {
        "p" => PartRef::Split(uid),
        "d" | "h" | "s" | "t" => PartRef::Door(uid),
        "w" => PartRef::Drawer(uid),
        "l" => PartRef::Link(uid),
        _ => PartRef::Carcass,
    }
}

impl Engine {
    /// (cabinet, part key) of a generated part.
    pub(crate) fn part_ref(&self, id: ObjectId) -> Option<(ObjectId, String)> {
        let key = match self.doc.objects.get(&id)? {
            DomainObject::Panel(p) => p.gen_key.clone()?,
            DomainObject::Hardware(h) => h.gen_key.clone()?,
            _ => return None,
        };
        if !self.doc.generated.contains(&id) {
            return None;
        }
        Some((self.doc.cabinet_of(id)?, key))
    }

    pub(crate) fn cabinet_def(&self, cab: ObjectId) -> Result<Cabinet, CoreError> {
        self.doc.object(cab)?.as_cabinet().cloned().ok_or(CoreError::NotFound { id: cab })
    }

    pub(crate) fn edit_cabinet(&mut self, cab: ObjectId, label: &str, f: impl FnOnce(&mut Cabinet) -> Result<(), CoreError>) -> Result<(), CoreError> {
        let mut def = self.cabinet_def(cab)?;
        f(&mut def)?;
        self.exec_cmd(Command::SetCabinet { id: cab, cabinet: Box::new(def), label: label.into() })
    }

    pub(crate) fn zones_info(&self, cab: ObjectId) -> Result<Value, CoreError> {
        let def = self.cabinet_def(cab)?;
        let layout = self.doc.cabinet_layout(cab).ok_or(CoreError::NotFound { id: cab })?;
        let world = self.doc.scene.world(cab);
        let attachments: Vec<Value> = def
            .zones
            .zones()
            .iter()
            .flat_map(|z| {
                let mut v = Vec::new();
                if let Some(f) = &z.front {
                    v.push(json!({ "zone": z.id, "uid": f.uid(), "front": f }));
                }
                for l in &z.links {
                    v.push(json!({ "zone": z.id, "uid": l.uid, "link": l }));
                }
                v
            })
            .collect();
        Ok(json!({
            "cabinet": cab,
            "name": def.name,
            "room": def.room,
            "matrix": world.to_matrix_col_major(),
            "zones": layout.zones,
            "positions": layout.positions,
            "attachments": attachments,
            "fittings": layout.fittings,
        }))
    }

    pub(crate) fn zone_add_panels(&mut self, r: ZoneAddPanels) -> Result<Value, CoreError> {
        let kind = r.kind;
        let t = r.thickness.unwrap_or_else(|| if kind == SplitKind::BackSub { self.doc.param_value(r.cabinet, "back_thickness").unwrap_or(8.6) } else { self.doc.param_value(r.cabinet, "thickness").unwrap_or(17.2) });
        let value = match r.lock {
            Lock::Ratio => r.value / 100.0,
            _ => r.value,
        };
        let mut created = Vec::new();
        for zone in &r.zones {
            let zone = *zone;
            self.edit_cabinet(r.cabinet, "Tạo tấm", |c| {
                let uids = c.zones.add_panels(zone, kind, r.count, t, r.lock, value).map_err(|e| bad("zone", e))?;
                if let Some(tilt) = r.tilt_deg {
                    for u in &uids {
                        if let Some(p) = c.zones.panel_mut(*u) {
                            p.tilt_deg = tilt;
                        }
                    }
                }
                created.extend(uids);
                Ok(())
            })?;
        }
        Ok(json!({ "uids": created }))
    }

    pub(crate) fn zone_set_front(&mut self, cab: ObjectId, zones: Vec<Uid>, front: Option<Front>) -> Result<(), CoreError> {
        self.edit_cabinet(cab, if front.is_some() { "Thêm cánh / ngăn kéo" } else { "Xóa cánh" }, |c| {
            for z in zones {
                let f = front.clone().map(|mut f| {
                    let uid = c.zones.alloc();
                    match &mut f {
                        Front::Doors(d) => d.uid = uid,
                        Front::Drawers(d) => d.uid = uid,
                    }
                    f
                });
                c.zones.set_front(z, f).map_err(|e| bad("zone", e))?;
            }
            Ok(())
        })
    }

    pub(crate) fn zone_add_link(&mut self, cab: ObjectId, zones: Vec<Uid>, kind: LinkKind, offset: f64) -> Result<(), CoreError> {
        self.edit_cabinet(cab, "Thêm liên kết", |c| {
            for z in zones {
                c.zones.add_link(z, kind, offset).map_err(|e| bad("zone", e))?;
            }
            Ok(())
        })
    }

    pub(crate) fn zone_remove(&mut self, cab: ObjectId, uid: Uid) -> Result<(), CoreError> {
        self.edit_cabinet(cab, "Xóa tấm", |c| {
            if c.zones.panel(uid).is_some() {
                c.zones.remove_panel(uid).map_err(|e| bad("uid", e))
            } else {
                c.zones.remove_attachment(uid).map_err(|e| bad("uid", e))
            }
        })
    }

    pub(crate) fn set_part_mod(&mut self, id: ObjectId, patch: PartModPatch) -> Result<(), CoreError> {
        let (cab, key) = self.part_ref(id).ok_or_else(|| bad("part", "not a generated part"))?;
        self.edit_cabinet(cab, "Chỉnh tấm", |c| {
            let m = c.mods.entry(key.clone()).or_default();
            if let Some(n) = patch.name {
                m.name = if n.trim().is_empty() { None } else { Some(n) };
            }
            if let Some(e) = patch.extend {
                m.extend = e;
                let tool = "11. Co giãn tấm".to_string();
                m.tools.retain(|t| *t != tool);
                if e.iter().any(|v| *v != 0.0) {
                    m.tools.push(tool);
                }
            }
            if let Some(d) = patch.extend_delta {
                for i in 0..4 {
                    m.extend[i] += d[i];
                }
                let tool = "11. Co giãn tấm".to_string();
                if !m.tools.contains(&tool) {
                    m.tools.push(tool);
                }
            }
            if let Some(t) = patch.thickness {
                m.thickness = t;
            }
            if patch.clear_tools {
                m.extend = [0.0; 4];
                m.features.clear();
                m.tools.clear();
            }
            if let Some(f) = patch.add_features {
                m.features.extend(f);
                if let Some(t) = patch.tool {
                    if !m.tools.contains(&t) {
                        m.tools.push(t);
                    }
                }
            }
            if m.is_default() {
                c.mods.remove(&key);
            }
            Ok(())
        })
    }

    /// Delete commands for generated parts: zone parts are removed from the
    /// cabinet definition (zones merge); carcass parts are marked deleted.
    pub(crate) fn delete_generated(&mut self, ids: &[ObjectId]) -> Result<Vec<Command>, CoreError> {
        let mut per_cab: std::collections::BTreeMap<ObjectId, Cabinet> = Default::default();
        for id in ids {
            let Some((cab, key)) = self.part_ref(*id) else { continue };
            if let std::collections::btree_map::Entry::Vacant(e) = per_cab.entry(cab) {
                e.insert(self.cabinet_def(cab)?);
            }
            let c = per_cab.get_mut(&cab).unwrap();
            match parse_key(&key) {
                PartRef::Split(u) => {
                    let _ = c.zones.remove_panel(u);
                }
                PartRef::Door(u) | PartRef::Drawer(u) | PartRef::Link(u) => {
                    let _ = c.zones.remove_attachment(u);
                }
                PartRef::Carcass => {
                    c.mods.entry(key).or_default().deleted = true;
                }
            }
        }
        Ok(per_cab.into_iter().map(|(id, c)| Command::SetCabinet { id, cabinet: Box::new(c), label: "Xóa tấm".into() }).collect())
    }

    /// Property edits on generated parts / cabinet options. Returns Ok(false)
    /// when `name` is not a zone property (caller falls back to parameters).
    pub(crate) fn set_zone_property(&mut self, id: ObjectId, name: &str, value: &str) -> Result<bool, CoreError> {
        // Cabinet-level options.
        if let Some(DomainObject::Cabinet(_)) = self.doc.objects.get(&id) {
            return match name {
                "room" => self.edit_cabinet(id, "Tên phòng", |c| {
                    c.room = value.trim().to_string();
                    Ok(())
                }).map(|_| true),
                "edge_mode" | "edge_band" | "edge_threshold" | "edge_min_length" | "edge_skip" => self
                    .edit_cabinet(id, "Luật dán cạnh", |c| {
                        let r = &mut c.edge_rule;
                        match name {
                            "edge_mode" => {
                                r.mode = match value.trim().to_ascii_uppercase().as_str() {
                                    "ALL" => aic_domain::EdgeMode::All,
                                    "NONE" => aic_domain::EdgeMode::None,
                                    _ => aic_domain::EdgeMode::ExposedOnly,
                                }
                            }
                            "edge_band" => {
                                let (code, t) = match value.trim().to_ascii_uppercase().as_str() {
                                    "DON-0.5" => ("DON-0.5", 0.5),
                                    "DON-2" => ("DON-2", 2.0),
                                    "KEP-1" => ("KEP-1", 1.0),
                                    _ => ("DON-1", 1.0),
                                };
                                r.band_code = code.into();
                                r.band_thickness = t;
                            }
                            "edge_threshold" => r.threshold = num(name, value)?,
                            "edge_min_length" => r.min_length = num(name, value)?,
                            _ => {
                                r.skip_thicknesses = value
                                    .split([',', ';', ' '])
                                    .filter(|s| !s.trim().is_empty())
                                    .map(|s| num(name, s))
                                    .collect::<Result<_, _>>()?;
                            }
                        }
                        Ok(())
                    })
                    .map(|_| true),
                _ => Ok(false),
            };
        }
        let Some((cab, key)) = self.part_ref(id) else { return Ok(false) };
        let pref = parse_key(&key);
        let p = |n: &str| name == n;
        // Part mods valid for every generated panel.
        if p("name") {
            self.set_part_mod(id, PartModPatch { name: Some(value.to_string()), ..Default::default() })?;
            return Ok(true);
        }
        if let Some(edge) = name.strip_prefix("ext_") {
            let i = match edge {
                "left" => 0,
                "right" => 1,
                "bottom" => 2,
                "top" => 3,
                _ => return Err(bad(name, "unknown edge")),
            };
            let (_, key2) = self.part_ref(id).unwrap();
            let mut e = self.cabinet_def(cab)?.mods.get(&key2).map(|m| m.extend).unwrap_or_default();
            e[i] = num(name, value)?;
            self.set_part_mod(id, PartModPatch { extend: Some(e), ..Default::default() })?;
            return Ok(true);
        }
        match pref {
            PartRef::Split(uid) => {
                let v = value.to_string();
                let n = name.to_string();
                let handled = matches!(name, "pos_ratio" | "pos_start" | "pos_end" | "pos_lock" | "split_kind" | "thickness" | "tilt_fb" | "tilt_lr");
                if !handled {
                    return Ok(false);
                }
                self.edit_cabinet(cab, "Chỉnh tấm", move |c| {
                    let sp = c.zones.panel_mut(uid).ok_or_else(|| bad(&n, "panel not found"))?;
                    match n.as_str() {
                        "pos_ratio" => {
                            sp.lock = Lock::Ratio;
                            sp.value = (num(&n, &v)? / 100.0).clamp(0.0, 1.0);
                        }
                        "pos_start" => {
                            sp.lock = Lock::FromStart;
                            sp.value = num(&n, &v)?.max(0.0);
                        }
                        "pos_end" => {
                            sp.lock = Lock::FromEnd;
                            sp.value = num(&n, &v)?.max(0.0);
                        }
                        "pos_lock" => {
                            sp.lock = match v.trim().to_ascii_uppercase().as_str() {
                                "EVEN" => Lock::Even,
                                "FROM_START" => Lock::FromStart,
                                "FROM_END" => Lock::FromEnd,
                                _ => Lock::Ratio,
                            }
                        }
                        "split_kind" => {
                            let k = match v.trim().to_ascii_uppercase().as_str() {
                                "SHELF_FIXED" => SplitKind::ShelfFixed,
                                "SHELF_ADJUSTABLE" => SplitKind::ShelfAdjustable,
                                "DIVIDER" => SplitKind::Divider,
                                "BACK_SUB" => SplitKind::BackSub,
                                _ => return Err(bad(&n, "unknown kind")),
                            };
                            if k.axis() != sp.kind.axis() {
                                return Err(bad(&n, "kind must keep the same direction"));
                            }
                            sp.kind = k;
                        }
                        "thickness" => {
                            let t = num(&n, &v)?;
                            if !(1.0..=100.0).contains(&t) {
                                return Err(bad(&n, "thickness 1..100"));
                            }
                            sp.thickness = t;
                        }
                        "tilt_fb" => sp.tilt_deg[0] = num(&n, &v)?,
                        _ => sp.tilt_deg[1] = num(&n, &v)?,
                    }
                    Ok(())
                })?;
                // Keep the resolved value meaningful when switching lock: store the current resolved value.
                Ok(true)
            }
            PartRef::Door(uid) if name.starts_with("door_") => {
                let v = value.to_string();
                let n = name.to_string();
                self.edit_cabinet(cab, "Chỉnh cánh", move |c| {
                    let Some(Front::Doors(d)) = c.zones.front_mut(uid) else { return Err(bad(&n, "door not found")) };
                    match n.as_str() {
                        "door_kind" => {
                            d.kind = match v.trim().to_ascii_uppercase().as_str() {
                                "DOUBLE" => DoorKind::Double,
                                "SLIDING" => DoorKind::Sliding,
                                _ => DoorKind::Single,
                            }
                        }
                        "door_mount" => d.mount = if v.trim().eq_ignore_ascii_case("INSET") { Mount::Inset } else { Mount::Overlay },
                        "door_hinge" => {
                            d.hinge = match v.trim().to_ascii_uppercase().as_str() {
                                "RIGHT" => HingeSide::Right,
                                "TOP" => HingeSide::Top,
                                "BOTTOM" => HingeSide::Bottom,
                                _ => HingeSide::Left,
                            }
                        }
                        "door_cols" => d.cols = count(&n, &v)?.max(1),
                        "door_rows" => d.rows = count(&n, &v)?.max(1),
                        "door_thickness" => d.thickness = Some(num(&n, &v)?),
                        "door_gap" => d.gap = Some(num(&n, &v)?),
                        "door_stop" => {
                            d.stop.kind = match v.trim().to_ascii_uppercase().as_str() {
                                "L_SHAPE" => StopRail::LShape,
                                "STRAIGHT" => StopRail::Straight,
                                _ => StopRail::None,
                            }
                        }
                        "door_stop_height" => d.stop.height = num(&n, &v)?,
                        "door_stop_cover" => d.stop.cover_up = num(&n, &v)?,
                        "door_stop_leg" => d.stop.leg_depth = num(&n, &v)?,
                        "door_stop_setback" => d.stop.setback = num(&n, &v)?,
                        _ => return Err(bad(&n, "unknown door property")),
                    }
                    Ok(())
                })?;
                Ok(true)
            }
            PartRef::Drawer(uid) if name.starts_with("drawer_") => {
                let v = value.to_string();
                let n = name.to_string();
                self.edit_cabinet(cab, "Chỉnh ngăn kéo", move |c| {
                    let Some(Front::Drawers(d)) = c.zones.front_mut(uid) else { return Err(bad(&n, "drawer not found")) };
                    match n.as_str() {
                        "drawer_count" => d.count = count(&n, &v)?.max(1),
                        "drawer_cols" => d.cols = count(&n, &v)?.max(1),
                        "drawer_mount" => d.mount = if v.trim().eq_ignore_ascii_case("INSET") { Mount::Inset } else { Mount::Overlay },
                        "drawer_face_thickness" => d.face_thickness = Some(num(&n, &v)?),
                        "drawer_side_gap" => d.side_gap = num(&n, &v)?,
                        "drawer_gap" => d.gap = num(&n, &v)?,
                        "drawer_box" => d.with_box = flag(&v),
                        _ => return Err(bad(&n, "unknown drawer property")),
                    }
                    Ok(())
                })?;
                Ok(true)
            }
            PartRef::Link(uid) if name == "link_offset" => {
                let off = num(name, value)?;
                self.edit_cabinet(cab, "Chỉnh liên kết", move |c| {
                    let zid = c.zones.zone_of_attachment(uid).ok_or_else(|| bad("link", "not found"))?;
                    for l in &mut c.zones.zone_mut(zid).unwrap().links {
                        if l.uid == uid {
                            l.offset = off;
                        }
                    }
                    Ok(())
                })?;
                Ok(true)
            }
            _ if name == "thickness" && !matches!(pref, PartRef::Split(_)) => {
                let t = num(name, value)?;
                self.set_part_mod(id, PartModPatch { thickness: Some(Some(t)), ..Default::default() })?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Default door spec used by the "Tạo cánh" panel.
pub(crate) fn default_door(kind: DoorKind, cols: u32, rows: u32, mount: Mount, hinge: HingeSide, thickness: Option<f64>, stop: Option<StopRailSpec>) -> Front {
    Front::Doors(DoorSpec { uid: 0, kind, cols: cols.max(1), rows: rows.max(1), mount, hinge, thickness, gap: None, stop: stop.unwrap_or_default() })
}

pub(crate) fn default_drawers(count: u32, cols: u32, mount: Mount, thickness: Option<f64>, with_box: bool) -> Front {
    let mut d = DrawerSpec::new(0, count.max(1), mount);
    d.cols = cols.max(1);
    d.face_thickness = thickness;
    d.with_box = with_box;
    Front::Drawers(d)
}

