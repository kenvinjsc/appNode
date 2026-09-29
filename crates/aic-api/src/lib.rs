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
mod render;

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
    pub(crate) relation_settings: RelationSettings,
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
            relation_settings: RelationSettings::default(),
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
        let r = self.handle(req);
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
            CreateCabinet { kind, position, parent, overrides, name } => {
                let mut spec = CabinetSpec::preset(kind);
                let o = overrides;
                if let Some(v) = o.width { spec.width = v; }
                if let Some(v) = o.height { spec.height = v; }
                if let Some(v) = o.depth { spec.depth = v; }
                if let Some(v) = o.thickness { spec.thickness = v; }
                if let Some(v) = o.shelves { spec.shelves = v; }
                if let Some(v) = o.doors { spec.doors = v; }
                if let Some(v) = o.drawers { spec.drawers = v; }
                if let Some(v) = o.back_panel { spec.back_panel = v; }
                if let Some(v) = o.carcass_material { spec.carcass_material = MaterialId::new(v); }
                if let Some(v) = o.front_material { spec.front_material = MaterialId::new(v); }
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
                let cmds = ids.iter().map(|id| Command::DeleteObject { id: *id }).collect();
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
                let band = enabled.then(|| EdgeBand { edge, material_code: "ABS-1".into(), thickness_mm: 1.0 });
                self.exec(Command::SetEdgeBand { id, edge, band })?;
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
            Undo => ok(json!({ "label": self.history.undo(&mut self.doc)? })),
            Redo => ok(json!({ "label": self.history.redo(&mut self.doc)? })),
            GetSceneTree => ok(properties::scene_tree(&self.doc)),
            GetProperties { id } => ok(properties::properties(self, id)?),
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
    fn set_parameter(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
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
                let band = enabled.then(|| EdgeBand { edge, material_code: "ABS-1".into(), thickness_mm: 1.0 });
                self.exec(Command::SetEdgeBand { id, edge, band })
            }
            _ => self.exec(Command::SetParameter { id, name: name.into(), value: value.into() }),
        }
    }

    fn set_material(&mut self, id: ObjectId, material: &str, slot: Option<&str>) -> Result<(), CoreError> {
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
