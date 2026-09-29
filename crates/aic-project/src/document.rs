use crate::{CoreError, Snapshot};
use aic_domain::cabinet::{self, base_params, material_for, CabinetStructure};
use aic_domain::{
    default_materials, Cabinet, CabinetSpec, DomainObject, Hardware, IdAllocator, Material, MaterialId, ObjectId, Panel, PanelRole,
    Room, Scene, SceneNode,
};
use aic_math::{Aabb, Obb, Transform3D};
use aic_parametric::{parse, Constraint, ParamGraph, ParamKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub unit: String,
    pub default_carcass_material: MaterialId,
    pub default_front_material: MaterialId,
    pub default_back_material: MaterialId,
    pub max_relation_gap_mm: f64,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            unit: "mm".into(),
            default_carcass_material: MaterialId::new("MDF18-WHITE"),
            default_front_material: MaterialId::new("MDF18-OAK"),
            default_back_material: MaterialId::new("HDF5-WHITE"),
            max_relation_gap_mm: 5.0,
        }
    }
}

/// What changed since the last drain (turned into UI events by the API layer).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChangeSet {
    pub created: BTreeSet<ObjectId>,
    pub deleted: BTreeSet<ObjectId>,
    /// Properties changed (name, params, visibility, ...).
    pub changed: BTreeSet<ObjectId>,
    /// Definition-space geometry changed (needs re-tessellation).
    pub geometry: BTreeSet<ObjectId>,
    /// World transform changed (only a matrix update on the render side).
    pub transform: BTreeSet<ObjectId>,
    pub tree: bool,
    pub loaded: bool,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        *self == ChangeSet::default()
    }
}

/// Parameter names that change a cabinet's structure (regeneration, not a solve).
pub const STRUCTURAL_PARAMS: &[&str] = &["shelves", "doors", "drawers", "back_panel", "top_style", "bottom_style"];

#[derive(Debug, Clone)]
pub struct Document {
    pub meta: ProjectMeta,
    pub settings: ProjectSettings,
    pub scene: Scene,
    pub objects: BTreeMap<ObjectId, DomainObject>,
    pub params: ParamGraph,
    pub materials: Vec<Material>,
    pub ids: IdAllocator,
    /// Parameter keys explicitly set by the user (survive cabinet regeneration).
    pub overrides: BTreeSet<ParamKey>,
    /// Objects created by a cabinet generator (matched on regeneration).
    pub generated: BTreeSet<ObjectId>,
    pub revision: u64,
    changes: ChangeSet,
}

pub fn key(owner: ObjectId, name: &str) -> ParamKey {
    format!("#{}.{}", owner.0, name)
}

pub fn split_key(k: &str) -> Option<(ObjectId, &str)> {
    let rest = k.strip_prefix('#')?;
    let (id, name) = rest.split_once('.')?;
    Some((ObjectId(id.parse().ok()?), name))
}

fn fmt_num(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    if r == r.trunc() && r.abs() < 1e15 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            meta: ProjectMeta { name: name.into(), description: String::new() },
            settings: ProjectSettings::default(),
            scene: Scene::new(),
            objects: BTreeMap::new(),
            params: ParamGraph::new(),
            materials: default_materials(),
            ids: IdAllocator::default(),
            overrides: BTreeSet::new(),
            generated: BTreeSet::new(),
            revision: 0,
            changes: ChangeSet::default(),
        }
    }

    // ------------------------------------------------------------------ access

    pub fn object(&self, id: ObjectId) -> Result<&DomainObject, CoreError> {
        self.objects.get(&id).ok_or(CoreError::NotFound { id })
    }

    fn object_mut(&mut self, id: ObjectId) -> Result<&mut DomainObject, CoreError> {
        self.objects.get_mut(&id).ok_or(CoreError::NotFound { id })
    }

    pub fn panel(&self, id: ObjectId) -> Option<&Panel> {
        self.objects.get(&id).and_then(|o| o.as_panel())
    }

    pub fn material(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.iter().find(|m| &m.id == id)
    }

    pub fn param_value(&self, owner: ObjectId, name: &str) -> Option<f64> {
        self.params.value(&key(owner, name))
    }

    pub fn param_source(&self, owner: ObjectId, name: &str) -> Option<&str> {
        self.params.get(&key(owner, name)).map(|p| p.source.as_str())
    }

    pub fn ensure_unlocked(&self, id: ObjectId) -> Result<(), CoreError> {
        if !self.scene.contains(id) {
            return Err(CoreError::NotFound { id });
        }
        if self.scene.is_effectively_locked(id) {
            return Err(CoreError::Locked { id });
        }
        Ok(())
    }

    pub fn cabinet_of(&self, id: ObjectId) -> Option<ObjectId> {
        if matches!(self.objects.get(&id), Some(DomainObject::Cabinet(_))) {
            return Some(id);
        }
        self.scene.find_ancestor(id, |a| matches!(self.objects.get(&a), Some(DomainObject::Cabinet(_))))
    }

    /// For doors: whether the hinges go on the left edge.
    pub fn hinge_left(&self, id: ObjectId) -> Option<bool> {
        let p = self.panel(id)?;
        if p.role != PanelRole::Door {
            return None;
        }
        let doors = self.cabinet_of(id).and_then(|c| self.objects.get(&c)).and_then(|o| o.as_cabinet()).map(|c| c.doors).unwrap_or(1);
        Some(doors == 1 || p.role_index % 2 == 0)
    }

    /// Oriented box of an object in world space (panels and hardware).
    pub fn world_obb(&self, id: ObjectId) -> Option<Obb> {
        let size = match self.objects.get(&id)? {
            DomainObject::Panel(p) => p.size(),
            DomainObject::Hardware(h) => h.size_mm,
            _ => return None,
        };
        Some(Obb::new(size, self.scene.world(id)))
    }

    /// World AABB of an object and its whole subtree.
    pub fn world_aabb(&self, id: ObjectId) -> Aabb {
        let mut b = Aabb::empty();
        for n in self.scene.subtree(id) {
            if let Some(o) = self.world_obb(n) {
                b = b.union(&o.aabb());
            } else if let Some(DomainObject::Room(r)) = self.objects.get(&n) {
                let w = self.scene.world(n);
                b = b.union(&Obb::new([r.width_mm, r.height_mm, r.depth_mm], w).aabb());
            }
        }
        b
    }

    // --------------------------------------------------------------- changes

    pub fn take_changes(&mut self) -> ChangeSet {
        let updated = self.scene.update_world();
        let mut c = std::mem::take(&mut self.changes);
        for id in updated {
            if !c.created.contains(&id) {
                c.transform.insert(id);
            }
        }
        for id in &c.deleted {
            c.transform.remove(id);
            c.changed.remove(id);
            c.geometry.remove(id);
        }
        if !c.is_empty() {
            self.revision += 1;
        }
        c
    }

    pub fn peek_changes(&self) -> &ChangeSet {
        &self.changes
    }

    pub(crate) fn mark_changed(&mut self, id: ObjectId) {
        self.changes.changed.insert(id);
    }

    pub(crate) fn mark_geometry(&mut self, id: ObjectId) {
        self.changes.geometry.insert(id);
        self.changes.changed.insert(id);
    }

    pub(crate) fn mark_tree(&mut self) {
        self.changes.tree = true;
    }

    pub fn mark_loaded(&mut self) {
        self.changes.loaded = true;
        self.changes.tree = true;
    }

    // -------------------------------------------------------------- params

    /// Resolve a reference written in an expression owned by `owner`.
    pub fn resolve_ref(&self, owner: ObjectId, name: &str) -> Result<ParamKey, String> {
        if name.starts_with('#') {
            return if split_key(name).is_some() { Ok(name.to_string()) } else { Err(format!("bad reference '{name}'")) };
        }
        match name.split_once('.') {
            None => Ok(key(owner, name)),
            Some((scope, rest)) => {
                let target = match scope {
                    "self" => Some(owner),
                    "parent" => self.scene.parent(owner),
                    "cabinet" => self.cabinet_of(owner),
                    "room" => {
                        let is_room = |i: ObjectId| matches!(self.objects.get(&i), Some(DomainObject::Room(_)));
                        if is_room(owner) { Some(owner) } else { self.scene.find_ancestor(owner, is_room) }
                    }
                    _ => return Err(format!("unknown scope '{scope}'")),
                };
                target.map(|t| key(t, rest)).ok_or_else(|| format!("'{name}' has no {scope}"))
            }
        }
    }

    fn compile(&self, owner: ObjectId, source: &str) -> Result<aic_parametric::Expr, CoreError> {
        let e = parse(source).map_err(|e| CoreError::InvalidParameter { name: source.into(), reason: e.to_string() })?;
        e.map_refs(&mut |r| self.resolve_ref(owner, r))
            .map_err(|reason| CoreError::InvalidParameter { name: source.into(), reason })
    }

    /// Define/redefine parameters of objects, then push values into the domain.
    pub(crate) fn define_params(&mut self, defs: Vec<(ObjectId, String, String)>) -> Result<(), CoreError> {
        if defs.is_empty() {
            return Ok(());
        }
        let mut compiled = Vec::with_capacity(defs.len());
        for (owner, name, source) in &defs {
            let expr = self.compile(*owner, source).map_err(|e| match e {
                CoreError::InvalidParameter { reason, .. } => CoreError::InvalidParameter { name: name.clone(), reason },
                e => e,
            })?;
            compiled.push((key(*owner, name), source.clone(), expr));
        }
        let first = defs[0].1.clone();
        let changed = self.params.set_many(compiled).map_err(|e| CoreError::from_param(&first, e))?;
        self.apply_param_values(&changed);
        Ok(())
    }

    pub(crate) fn remove_params_of(&mut self, owner: ObjectId) -> Vec<(String, String)> {
        let prefix = format!("#{}.", owner.0);
        let keys: Vec<ParamKey> = self.params.keys_with_prefix(&prefix).cloned().collect();
        let saved: Vec<(String, String)> = keys
            .iter()
            .filter_map(|k| self.params.get(k).map(|p| (split_key(k).unwrap().1.to_string(), p.source.clone())))
            .collect();
        let dirty = self.params.remove(&keys);
        self.params.remove_constraints_with_prefix(&prefix);
        self.apply_param_values(&dirty);
        saved
    }

    pub(crate) fn add_constraint(&mut self, owner: ObjectId, id: &str, source: &str, code: &str, message: &str) -> Result<(), CoreError> {
        let expr = self.compile(owner, source)?;
        self.params.add_constraint(Constraint {
            id: format!("#{}.{}", owner.0, id),
            expr,
            source: source.into(),
            code: code.into(),
            message: message.into(),
        });
        Ok(())
    }

    /// Push computed parameter values into domain fields / scene transforms.
    pub(crate) fn apply_param_values(&mut self, keys: &[ParamKey]) {
        for k in keys {
            let Some((owner, name)) = split_key(k) else { continue };
            let Some(v) = self.params.value(k) else { continue };
            let Some(obj) = self.objects.get_mut(&owner) else { continue };
            let mut geometry = false;
            let mut changed = true;
            match (obj, name) {
                (DomainObject::Panel(p), "width") => {
                    geometry = p.width_mm != v;
                    p.width_mm = v;
                }
                (DomainObject::Panel(p), "height") => {
                    geometry = p.height_mm != v;
                    p.height_mm = v;
                }
                (DomainObject::Panel(p), "thickness") => {
                    geometry = p.thickness_mm != v;
                    p.thickness_mm = v;
                }
                (DomainObject::Hardware(h), "length") => {
                    geometry = h.size_mm[0] != v;
                    h.size_mm[0] = v;
                }
                (DomainObject::Room(r), "width") => {
                    geometry = r.width_mm != v;
                    r.width_mm = v;
                }
                (DomainObject::Room(r), "depth") => {
                    geometry = r.depth_mm != v;
                    r.depth_mm = v;
                }
                (DomainObject::Room(r), "height") => {
                    geometry = r.height_mm != v;
                    r.height_mm = v;
                }
                (_, "x" | "y" | "z") => {
                    changed = false;
                    let axis = match name {
                        "x" => 0,
                        "y" => 1,
                        _ => 2,
                    };
                    if let Ok(node) = self.scene.node(owner) {
                        let mut t = node.local_transform;
                        if t.translation[axis] != v {
                            t.translation[axis] = v;
                            let _ = self.scene.set_local_transform(owner, t);
                        }
                    }
                }
                _ => {}
            }
            if geometry {
                self.mark_geometry(owner);
            } else if changed {
                self.mark_changed(owner);
            }
        }
    }

    // ---------------------------------------------------- object construction

    fn insert_object(&mut self, obj: DomainObject, parent: Option<ObjectId>, t: Transform3D, index: Option<usize>) -> Result<ObjectId, CoreError> {
        let id = obj.id();
        self.ids.reserve(id);
        self.scene.insert(SceneNode::new(id, t), parent, index)?;
        self.objects.insert(id, obj);
        self.changes.created.insert(id);
        self.changes.deleted.remove(&id);
        self.mark_tree();
        Ok(id)
    }

    fn position_params(id: ObjectId, pos: &[String; 3]) -> Vec<(ObjectId, String, String)> {
        vec![(id, "x".into(), pos[0].clone()), (id, "y".into(), pos[1].clone()), (id, "z".into(), pos[2].clone())]
    }

    pub(crate) fn create_room(&mut self, id: Option<ObjectId>, name: String, w: f64, d: f64, h: f64) -> Result<ObjectId, CoreError> {
        let id = id.unwrap_or_else(|| self.ids.alloc());
        let room = Room { id, name, width_mm: w, depth_mm: d, height_mm: h, wall_thickness_mm: 100.0 };
        self.insert_object(DomainObject::Room(room), None, Transform3D::IDENTITY, None)?;
        let mut defs = Self::position_params(id, &["0".into(), "0".into(), "0".into()]);
        defs.push((id, "width".into(), fmt_num(w)));
        defs.push((id, "depth".into(), fmt_num(d)));
        defs.push((id, "height".into(), fmt_num(h)));
        self.define_params(defs)?;
        Ok(id)
    }

    pub(crate) fn create_panel(
        &mut self,
        id: Option<ObjectId>,
        name: String,
        size: [f64; 3],
        material: MaterialId,
        transform: Transform3D,
        parent: Option<ObjectId>,
    ) -> Result<ObjectId, CoreError> {
        if size.iter().any(|v| !(*v > 0.0) || !v.is_finite()) {
            return Err(CoreError::InvalidParameter { name: "size".into(), reason: "dimensions must be > 0".into() });
        }
        if self.material(&material).is_none() {
            return Err(CoreError::UnknownMaterial { material: material.0 });
        }
        let id = id.unwrap_or_else(|| self.ids.alloc());
        let panel = Panel::new(id, name, PanelRole::Generic, size, material);
        self.insert_object(DomainObject::Panel(panel), parent, transform, None)?;
        let t = transform.translation;
        let mut defs = Self::position_params(id, &t.map(fmt_num));
        defs.push((id, "width".into(), fmt_num(size[0])));
        defs.push((id, "height".into(), fmt_num(size[1])));
        defs.push((id, "thickness".into(), fmt_num(size[2])));
        self.define_params(defs)?;
        Ok(id)
    }

    pub(crate) fn create_cabinet(
        &mut self,
        id: Option<ObjectId>,
        spec: &CabinetSpec,
        position: [f64; 3],
        parent: Option<ObjectId>,
        name: Option<String>,
    ) -> Result<ObjectId, CoreError> {
        for m in [&spec.carcass_material, &spec.front_material, &spec.back_material] {
            if self.material(m).is_none() {
                return Err(CoreError::UnknownMaterial { material: m.0.clone() });
            }
        }
        let id = id.unwrap_or_else(|| self.ids.alloc());
        let cab = Cabinet {
            id,
            name: name.unwrap_or_else(|| spec.kind.label().to_string()),
            kind: spec.kind,
            shelves: spec.shelves,
            doors: spec.doors,
            drawers: spec.drawers,
            back_panel: spec.back_panel,
            top_style: spec.top_style,
            bottom_style: spec.bottom_style,
            carcass_material: spec.carcass_material.clone(),
            front_material: spec.front_material.clone(),
            back_material: spec.back_material.clone(),
        };
        self.insert_object(DomainObject::Cabinet(cab), parent, Transform3D::from_translation(position[0], position[1], position[2]), None)?;
        let mut defs = Self::position_params(id, &position.map(fmt_num));
        defs.extend(base_params(spec).into_iter().map(|(n, s)| (id, n, s)));
        let result = self.define_params(defs).and_then(|_| self.regenerate_cabinet(id));
        if let Err(e) = result {
            // Roll back the partially created cabinet.
            let _ = self.remove_subtree(id);
            self.changes.created.remove(&id);
            self.changes.deleted.remove(&id);
            return Err(e);
        }
        Ok(id)
    }

    /// (Re)build the children of a cabinet from its structure. Generated children
    /// are matched by (role, index) so their ids — and user overrides — are stable.
    pub(crate) fn regenerate_cabinet(&mut self, id: ObjectId) -> Result<(), CoreError> {
        let cab = self.object(id)?.as_cabinet().cloned().ok_or(CoreError::NotFound { id })?;
        let st = CabinetStructure {
            kind: cab.kind,
            shelves: cab.shelves,
            doors: cab.doors,
            drawers: cab.drawers,
            back_panel: cab.back_panel,
            top_style: cab.top_style,
            bottom_style: cab.bottom_style,
        };
        let layout = cabinet::generate(&st);

        // Derived params + constraints of the cabinet itself.
        self.params.remove_constraints_with_prefix(&format!("#{}.", id.0));
        let derived: Vec<(ObjectId, String, String)> = layout.derived.iter().map(|(n, s)| (id, n.clone(), s.clone())).collect();
        self.define_params(derived)?;

        // Index existing generated children.
        let mut existing_panels: BTreeMap<(PanelRole, u32), ObjectId> = BTreeMap::new();
        let mut existing_hw: BTreeMap<(aic_domain::HardwareKind, u32), ObjectId> = BTreeMap::new();
        for c in self.scene.children(id).to_vec() {
            if !self.generated.contains(&c) {
                continue;
            }
            match self.objects.get(&c) {
                Some(DomainObject::Panel(p)) => {
                    existing_panels.insert((p.role, p.role_index), c);
                }
                Some(DomainObject::Hardware(h)) => {
                    existing_hw.insert((h.kind, h.role_index), c);
                }
                _ => {}
            }
        }

        let mut defs: Vec<(ObjectId, String, String)> = Vec::new();
        let mut keep: BTreeSet<ObjectId> = BTreeSet::new();
        for t in &layout.panels {
            let material = material_for(t.material, &cab.carcass_material, &cab.front_material, &cab.back_material);
            let pid = match existing_panels.get(&(t.role, t.index)) {
                Some(pid) => *pid,
                None => {
                    let pid = self.ids.alloc();
                    let mut p = Panel::new(pid, t.name.clone(), t.role, [1.0, 1.0, 1.0], material);
                    p.role_index = t.index;
                    p.grain_direction = t.grain;
                    self.insert_object(DomainObject::Panel(p), Some(id), Transform3D::new([0.0; 3], t.rotation_deg), None)?;
                    self.generated.insert(pid);
                    pid
                }
            };
            keep.insert(pid);
            let fields = [
                ("width", &t.width),
                ("height", &t.height),
                ("thickness", &t.thickness),
                ("x", &t.position[0]),
                ("y", &t.position[1]),
                ("z", &t.position[2]),
            ];
            for (n, src) in fields {
                let k = key(pid, n);
                if !self.overrides.contains(&k) && self.params.get(&k).map(|p| &p.source) != Some(src) {
                    defs.push((pid, n.to_string(), src.clone()));
                }
            }
        }
        for t in &layout.hardware {
            let hid = match existing_hw.get(&(t.kind, t.index)) {
                Some(h) => *h,
                None => {
                    let hid = self.ids.alloc();
                    let hw = Hardware {
                        id: hid,
                        name: t.name.clone(),
                        kind: t.kind,
                        role_index: t.index,
                        size_mm: t.size,
                        catalog_code: t.catalog_code.clone(),
                    };
                    self.insert_object(DomainObject::Hardware(hw), Some(id), Transform3D::IDENTITY, None)?;
                    self.generated.insert(hid);
                    hid
                }
            };
            keep.insert(hid);
            let mut fields: Vec<(&str, &String)> = vec![("x", &t.position[0]), ("y", &t.position[1]), ("z", &t.position[2])];
            if let Some(l) = &t.length_expr {
                fields.push(("length", l));
            }
            for (n, src) in fields {
                let k = key(hid, n);
                if !self.overrides.contains(&k) && self.params.get(&k).map(|p| &p.source) != Some(src) {
                    defs.push((hid, n.to_string(), src.clone()));
                }
            }
        }
        // Remove generated children that no longer exist in the layout.
        let stale: Vec<ObjectId> = self
            .scene
            .children(id)
            .iter()
            .copied()
            .filter(|c| self.generated.contains(c) && !keep.contains(c))
            .collect();
        for c in stale {
            self.remove_subtree(c)?;
        }
        self.define_params(defs)?;
        // Drop derived params that the new layout no longer defines.
        let wanted: BTreeSet<String> = base_params(&CabinetSpec::preset(cab.kind))
            .into_iter()
            .map(|(n, _)| n)
            .chain(layout.derived.iter().map(|(n, _)| n.clone()))
            .chain(["x", "y", "z"].iter().map(|s| s.to_string()))
            .collect();
        let prefix = format!("#{}.", id.0);
        let extra: Vec<ParamKey> = self
            .params
            .keys_with_prefix(&prefix)
            .filter(|k| split_key(k).is_some_and(|(_, n)| !wanted.contains(n)))
            .cloned()
            .collect();
        if !extra.is_empty() {
            let dirty = self.params.remove(&extra);
            self.apply_param_values(&dirty);
        }
        for (i, c) in layout.constraints.iter().enumerate() {
            // Ordered ids: the most fundamental constraint is reported first.
            self.add_constraint(id, &format!("constraint.{i:02}.{}", c.code), &c.expr, &c.code, &c.message)?;
        }
        self.params.check_all_constraints().map_err(|e| CoreError::from_param("cabinet", e))?;
        self.mark_changed(id);
        self.mark_tree();
        Ok(())
    }

    // ------------------------------------------------------ subtree snapshots

    pub fn snapshot(&self, root: ObjectId) -> Result<Snapshot, CoreError> {
        self.scene.node(root)?;
        let ids = self.scene.subtree(root);
        let nodes = ids.iter().map(|i| self.scene.node(*i).cloned()).collect::<Result<Vec<_>, _>>()?;
        let objects = ids.iter().filter_map(|i| self.objects.get(i).cloned()).collect();
        let mut params = Vec::new();
        let mut constraints = Vec::new();
        for i in &ids {
            let prefix = format!("#{}.", i.0);
            for k in self.params.keys_with_prefix(&prefix) {
                let p = self.params.get(k).unwrap();
                params.push((k.clone(), p.source.clone()));
            }
            for c in self.params.constraints() {
                if c.id.starts_with(&prefix) {
                    constraints.push(c.clone());
                }
            }
        }
        Ok(Snapshot {
            root,
            index: self.scene.index_in_parent(root),
            nodes,
            objects,
            params,
            constraints,
            overrides: self.overrides.iter().filter(|k| split_key(k).is_some_and(|(o, _)| ids.contains(&o))).cloned().collect(),
            generated: ids.iter().filter(|i| self.generated.contains(i)).copied().collect(),
        })
    }

    /// Remove an object and its subtree; returns what is needed to restore it.
    pub(crate) fn remove_subtree(&mut self, root: ObjectId) -> Result<Snapshot, CoreError> {
        let snap = self.snapshot(root)?;
        let ids: Vec<ObjectId> = snap.nodes.iter().map(|n| n.id).collect();
        self.scene.remove_subtree(root)?;
        for i in &ids {
            self.objects.remove(i);
            self.generated.remove(i);
            self.changes.deleted.insert(*i);
            self.changes.created.remove(i);
        }
        for i in ids.iter().rev() {
            self.remove_params_of(*i);
        }
        self.overrides.retain(|k| split_key(k).is_none_or(|(o, _)| !ids.contains(&o)));
        if let Some(p) = snap.nodes[0].parent {
            self.mark_changed(p);
        }
        self.mark_tree();
        Ok(snap)
    }

    pub(crate) fn restore(&mut self, snap: &Snapshot) -> Result<(), CoreError> {
        for n in &snap.nodes {
            if self.scene.contains(n.id) {
                return Err(CoreError::InvalidProject { reason: format!("id {} already exists", n.id) });
            }
        }
        self.scene.restore_subtree(snap.nodes.clone(), snap.index)?;
        for o in &snap.objects {
            self.ids.reserve(o.id());
            self.objects.insert(o.id(), o.clone());
            self.changes.created.insert(o.id());
            self.changes.deleted.remove(&o.id());
        }
        self.generated.extend(snap.generated.iter().copied());
        self.overrides.extend(snap.overrides.iter().cloned());
        let defs: Vec<(ObjectId, String, String)> = snap
            .params
            .iter()
            .filter_map(|(k, s)| split_key(k).map(|(o, n)| (o, n.to_string(), s.clone())))
            .collect();
        self.define_params(defs)?;
        for c in &snap.constraints {
            if let Some((owner, local)) = split_key(&c.id) {
                self.add_constraint(owner, local, &c.source, &c.code, &c.message)?;
            }
        }
        if let Some(p) = snap.nodes[0].parent {
            self.mark_changed(p);
        }
        self.mark_tree();
        Ok(())
    }

    // ------------------------------------------------------- direct setters

    /// Set a parameter by name. Structural cabinet params regenerate children.
    pub(crate) fn set_param_raw(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
        let k = key(id, name);
        if !self.params.contains(&k) {
            return Err(CoreError::InvalidParameter { name: name.into(), reason: "unknown parameter".into() });
        }
        self.define_params(vec![(id, name.to_string(), value.trim().to_string())])?;
        if self.generated.contains(&id) || matches!(self.objects.get(&id), Some(DomainObject::Cabinet(_))) {
            self.overrides.insert(k);
        }
        self.mark_changed(id);
        Ok(())
    }

    pub(crate) fn set_structure(&mut self, id: ObjectId, name: &str, value: &str) -> Result<(), CoreError> {
        let v = value.trim().trim_start_matches('=').trim();
        let cab = match self.object_mut(id)? {
            DomainObject::Cabinet(c) => c,
            _ => return Err(CoreError::InvalidParameter { name: name.into(), reason: "not a cabinet".into() }),
        };
        let count = || -> Result<u32, CoreError> {
            let n: f64 = v.parse().map_err(|_| CoreError::InvalidParameter { name: name.into(), reason: "expected a whole number".into() })?;
            if !(0.0..=50.0).contains(&n) || n.fract() != 0.0 {
                return Err(CoreError::InvalidParameter { name: name.into(), reason: "expected 0..50".into() });
            }
            Ok(n as u32)
        };
        let flag = || -> Result<bool, CoreError> {
            match v.to_ascii_lowercase().as_str() {
                "1" | "true" | "on" | "yes" => Ok(true),
                "0" | "false" | "off" | "no" => Ok(false),
                _ => Err(CoreError::InvalidParameter { name: name.into(), reason: "expected on/off".into() }),
            }
        };
        let style = || -> Result<aic_domain::JoinStyle, CoreError> {
            match v.to_ascii_uppercase().as_str() {
                "INSET" => Ok(aic_domain::JoinStyle::Inset),
                "OVERLAY" => Ok(aic_domain::JoinStyle::Overlay),
                _ => Err(CoreError::InvalidParameter { name: name.into(), reason: "expected INSET/OVERLAY".into() }),
            }
        };
        match name {
            "shelves" => cab.shelves = count()?,
            "doors" => cab.doors = count()?,
            "drawers" => cab.drawers = count()?,
            "back_panel" => cab.back_panel = flag()?,
            "top_style" => cab.top_style = style()?,
            "bottom_style" => cab.bottom_style = style()?,
            _ => return Err(CoreError::InvalidParameter { name: name.into(), reason: "unknown parameter".into() }),
        }
        self.regenerate_cabinet(id)
    }

    pub(crate) fn set_transform_raw(&mut self, id: ObjectId, t: Transform3D) -> Result<Transform3D, CoreError> {
        if !t.is_finite() {
            return Err(CoreError::InvalidTransform);
        }
        let old = self.scene.node(id)?.local_transform;
        let mut defs = Vec::new();
        for (i, n) in ["x", "y", "z"].iter().enumerate() {
            let k = key(id, n);
            if self.params.contains(&k) && self.params.value(&k) != Some(t.translation[i]) {
                defs.push((id, n.to_string(), fmt_num(t.translation[i])));
            }
        }
        // Rotation lives on the node; translation goes through params so expressions are replaced.
        let mut rot_only = old;
        rot_only.rotation_deg = t.rotation_deg;
        self.scene.set_local_transform(id, rot_only)?;
        if !defs.is_empty() {
            for (o, n, _) in &defs {
                self.overrides.insert(key(*o, n));
            }
            self.define_params(defs)?;
        }
        let mut cur = self.scene.node(id)?.local_transform;
        if cur.translation != t.translation {
            // Objects without position params.
            cur.translation = t.translation;
            self.scene.set_local_transform(id, cur)?;
        }
        self.mark_changed(id);
        Ok(old)
    }

    pub(crate) fn set_name_raw(&mut self, id: ObjectId, name: String) -> Result<String, CoreError> {
        let o = self.object_mut(id)?;
        let old = o.name().to_string();
        o.set_name(name);
        self.mark_changed(id);
        self.mark_tree();
        Ok(old)
    }

    pub(crate) fn set_material_raw(&mut self, id: ObjectId, material: MaterialId) -> Result<MaterialId, CoreError> {
        if self.material(&material).is_none() {
            return Err(CoreError::UnknownMaterial { material: material.0 });
        }
        let thickness = self.material(&material).map(|m| m.thickness_mm);
        let p = match self.object_mut(id)? {
            DomainObject::Panel(p) => p,
            _ => return Err(CoreError::InvalidParameter { name: "material".into(), reason: "not a panel".into() }),
        };
        let old = std::mem::replace(&mut p.material_id, material);
        self.mark_geometry(id);
        let _ = thickness;
        Ok(old)
    }

    pub(crate) fn with_panel<R>(&mut self, id: ObjectId, f: impl FnOnce(&mut Panel) -> Result<R, CoreError>) -> Result<R, CoreError> {
        let r = match self.object_mut(id)? {
            DomainObject::Panel(p) => f(p)?,
            _ => return Err(CoreError::InvalidFeature { reason: "not a panel".into() }),
        };
        self.mark_geometry(id);
        Ok(r)
    }

    pub(crate) fn with_cabinet<R>(&mut self, id: ObjectId, f: impl FnOnce(&mut Cabinet) -> R) -> Result<R, CoreError> {
        let r = match self.object_mut(id)? {
            DomainObject::Cabinet(c) => f(c),
            _ => return Err(CoreError::InvalidParameter { name: "cabinet".into(), reason: "not a cabinet".into() }),
        };
        self.mark_changed(id);
        Ok(r)
    }

    /// Structural value of a cabinet as text (for properties & undo).
    pub fn structure_value(&self, id: ObjectId, name: &str) -> Option<String> {
        let c = self.objects.get(&id)?.as_cabinet()?;
        Some(match name {
            "shelves" => c.shelves.to_string(),
            "doors" => c.doors.to_string(),
            "drawers" => c.drawers.to_string(),
            "back_panel" => if c.back_panel { "on".into() } else { "off".into() },
            "top_style" => format!("{:?}", c.top_style).to_uppercase(),
            "bottom_style" => format!("{:?}", c.bottom_style).to_uppercase(),
            _ => return None,
        })
    }

    pub fn format_number(v: f64) -> String {
        fmt_num(v)
    }
}
