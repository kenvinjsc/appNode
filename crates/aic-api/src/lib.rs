//! AIC API: the single entry point the UI talks to.
//!
//! ```text
//! UI action → Request (JSON) → Engine::dispatch → Command/Query → Response + CoreEvents
//! ```
//! The engine owns the document, the undo history and all derived caches
//! (meshes, relations, manufacturing data, nesting results).

mod mfg;
mod properties;
pub mod protocol;
mod costing;
mod render;
mod zones;
pub mod shape;
mod templates;
pub mod library;
mod structure_api;
mod runs;
pub mod relations_edit;

pub use protocol::{ApiError, CoreEvent, Request, Response};

use aic_assembly::{AssemblyGraph, RelationSettings};
use aic_domain::{CabinetSpec, DomainObject, EdgeBand, MaterialId, MaterialSlot, ObjectId, PanelRole};
use aic_geometry::{CsgKernel, MeshData};
use aic_math::Transform3D;
use aic_nesting::NestingResult;
use aic_project::{ChangeSet, Command, CoreError, Document, History, ProjectFile};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Engine {
    pub(crate) doc: Document,
    pub(crate) history: History,
    pub(crate) kernel: CsgKernel,
    pub(crate) mesh_cache: HashMap<String, Arc<MeshData>>,
    pub(crate) relations: Option<(u64, Arc<AssemblyGraph>)>,
    pub(crate) joints: Option<(u64, Arc<HashMap<ObjectId, Vec<aic_manufacturing::DerivedFeature>>>)>,
    pub(crate) nesting: HashMap<String, (u64, NestingResult)>,
    pub(crate) nesting_settings: HashMap<String, aic_nesting::NestingSettings>,
    pub(crate) relation_settings: RelationSettings,
    /// Thư viện mẫu dùng chung mọi dự án (see `library.rs`).
    pub(crate) library: library::Library,
    pub(crate) library_path: Option<std::path::PathBuf>,
    pending_events: Vec<CoreEvent>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

fn ok(v: impl serde::Serialize) -> Result<Value, CoreError> {
    Ok(serde_json::to_value(v).expect("serialisable result"))
}

impl Engine {
    pub fn new() -> Self {
        let mut e = Self {
            doc: Document::new("Untitled"),
            history: History::new(),
            kernel: CsgKernel::new(),
            mesh_cache: HashMap::new(),
            relations: None,
            joints: None,
            nesting: HashMap::new(),
            nesting_settings: HashMap::new(),
            relation_settings: RelationSettings::default(),
            library: library::Library::default(),
            library_path: None,
            pending_events: Vec::new(),
        };
        e.doc.mark_loaded();
        e
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// JSON in, JSON out (used by Tauri and the dev server).
    pub fn dispatch_json(&mut self, request: &str) -> String {
        let resp = match serde_json::from_str::<Request>(request) {
            Ok(r) => self.dispatch(r),
            Err(e) => self.respond(Err(CoreError::InvalidParameter { name: "request".into(), reason: e.to_string() })),
        };
        serde_json::to_string(&resp).expect("response serialises")
    }

    pub fn dispatch(&mut self, req: Request) -> Response {
        use Request::*;
        // Dãy tủ theo tủ: lệnh sửa dữ liệu nào làm tủ trong dãy đổi hộp bao thì sinh lại
        // tấm dãy trong cùng bước undo.
        let sync = self.has_runs()
            && !matches!(req, Undo | Redo | CreateProject { .. } | LoadProject { .. } | CreateRun { .. } | UpdateRun { .. } | DeleteRun { .. });
        let mark = self.history.mark();
        let mut r = self.handle(req);
        if r.is_ok() && sync && self.history.mark() > mark {
            let label = self.history.label_at(mark).unwrap_or_default();
            match self.sync_runs() {
                Ok(()) => self.history.squash(mark, &label),
                Err(e) => {
                    self.history.rollback(&mut self.doc, mark);
                    r = Err(e);
                }
            }
        }
        self.respond(r)
    }

    fn respond(&mut self, r: Result<Value, CoreError>) -> Response {
        let changes = self.doc.take_changes();
        let mut events = std::mem::take(&mut self.pending_events);
        events.extend(events_from(&changes));
        match r {
            Ok(result) => Response {
                ok: true,
                result,
                events,
                error: None,
                revision: self.doc.revision,
                can_undo: self.history.can_undo(),
                can_redo: self.history.can_redo(),
            },
            Err(e) => Response {
                ok: false,
                result: Value::Null,
                events,
                error: Some(e.into()),
                revision: self.doc.revision,
                can_undo: self.history.can_undo(),
                can_redo: self.history.can_redo(),
            },
        }
    }

    fn exec(&mut self, cmd: Command) -> Result<(), CoreError> {
        self.history.execute(&mut self.doc, cmd)
    }

    pub(crate) fn exec_cmd(&mut self, cmd: Command) -> Result<(), CoreError> {
        self.exec(cmd)
    }

    fn reset(&mut self, doc: Document) {
        self.doc = doc;
        self.doc.mark_loaded();
        self.history.clear();
        self.relations = None;
        self.joints = None;
        self.nesting.clear();
    }

    fn handle(&mut self, req: Request) -> Result<Value, CoreError> {
        use Request::*;
        match req {
            CreateProject { name } => {
                self.reset(Document::new(name));
                self.pending_events.push(CoreEvent::ProjectLoaded);
                ok(json!({}))
            }
            LoadProject { project } => {
                let file = ProjectFile::from_json(&project.to_string())?;
                let doc = file.into_document()?;
                self.reset(doc);
                self.pending_events.push(CoreEvent::ProjectLoaded);
                ok(json!({ "name": self.doc.meta.name }))
            }
            SaveProject => {
                let mut file = ProjectFile::from_document(&self.doc);
                file.relations = serde_json::to_value(self.relations().relations()).unwrap_or(Value::Null);
                ok(file)
            }
            CreateRoom { width, depth, height } => {
                let n = self.doc.objects.values().filter(|o| matches!(o, DomainObject::Room(_))).count() + 1;
                self.exec(Command::CreateRoom { id: None, name: format!("Room {n:02}"), width, depth, height })?;
                ok(json!({ "id": self.last_root() }))
            }
            CreateCabinet { kind, position, parent, overrides, name, room, floor, after } => {
                let mut spec = CabinetSpec::preset(kind);
                if let Some(r) = room {
                    spec.room = r.trim().to_string();
                }
                if let Some(f) = floor {
                    spec.floor = f.trim().to_string();
                }
                // Automatic name by frame kind: BếpDưới01, TủQA01, …
                let name = name.filter(|n| !n.trim().is_empty()).or_else(|| {
                    let prefix = kind.frame_name();
                    let taken: std::collections::BTreeSet<String> =
                        self.doc.objects.values().filter_map(|o| o.as_cabinet()).map(|c| c.name.clone()).collect();
                    (1..1000).map(|i| format!("{prefix}{i:02}")).find(|n| !taken.contains(n))
                });
                // Next to the given cabinet: its origin + its width along X.
                let position = match after {
                    Some(a) if self.doc.objects.get(&a).and_then(|o| o.as_cabinet()).is_some() => {
                        let w = self.doc.param_value(a, "width").unwrap_or(0.0);
                        Some(self.doc.scene.world(a).transform_point([w, 0.0, 0.0]))
                    }
                    // No anchor: in a room, continue that room's row (right of its last
                    // cabinet); a new room starts its own area right of everything else.
                    _ if position.is_none() => self.room_slot(&spec.floor, &spec.room),
                    _ => position,
                };
                let o = overrides;
                if let Some(v) = o.width { spec.width = v; }
                if let Some(v) = o.height { spec.height = v; }
                if let Some(v) = o.depth { spec.depth = v; }
                if let Some(v) = o.thickness { spec.thickness = v; }
                if let Some(v) = o.plinth_height { spec.plinth_height = v.max(0.0); }
                if let Some(v) = o.shelves { spec.shelves = v; }
                if let Some(v) = o.doors { spec.doors = v; }
                if let Some(v) = o.drawers { spec.drawers = v; }
                if let Some(v) = o.back_panel { spec.back_panel = v; }
                if let Some(v) = o.carcass_material { spec.carcass_material = MaterialId::new(v); }
                if let Some(v) = o.front_material { spec.front_material = MaterialId::new(v); }
                let style = |v: &str| match v.to_ascii_uppercase().as_str() {
                    "OVERLAY" => aic_domain::JoinStyle::Overlay,
                    "RAILS" => aic_domain::JoinStyle::Rails,
                    _ => aic_domain::JoinStyle::Inset,
                };
                if let Some(v) = &o.top_style { spec.top_style = style(v); }
                if let Some(v) = &o.bottom_style { spec.bottom_style = style(v); }
                if let Some(r) = o.edge_rule { spec.edge_rule = Some(r); }
                if let Some(g) = o.back_groove { spec.back_groove = g.max(0.0); }
                let id = self.doc.ids.clone().alloc();
                self.exec(Command::CreateCabinet { id: Some(id), spec, position: position.unwrap_or([0.0; 3]), parent, name })?;
                ok(json!({ "id": id }))
            }
            CreatePanel { name, width, height, thickness, material, transform, parent } => {
                let id = self.doc.ids.clone().alloc();
                self.exec(Command::CreatePanel {
                    id: Some(id),
                    name: name.unwrap_or_else(|| "Panel".into()),
                    size: [width, height, thickness],
                    material: MaterialId::new(material.unwrap_or_else(|| self.doc.settings.default_carcass_material.0.clone())),
                    transform: transform.unwrap_or_default(),
                    parent,
                })?;
                ok(json!({ "id": id }))
            }
            DeleteObjects { ids } => {
                let ids = self.top_level_only(ids);
                let (gen, plain): (Vec<ObjectId>, Vec<ObjectId>) = ids.iter().partition(|id| self.part_ref(**id).is_some());
                let mut cmds: Vec<Command> = self.delete_generated(&gen)?;
                cmds.extend(plain.iter().map(|id| Command::DeleteObject { id: *id }));
                self.exec(Command::Batch { label: "Delete".into(), commands: cmds })?;
                ok(json!({ "deleted": ids }))
            }
            DuplicateObjects { ids } => {
                let ids = self.top_level_only(ids);
                let before: std::collections::BTreeSet<ObjectId> = self.doc.objects.keys().copied().collect();
                let cmds = ids.iter().map(|id| Command::DuplicateObject { id: *id, offset: None }).collect();
                self.exec(Command::Batch { label: "Duplicate".into(), commands: cmds })?;
                let created: Vec<ObjectId> = self
                    .doc
                    .objects
                    .keys()
                    .copied()
                    .filter(|k| !before.contains(k))
                    .filter(|k| self.doc.scene.parent(*k).is_none_or(|p| before.contains(&p)))
                    .collect();
                ok(json!({ "created": created }))
            }
            SetParameter { id, name, value } => {
                self.set_parameter(id, &name, &value)?;
                ok(json!({}))
            }
            SetTransform { id, transform } => {
                self.exec(Command::SetTransform { id, transform })?;
                ok(json!({}))
            }
            SetName { id, name } => {
                self.exec(Command::SetName { id, name })?;
                ok(json!({}))
            }
            SetVisible { ids, visible } => {
                let cmds = ids.into_iter().map(|id| Command::SetVisible { id, visible }).collect();
                self.exec(Command::Batch { label: "Visibility".into(), commands: cmds })?;
                ok(json!({}))
            }
            SetLocked { ids, locked } => {
                let cmds = ids.into_iter().map(|id| Command::SetLocked { id, locked }).collect();
                self.exec(Command::Batch { label: "Lock".into(), commands: cmds })?;
                ok(json!({}))
            }
            Reparent { id, parent, index } => {
                self.exec(Command::Reparent { id, parent, index })?;
                ok(json!({}))
            }
            SetMaterial { id, material, slot } => {
                self.set_material(id, &material, slot.as_deref())?;
                ok(json!({}))
            }
            SetEdgeBand { id, edge, enabled } => {
                self.set_edge(id, edge, enabled)?;
                ok(json!({}))
            }
            GetCosting => ok(self.costing()?),
            SetPrice { key, value } => {
                self.exec(Command::SetPrice { key, value })?;
                ok(json!({}))
            }
            AddFeature { id, feature } => {
                self.exec(Command::AddFeature { id, feature, index: None })?;
                ok(json!({}))
            }
            RemoveFeature { id, index } => {
                self.exec(Command::RemoveFeature { id, index })?;
                ok(json!({}))
            }
            GetZones { cabinet } => ok(self.zones_info(cabinet)?),
            ZoneAddPanels(r) => ok(self.zone_add_panels(r)?),
            SplitZone(r) => ok(self.split_zone(r)?),
            ZoneAddDoors { cabinet, zones, kind, cols, rows, mount, hinge, thickness, stop } => {
                self.zone_set_front(cabinet, zones, Some(zones::default_door(kind, cols, rows, mount, hinge, thickness, stop)))?;
                ok(json!({}))
            }
            ZoneAddDrawers { cabinet, zones, count, cols, mount, thickness, with_box, inner, false_front } => {
                let mut f = zones::default_drawers(count, cols, mount, thickness, with_box && !false_front);
                if let aic_domain::zone::Front::Drawers(d) = &mut f {
                    d.inner = inner;
                    d.false_front = false_front;
                }
                self.zone_set_front(cabinet, zones, Some(f))?;
                ok(json!({}))
            }
            ZoneAddLink { cabinet, zones, kind, offset } => {
                self.zone_add_link(cabinet, zones, kind, offset)?;
                ok(json!({}))
            }
            ZoneRemove { cabinet, uid } => {
                self.zone_remove(cabinet, uid)?;
                ok(json!({}))
            }
            SetPartMod { id, patch } => {
                self.set_part_mod(id, patch)?;
                ok(json!({}))
            }
            SetBay { cabinet, zone, index, mode, value } => {
                self.set_bay(cabinet, zone, index, mode, value)?;
                ok(json!({}))
            }
            SetRelation { a, b, kind, gap } => ok(self.set_relation(a, b, kind, gap)?),
            ResizePanelSide { id, side, delta, constrained } => {
                self.resize_panel_side(id, side, delta, constrained)?;
                ok(json!({}))
            }
            MoveObjects { ids, delta } => {
                if delta.iter().any(|v| !v.is_finite()) {
                    return Err(CoreError::InvalidTransform);
                }
                let ids = self.top_level_only(ids);
                let cmds = ids.iter().map(|id| self.shift_world(*id, delta)).collect::<Result<Vec<_>, _>>()?;
                if !cmds.is_empty() {
                    self.exec(Command::Batch { label: "Di chuyển".into(), commands: cmds })?;
                }
                ok(json!({ "moved": ids.len() }))
            }
            ResizeCabinet { id, name, value, stretch, edge } => {
                self.resize_with_stretch(id, &name, value, stretch, edge)?;
                ok(json!({}))
            }
            SetDrawerHeight { cabinet, uid, index, mode, value } => {
                self.set_drawer_height(cabinet, uid, index, mode, value)?;
                ok(json!({}))
            }
            MoveDrawerDivider { cabinet, uid, index, before } => {
                self.move_drawer_divider(cabinet, uid, index, before)?;
                ok(json!({}))
            }
            EqualizeSplit { cabinet, zone } => {
                self.equalize_split(cabinet, zone)?;
                ok(json!({}))
            }
            MoveSplitPanel { id, before } => ok(self.move_split_panel(id, before)?),
            CreateRun { ids, rules } => ok(self.create_run(ids, rules)?),
            CreateCorner { hand, kind, wall, width, height, depth, door_width: _, position, room, floor, after } if kind.as_deref().is_some_and(|k| k.eq_ignore_ascii_case("DIAGONAL")) => {
                let hinge_left = !hand.eq_ignore_ascii_case("RIGHT");
                let w = width.unwrap_or(if wall { 600.0 } else { 900.0 });
                let d = depth.unwrap_or(if wall { 320.0 } else { 580.0 });
                if d > w - 150.0 {
                    return Err(CoreError::ConstraintViolated { constraint: "CORNER_DIAGONAL".into(), message: format!("depth {d} in {w}") });
                }
                let (room, floor) = match after.and_then(|a| self.doc.objects.get(&a)).and_then(|o| o.as_cabinet()) {
                    Some(c) => (room.or_else(|| Some(c.room.clone())), floor.or_else(|| Some(c.floor.clone()))),
                    None => (room, floor),
                };
                let mark = self.history.mark();
                let res = (|| -> Result<Value, CoreError> {
                    let overrides = protocol::CabinetOverrides { width: Some(w), height, depth: Some(d), doors: Some(0), shelves: Some(0), ..Default::default() };
                    let kind = if wall { aic_domain::CabinetKind::Wall } else { aic_domain::CabinetKind::Base };
                    let r = self.handle(Request::CreateCabinet { kind, position, parent: None, overrides, name: Some(if wall { "BếpTrênGócChéo".into() } else { "BếpGócChéo".into() }), room, floor, after })?;
                    let cab: ObjectId = serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
                    self.edit_cabinet(cab, "Tủ góc chéo", |c| {
                        c.rules.diagonal = Some(aic_domain::structure::DiagonalCorner { hinge_left, ..Default::default() });
                        Ok(())
                    })?;
                    Ok(json!({ "id": cab }))
                })();
                match res {
                    Ok(v) => {
                        self.history.squash(mark, "Tủ góc chéo");
                        ok(v)
                    }
                    Err(e) => {
                        self.history.rollback(&mut self.doc, mark);
                        Err(e)
                    }
                }
            }
            CreateCorner { hand, width, height, depth, door_width, position, room, floor, after, .. } => {
                let left = match hand.to_ascii_uppercase().as_str() {
                    "LEFT" => true,
                    "RIGHT" => false,
                    _ => return Err(CoreError::InvalidParameter { name: "hand".into(), reason: "LEFT | RIGHT".into() }),
                };
                let w = width.unwrap_or(1100.0);
                let dw = door_width.unwrap_or(450.0);
                if dw < 250.0 || dw > w - 200.0 {
                    return Err(CoreError::ConstraintViolated { constraint: "CORNER_DOOR".into(), message: format!("door {dw} in {w}") });
                }
                // Đặt cạnh tủ `after`: cùng phòng / tầng với tủ đó.
                let (room, floor) = match after.and_then(|a| self.doc.objects.get(&a)).and_then(|o| o.as_cabinet()) {
                    Some(c) => (room.or_else(|| Some(c.room.clone())), floor.or_else(|| Some(c.floor.clone()))),
                    None => (room, floor),
                };
                let mark = self.history.mark();
                let res = (|| -> Result<Value, CoreError> {
                    let overrides = protocol::CabinetOverrides { width: Some(w), height, depth, doors: Some(0), shelves: Some(0), ..Default::default() };
                    let r = self.handle(Request::CreateCabinet { kind: aic_domain::CabinetKind::Base, position, parent: None, overrides, name: Some(format!("BếpGóc{}", if left { "Trái" } else { "Phải" })), room, floor, after })?;
                    let cab: ObjectId = serde_json::from_value(r["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
                    let root = self.cabinet_def(cab)?.zones.root.id;
                    let t = self.doc.param_value(cab, "thickness").unwrap_or(17.2);
                    // Ô cánh = rộng cánh − phần phủ lên hồi (t − khe) + nửa khe giữa cánh và tấm mù.
                    let gap = self.doc.param_value(cab, "door_gap").unwrap_or(2.0);
                    let opening = (dw - (t - gap) + gap / 2.0).max(100.0);
                    self.split_zone(protocol::SplitZone { cabinet: cab, zone: root, kind: aic_domain::zone::SplitKind::VirtualV, formula: format!("{opening}"), from_end: left, thickness: None })?;
                    let layout = self.doc.cabinet_layout(cab).ok_or(CoreError::NotFound { id: cab })?;
                    let child = |i: usize| layout.bays.iter().find(|b| b.zone == root && b.index == i).map(|b| b.child);
                    let (blind, door) = if left { (child(0), child(1)) } else { (child(1), child(0)) };
                    let (Some(blind), Some(door)) = (blind, door) else { return Err(CoreError::NotFound { id: cab }) };
                    let hinge = if left { aic_domain::zone::HingeSide::Right } else { aic_domain::zone::HingeSide::Left };
                    self.zone_set_front(cab, vec![door], Some(zones::default_door(aic_domain::zone::DoorKind::Single, 1, 1, aic_domain::zone::Mount::Overlay, hinge, None, None)))?;
                    let mut mute = zones::default_door(aic_domain::zone::DoorKind::Single, 1, 1, aic_domain::zone::Mount::Overlay, hinge, None, None);
                    if let aic_domain::zone::Front::Doors(d) = &mut mute {
                        d.fixed = true;
                    }
                    self.zone_set_front(cab, vec![blind], Some(mute))?;
                    Ok(json!({ "id": cab }))
                })();
                match res {
                    Ok(v) => {
                        self.history.squash(mark, "Tủ góc L");
                        ok(v)
                    }
                    Err(e) => {
                        self.history.rollback(&mut self.doc, mark);
                        Err(e)
                    }
                }
            }
            UpdateRun { name, rules } => {
                self.update_run(&name, rules)?;
                ok(json!({}))
            }
            DeleteRun { name } => {
                self.delete_run(&name)?;
                ok(json!({}))
            }
            GetRuns => ok(self.get_runs()),
            GetMaterialSets => ok(self.material_sets_info()),
            SaveMaterialSet { cabinet, name } => {
                self.save_material_set(cabinet, &name)?;
                ok(json!({}))
            }
            ApplyMaterialSet { ids, room, name } => ok(json!({ "cabinets": self.apply_material_set(ids, room, &name)? })),
            DeleteMaterialSet { name } => {
                self.delete_material_set(&name)?;
                ok(json!({}))
            }
            ShapeTool { ids, op } => ok(self.shape_tool(&ids, &op)?),
            MergePanels { ids } => ok(self.merge_panels(&ids)?),
            Undo => ok(json!({ "label": self.history.undo(&mut self.doc)? })),
            Redo => ok(json!({ "label": self.history.redo(&mut self.doc)? })),
            GetSceneTree => ok(properties::scene_tree(&self.doc)),
            GetProperties { id } => ok(properties::properties(self, id)?),
            SaveTemplate { cabinet, name, to_library } => ok(self.save_template(cabinet, &name, to_library)?),
            InsertTemplate { name, width, height, depth, position, room, floor, after } => ok(self.insert_template(&name, [width, height, depth], position, room, floor, after)?),
            DeleteTemplate { name } => {
                self.delete_template(&name)?;
                ok(json!({}))
            }
            GetTemplates => ok(self.templates_info()),
            GetStructure { cabinet } => ok(self.get_structure(cabinet)?),
            SaveGroupPreset { cabinet, group, name } => {
                self.save_group_preset(cabinet, &group, &name)?;
                ok(json!({}))
            }
            ApplyGroupPreset { ids, group, name } => {
                self.apply_group_preset(&ids, &group, &name)?;
                ok(json!({}))
            }
            DeleteGroupPreset { group, name } => {
                self.delete_group_preset(&group, &name)?;
                ok(json!({}))
            }
            SaveZonePreset { cabinet, zone, name } => ok(self.save_zone_preset(cabinet, zone, &name)?),
            ApplyZonePreset { cabinet, zones, name } => {
                self.apply_zone_preset(cabinet, &zones, &name)?;
                ok(json!({}))
            }
            DeleteZonePreset { name } => {
                self.library_remove("zone", &name)?;
                ok(json!({}))
            }
            SaveRulePreset { cabinet, name, to_library } => {
                self.save_rule_preset(cabinet, &name, to_library)?;
                ok(json!({}))
            }
            DeleteRulePreset { name } => {
                self.delete_rule_preset(&name)?;
                ok(json!({}))
            }
            ApplyRulePreset { ids, name } => {
                self.apply_rule_preset(&ids, &name)?;
                ok(json!({}))
            }
            ArraySplitPanel { id, count } => ok(self.array_split_panel(id, count)?),
            ArrayCabinet { id, count, axis, gap } => ok(self.array_cabinet(id, count, axis, gap)?),
            MirrorCabinet { id } => {
                self.mirror_cabinet(id)?;
                ok(json!({}))
            }
            GetPropertiesMulti { ids } => ok(properties::properties_multi(self, &ids)?),
            SetParameterMulti { ids, name, value } => {
                let mark = self.history.mark();
                for id in &ids {
                    if let Err(e) = self.set_parameter(*id, &name, &value) {
                        self.history.rollback(&mut self.doc, mark);
                        return Err(e);
                    }
                }
                self.history.squash(mark, "Sửa nhiều đối tượng");
                ok(json!({ "changed": ids.len() }))
            }
            GetRenderObjects { ids, known_keys } => ok(self.render_objects(ids, &known_keys)?),
            GetMaterials => ok(&self.doc.materials),
            GetRelations { id } => {
                let g = self.relations();
                let rels = match id {
                    Some(id) => g.relations_of(id),
                    None => g.relations().to_vec(),
                };
                ok(json!({ "relations": rels, "names": self.names() }))
            }
            GetManufacturing { id } => ok(self.flat_panel(id)?),
            GetParts => ok(self.parts()),
            RunNesting { material, settings } => ok(self.run_nesting(material, settings.unwrap_or_default())?),
            GenerateCnc { material, sheet_id } => ok(self.generate_cnc(&material, sheet_id)?),
            Snap { id, delta, grid } => ok(self.snap(id, delta, grid)?),
            GetBounds { ids } => {
                let mut b = aic_math::Aabb::empty();
                for id in ids {
                    b = b.union(&self.doc.world_aabb(id));
                }
                ok(if b.is_empty() { Value::Null } else { json!({ "min": b.min, "max": b.max }) })
            }
            GetTransform { id } => {
                let node = self.doc.scene.node(id).map_err(CoreError::from)?;
                let local = node.local_transform;
                let parent_world = node.parent.map(|p| self.world(p)).unwrap_or_default();
                let world = self.world(id);
                ok(json!({
                    "local": local,
                    "world": world,
                    "parent_world": parent_world,
                    "world_matrix": world.to_matrix_col_major(),
                    "parent_matrix": parent_world.to_matrix_col_major(),
                }))
            }
            GetStatus => ok(json!({
                "name": self.doc.meta.name,
                "revision": self.doc.revision,
                "objects": self.doc.objects.len(),
                "panels": self.doc.objects.values().filter(|o| o.as_panel().is_some()).count(),
                "undo": self.history.undo_label(),
                "redo": self.history.redo_label(),
            })),
        }
    }

    /// Placement for a new cabinet of (`floor`, `room`): right of that room's rightmost
    /// cabinet, or (empty room) 1500 mm right of every cabinet on the floor, or (empty
    /// floor) its own area right of everything else.
    fn room_slot(&self, floor: &str, room: &str) -> Option<[f64; 3]> {
        let right = |id: ObjectId| -> [f64; 3] {
            let w = self.doc.param_value(id, "width").unwrap_or(0.0);
            self.doc.scene.world(id).transform_point([w, 0.0, 0.0])
        };
        let cabs: Vec<(ObjectId, &str, &str)> = self
            .doc
            .objects
            .iter()
            .filter_map(|(id, o)| o.as_cabinet().map(|c| (*id, c.floor.as_str(), c.room.as_str())))
            .collect();
        let pick = |filter: &dyn Fn(&str, &str) -> bool| {
            cabs.iter().filter(|(_, f, r)| filter(f, r)).map(|(id, _, _)| right(*id)).max_by(|a, b| a[0].total_cmp(&b[0]))
        };
        if let Some(p) = pick(&|f, r| f == floor && r == room) {
            return Some(p);
        }
        if let Some(p) = pick(&|f, _| f == floor) {
            return Some([p[0] + 1500.0, 0.0, 0.0]);
        }
        pick(&|_, _| true).map(|p| [p[0] + 3000.0, 0.0, 0.0])
    }

    fn last_root(&self) -> Option<ObjectId> {
        self.doc.scene.roots().last().copied()
    }

    /// Drop ids whose ancestor is also selected (they go with it).
    fn top_level_only(&self, ids: Vec<ObjectId>) -> Vec<ObjectId> {
        let set: std::collections::BTreeSet<ObjectId> = ids.iter().copied().collect();
        let mut out: Vec<ObjectId> = ids
            .into_iter()
            .filter(|id| self.doc.scene.find_ancestor(*id, |a| set.contains(&a)).is_none())
            .collect();
        out.dedup();
        out
    }

    fn names(&self) -> HashMap<ObjectId, String> {
        self.doc.objects.iter().map(|(k, o)| (*k, o.name().to_string())).collect()
    }

    /// Property-panel edits are routed here and turned into the right command.
    pub(crate) fn set_parameter_pub(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
        self.set_parameter(id, name, value)
    }

    fn set_parameter(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
        if self.set_zone_property(id, name, value)? {
            return Ok(());
        }
        match name {
            "name" => self.exec(Command::SetName { id, name: value.into() }),
            "rx" | "ry" | "rz" => {
                let v: f64 = value
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .parse()
                    .map_err(|_| CoreError::InvalidParameter { name: name.into(), reason: "expected degrees".into() })?;
                let mut t = self.doc.scene.node(id).map_err(CoreError::from)?.local_transform;
                t.rotation_deg[match name { "rx" => 0, "ry" => 1, _ => 2 }] = v;
                self.exec(Command::SetTransform { id, transform: t })
            }
            "material" | "carcass_material" => self.set_material(id, value, Some("carcass")),
            "front_material" => self.set_material(id, value, Some("front")),
            "back_material" => self.set_material(id, value, Some("back")),
            n if n.starts_with("edge_") => {
                let edge = match &n[5..] {
                    "left" => aic_domain::EdgeSide::Left,
                    "right" => aic_domain::EdgeSide::Right,
                    "top" => aic_domain::EdgeSide::Top,
                    "bottom" => aic_domain::EdgeSide::Bottom,
                    _ => return Err(CoreError::InvalidParameter { name: n.into(), reason: "unknown edge".into() }),
                };
                let enabled = matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes");
                self.set_edge(id, edge, enabled)
            }
            "width" | "height" | "depth" if self.doc.objects.get(&id).and_then(|o| o.as_cabinet()).is_some() => self.resize_cabinet(id, name, value),
            _ => self.exec(Command::SetParameter { id, name: name.into(), value: value.into() }),
        }
    }

    /// W / H / D of a cabinet: a parameter change (never a scale) plus an origin shift
    /// by the cabinet's anchor, refused when a zone would become unsolvable.
    fn resize_cabinet(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
        let axis = match name {
            "width" => 0,
            "height" => 1,
            _ => 2,
        };
        let set = Command::SetParameter { id, name: name.into(), value: value.into() };
        let Ok(new) = value.trim().trim_start_matches('=').trim().replace(',', ".").parse::<f64>() else {
            return self.exec(set); // expression: solved by the parametric engine
        };
        let old = self.doc.param_value(id, name).unwrap_or(new);
        let def = self.doc.objects.get(&id).and_then(|o| o.as_cabinet()).cloned().ok_or(CoreError::NotFound { id })?;
        let mut values = self.doc.cabinet_values(id);
        match axis {
            0 => values.width = new,
            1 => values.height = new,
            _ => values.depth = new,
        }
        let before = self.doc.cabinet_layout(id).map(|l| l.problems).unwrap_or_default();
        let after = aic_domain::build_cabinet(&def, values).problems;
        if after.iter().any(|z| !before.contains(z)) {
            return Err(CoreError::ConstraintViolated { constraint: "ZONE_TOO_SMALL".into(), message: format!("zones {after:?}") });
        }
        let anchor = [def.anchors.width, def.anchors.height, def.anchors.depth][axis];
        let f = anchor.factor();
        let delta = new - old;
        if delta.abs() < 1e-9 {
            return self.exec(set);
        }
        let mut cmds = vec![set];
        if f != 0.0 {
            let mut t = self.doc.scene.node(id).map_err(CoreError::from)?.local_transform;
            let mut off = [0.0; 3];
            off[axis] = -delta * f;
            t.translation = t.transform_point(off);
            cmds.push(Command::SetTransform { id, transform: t });
        }
        // Dãy tủ: cabinets standing flush in the same row follow a width change — the
        // ones on the growing side are pushed / pulled so the row stays closed.
        if axis == 0 {
            let dir = self.doc.scene.world(id).transform_vector([1.0, 0.0, 0.0]);
            for (chain, amount) in [(self.row_chain(id, true), delta * (1.0 - f)), (self.row_chain(id, false), -delta * f)] {
                if amount.abs() < 1e-9 {
                    continue;
                }
                for c in chain {
                    cmds.push(self.shift_world(c, [dir[0] * amount, dir[1] * amount, dir[2] * amount])?);
                }
            }
        }
        self.exec(Command::Batch { label: "Đổi kích thước tủ".into(), commands: cmds })
    }

    /// Đổi kích thước tủ + chế độ dãn khoang (one undo step). The bays of every split
    /// along the axis (and drawer fronts for the height) are first rewritten so the
    /// change goes where the user wants: all bays by their current ratio, or only the
    /// bay at the dragged edge; then the normal resize runs.
    fn resize_with_stretch(&mut self, id: ObjectId, name: &str, value: f64, stretch: protocol::Stretch, edge: Option<aic_domain::Anchor>) -> Result<(), CoreError> {
        use aic_domain::zone::{Bay, Front};
        let axis = match name {
            "width" => 0,
            "height" => 1,
            "depth" => 2,
            _ => return Err(CoreError::InvalidParameter { name: name.into(), reason: "width | height | depth".into() }),
        };
        if stretch == protocol::Stretch::Keep {
            return self.set_parameter(id, name, &format!("{value}"));
        }
        let def = self.doc.objects.get(&id).and_then(|o| o.as_cabinet()).cloned().ok_or(CoreError::NotFound { id })?;
        let layout = self.doc.cabinet_layout(id).ok_or(CoreError::NotFound { id })?;
        // Which end grows: the dragged edge, else the side opposite the anchor.
        let anchor = [def.anchors.width, def.anchors.height, def.anchors.depth][axis];
        let grows = edge.unwrap_or(match anchor {
            aic_domain::Anchor::Start => aic_domain::Anchor::End,
            aic_domain::Anchor::End => aic_domain::Anchor::Start,
            aic_domain::Anchor::Center => aic_domain::Anchor::Center,
        });
        let is_edge = |i: usize, n: usize| match grows {
            aic_domain::Anchor::End => i + 1 == n,
            aic_domain::Anchor::Start => i == 0,
            aic_domain::Anchor::Center => i == 0 || i + 1 == n,
        };
        let rewrite = |sizes: &[f64]| -> Vec<Bay> {
            let usable: f64 = sizes.iter().sum();
            sizes
                .iter()
                .enumerate()
                .map(|(i, s)| match stretch {
                    protocol::Stretch::Proportional => Bay::percent(if usable > 0.0 { s / usable * 100.0 } else { 0.0 }),
                    _ => if is_edge(i, sizes.len()) { Bay::auto() } else { Bay::lock(*s) },
                })
                .collect()
        };
        let mut splits: std::collections::BTreeMap<u32, Vec<(usize, f64)>> = Default::default();
        for b in layout.bays.iter().filter(|b| b.axis == axis) {
            splits.entry(b.zone).or_default().push((b.index, b.size));
        }
        let mut stacks: std::collections::BTreeMap<u32, Vec<(usize, f64)>> = Default::default();
        if axis == 1 {
            for f in &layout.front_bays {
                let v = stacks.entry(f.uid).or_default();
                if !v.iter().any(|x| x.0 == f.index) {
                    v.push((f.index, f.size));
                }
            }
        }
        let mark = self.history.mark();
        let res = self
            .edit_cabinet(id, "Chế độ dãn khoang", |c| {
                for (zone, mut v) in splits {
                    v.sort_by_key(|x| x.0);
                    let sizes: Vec<f64> = v.into_iter().map(|x| x.1).collect();
                    if let Some(s) = c.zones.split_mut(zone) {
                        if sizes.len() == s.panels.len() + 1 {
                            s.bays = rewrite(&sizes);
                        }
                    }
                }
                for (uid, mut v) in stacks {
                    v.sort_by_key(|x| x.0);
                    let sizes: Vec<f64> = v.into_iter().map(|x| x.1).collect();
                    if let Some(Front::Drawers(d)) = c.zones.front_mut(uid) {
                        if sizes.len() == d.count as usize {
                            d.heights = rewrite(&sizes);
                        }
                    }
                }
                Ok(())
            })
            .and_then(|_| self.set_parameter(id, name, &format!("{value}")));
        match res {
            Ok(()) => {
                self.history.squash(mark, "Đổi kích thước tủ");
                Ok(())
            }
            Err(e) => {
                self.history.rollback(&mut self.doc, mark);
                Err(e)
            }
        }
    }

    /// Cabinets touching `id` in a row (same rotation, same height level and depth
    /// line), walking to the right (`right`) or the left, neighbour after neighbour.
    fn row_chain(&self, id: ObjectId, right: bool) -> Vec<ObjectId> {
        let near = |a: [f64; 3], b: [f64; 3]| (0..3).all(|k| (a[k] - b[k]).abs() < 2.0);
        let cabs: Vec<ObjectId> = self.doc.objects.iter().filter(|(_, o)| o.as_cabinet().is_some()).map(|(k, _)| *k).collect();
        let width = |c: ObjectId| self.doc.param_value(c, "width").unwrap_or(0.0);
        let x_of = |c: ObjectId| self.doc.scene.world(c).transform_vector([1.0, 0.0, 0.0]);
        let x0 = x_of(id);
        let mut out = Vec::new();
        let mut cur = id;
        loop {
            let w = self.doc.scene.world(cur);
            let next = cabs.iter().copied().find(|&c| {
                if c == id || out.contains(&c) {
                    return false;
                }
                let xc = x_of(c);
                if xc[0] * x0[0] + xc[1] * x0[1] + xc[2] * x0[2] < 0.999 {
                    return false;
                }
                let wc = self.doc.scene.world(c);
                if right {
                    near(wc.translation, w.transform_point([width(cur), 0.0, 0.0]))
                } else {
                    near(wc.transform_point([width(c), 0.0, 0.0]), w.translation)
                }
            });
            match next {
                Some(c) => {
                    out.push(c);
                    cur = c;
                }
                None => return out,
            }
        }
    }

    /// SetTransform that moves an object by a world-space vector.
    fn shift_world(&self, id: ObjectId, v: [f64; 3]) -> Result<Command, CoreError> {
        let mut t = self.doc.scene.node(id).map_err(CoreError::from)?.local_transform;
        let local = match self.doc.scene.parent(id) {
            Some(p) => self.doc.scene.world(p).inverse().transform_vector(v),
            None => v,
        };
        for k in 0..3 {
            t.translation[k] += local[k];
        }
        Ok(Command::SetTransform { id, transform: t })
    }

    /// Edge band on/off: generated parts store a manual override in the cabinet's part mods.
    fn set_edge(&mut self, id: ObjectId, edge: aic_domain::EdgeSide, enabled: bool) -> Result<(), CoreError> {
        if let Some((cab, key)) = self.part_ref(id) {
            return self.edit_cabinet(cab, "Dán cạnh", |c| {
                c.mods.entry(key).or_default().edges.insert(edge, enabled);
                Ok(())
            });
        }
        let band = enabled.then(|| EdgeBand { edge, material_code: "DON-1".into(), thickness_mm: 1.0 });
        self.exec(Command::SetEdgeBand { id, edge, band })
    }

    pub(crate) fn set_material(&mut self, id: ObjectId, material: &str, slot: Option<&str>) -> Result<(), CoreError> {
        let material = MaterialId::new(material);
        match self.doc.object(id)? {
            DomainObject::Panel(_) => self.exec(Command::SetMaterial { id, material }),
            DomainObject::Cabinet(_) => {
                let slot = match slot.unwrap_or("carcass") {
                    "front" => MaterialSlot::Front,
                    "back" => MaterialSlot::Back,
                    _ => MaterialSlot::Carcass,
                };
                let in_slot = |r: PanelRole| match slot {
                    MaterialSlot::Front => matches!(r, PanelRole::Door | PanelRole::DrawerFront),
                    MaterialSlot::Back => r == PanelRole::Back,
                    MaterialSlot::Carcass => !matches!(r, PanelRole::Door | PanelRole::DrawerFront | PanelRole::Back),
                };
                let mut cmds: Vec<Command> = self
                    .doc
                    .scene
                    .subtree(id)
                    .into_iter()
                    .filter(|c| self.doc.panel(*c).is_some_and(|p| in_slot(p.role)))
                    .map(|c| Command::SetMaterial { id: c, material: material.clone() })
                    .collect();
                cmds.push(Command::SetCabinetSlot { id, slot, material });
                self.exec(Command::Batch { label: "Material".into(), commands: cmds })
            }
            _ => Err(CoreError::InvalidParameter { name: "material".into(), reason: "object has no material".into() }),
        }
    }

    fn snap(&mut self, id: ObjectId, delta: [f64; 3], grid: Option<f64>) -> Result<Value, CoreError> {
        self.doc.object(id)?;
        let moving = self.doc.world_aabb(id);
        if moving.is_empty() {
            return ok(json!({ "delta": delta, "hints": [] }));
        }
        let own: std::collections::BTreeSet<ObjectId> = self.doc.scene.subtree(id).into_iter().collect();
        let others: Vec<(ObjectId, aic_math::Aabb)> = self
            .doc
            .objects
            .keys()
            .copied()
            .filter(|o| !own.contains(o) && self.doc.scene.is_effectively_visible(*o))
            .filter_map(|o| self.doc.world_obb(o).map(|b| (o, b.aabb())))
            .collect();
        let settings = aic_spatial::SnapSettings { tolerance_mm: 20.0, grid_mm: grid.or(Some(10.0)) };
        ok(aic_spatial::snap_translation(&moving, delta, &others, &settings))
    }

    pub(crate) fn world(&self, id: ObjectId) -> Transform3D {
        self.doc.scene.world(id)
    }
}

fn events_from(c: &ChangeSet) -> Vec<CoreEvent> {
    let v = |s: &std::collections::BTreeSet<ObjectId>| s.iter().copied().collect::<Vec<_>>();
    let mut out = Vec::new();
    if c.loaded {
        out.push(CoreEvent::ProjectLoaded);
    }
    if !c.created.is_empty() {
        out.push(CoreEvent::ObjectCreated { ids: v(&c.created) });
    }
    if !c.deleted.is_empty() {
        out.push(CoreEvent::ObjectDeleted { ids: v(&c.deleted) });
        out.push(CoreEvent::SelectionInvalidated { ids: v(&c.deleted) });
    }
    let changed: Vec<ObjectId> = c.changed.iter().copied().filter(|i| !c.created.contains(i)).collect();
    if !changed.is_empty() {
        out.push(CoreEvent::ObjectChanged { ids: changed });
    }
    let geometry: Vec<ObjectId> = c.geometry.iter().copied().filter(|i| !c.created.contains(i)).collect();
    if !geometry.is_empty() {
        out.push(CoreEvent::GeometryChanged { ids: geometry });
    }
    if !c.transform.is_empty() {
        out.push(CoreEvent::TransformChanged { ids: v(&c.transform) });
    }
    if c.tree {
        out.push(CoreEvent::SceneTreeChanged);
    }
    out
}

#[cfg(test)]
mod tests;
