//! Undoable commands. `execute` applies the command and returns its inverse, so
//! undo/redo never snapshot the whole project (only the affected subtree for
//! destructive operations).

use crate::document::{key, split_key};
use crate::{CoreError, Document, STRUCTURAL_PARAMS};
use aic_domain::cabinet::MaterialSlot;
use aic_domain::{CabinetSpec, DomainObject, EdgeBand, EdgeSide, MachiningFeature, MaterialId, ObjectId, SceneNode};
use aic_math::Transform3D;
use aic_parametric::{Constraint, ParamKey};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A detached subtree: nodes, objects and parameter sources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub root: ObjectId,
    pub index: Option<usize>,
    pub nodes: Vec<SceneNode>,
    pub objects: Vec<DomainObject>,
    pub params: Vec<(ParamKey, String)>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub overrides: Vec<ParamKey>,
    #[serde(default)]
    pub generated: Vec<ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    CreateRoom { id: Option<ObjectId>, name: String, width: f64, depth: f64, height: f64 },
    CreateCabinet { id: Option<ObjectId>, spec: CabinetSpec, position: [f64; 3], parent: Option<ObjectId>, name: Option<String> },
    CreatePanel {
        id: Option<ObjectId>,
        name: String,
        size: [f64; 3],
        material: MaterialId,
        transform: Transform3D,
        parent: Option<ObjectId>,
    },
    DeleteObject { id: ObjectId },
    Restore { snapshot: Box<Snapshot> },
    /// Replace the subtree rooted at `snapshot.root` with the snapshot.
    ReplaceSubtree { snapshot: Box<Snapshot> },
    DuplicateObject { id: ObjectId, offset: Option<[f64; 3]> },
    SetParameter { id: ObjectId, name: String, value: String },
    /// Restore raw parameter sources and override flags (inverse of SetParameter).
    RestoreParameters { entries: Vec<(ObjectId, String, String, bool)> },
    SetTransform { id: ObjectId, transform: Transform3D },
    SetName { id: ObjectId, name: String },
    SetVisible { id: ObjectId, visible: bool },
    SetLocked { id: ObjectId, locked: bool },
    Reparent { id: ObjectId, parent: Option<ObjectId>, index: Option<usize> },
    SetMaterial { id: ObjectId, material: MaterialId },
    /// Set only a cabinet's material slot (panels are changed by separate commands).
    SetCabinetSlot { id: ObjectId, slot: MaterialSlot, material: MaterialId },
    SetEdgeBand { id: ObjectId, edge: EdgeSide, band: Option<EdgeBand> },
    AddFeature { id: ObjectId, feature: MachiningFeature, index: Option<usize> },
    RemoveFeature { id: ObjectId, index: usize },
    /// Replace a cabinet definition (zones, fronts, options, part mods); inverse = old definition.
    SetCabinet { id: ObjectId, cabinet: Box<aic_domain::Cabinet>, label: String },
    Batch { label: String, commands: Vec<Command> },
}

impl Command {
    pub fn label(&self) -> &str {
        match self {
            Command::CreateRoom { .. } => "Create room",
            Command::CreateCabinet { .. } => "Create cabinet",
            Command::CreatePanel { .. } => "Create panel",
            Command::DeleteObject { .. } => "Delete",
            Command::Restore { .. } => "Restore",
            Command::ReplaceSubtree { .. } => "Change structure",
            Command::DuplicateObject { .. } => "Duplicate",
            Command::SetParameter { .. } | Command::RestoreParameters { .. } => "Set parameter",
            Command::SetTransform { .. } => "Move",
            Command::SetName { .. } => "Rename",
            Command::SetVisible { .. } => "Visibility",
            Command::SetLocked { .. } => "Lock",
            Command::Reparent { .. } => "Reparent",
            Command::SetMaterial { .. } | Command::SetCabinetSlot { .. } => "Material",
            Command::SetEdgeBand { .. } => "Edge band",
            Command::AddFeature { .. } => "Add feature",
            Command::RemoveFeature { .. } => "Remove feature",
            Command::Batch { label, .. } | Command::SetCabinet { label, .. } => label,
        }
    }

    /// Execute and return the inverse command.
    pub fn execute(self, doc: &mut Document) -> Result<Command, CoreError> {
        match self {
            Command::CreateRoom { id, name, width, depth, height } => {
                let id = doc.create_room(id, name, width, depth, height)?;
                Ok(Command::DeleteObject { id })
            }
            Command::CreateCabinet { id, spec, position, parent, name } => {
                if let Some(p) = parent {
                    doc.ensure_unlocked(p)?;
                }
                let id = doc.create_cabinet(id, &spec, position, parent, name)?;
                Ok(Command::DeleteObject { id })
            }
            Command::CreatePanel { id, name, size, material, transform, parent } => {
                if let Some(p) = parent {
                    doc.ensure_unlocked(p)?;
                }
                let id = doc.create_panel(id, name, size, material, transform, parent)?;
                Ok(Command::DeleteObject { id })
            }
            Command::DeleteObject { id } => {
                doc.ensure_unlocked(id)?;
                let snap = doc.remove_subtree(id)?;
                Ok(Command::Restore { snapshot: Box::new(snap) })
            }
            Command::Restore { snapshot } => {
                doc.restore(&snapshot)?;
                Ok(Command::DeleteObject { id: snapshot.root })
            }
            Command::ReplaceSubtree { snapshot } => {
                let current = doc.remove_subtree(snapshot.root)?;
                if let Err(e) = doc.restore(&snapshot) {
                    doc.restore(&current)?;
                    return Err(e);
                }
                Ok(Command::ReplaceSubtree { snapshot: Box::new(current) })
            }
            Command::DuplicateObject { id, offset } => {
                let snap = doc.snapshot(id)?;
                let dup = duplicate_snapshot(doc, &snap, offset)?;
                let new_root = dup.root;
                doc.restore(&dup)?;
                Ok(Command::DeleteObject { id: new_root })
            }
            Command::SetParameter { id, name, value } => {
                doc.ensure_unlocked(id)?;
                if STRUCTURAL_PARAMS.contains(&name.as_str()) {
                    let before = doc.snapshot(id)?;
                    if let Err(e) = doc.set_structure(id, &name, &value) {
                        // Roll back partially regenerated subtree.
                        let _ = Command::ReplaceSubtree { snapshot: Box::new(before) }.execute(doc);
                        return Err(e);
                    }
                    return Ok(Command::ReplaceSubtree { snapshot: Box::new(before) });
                }
                let k = key(id, &name);
                let old_source = doc
                    .params
                    .get(&k)
                    .map(|p| p.source.clone())
                    .ok_or_else(|| CoreError::InvalidParameter { name: name.clone(), reason: "unknown parameter".into() })?;
                let was_override = doc.overrides.contains(&k);
                doc.set_param_raw(id, &name, &value)?;
                Ok(Command::RestoreParameters { entries: vec![(id, name, old_source, was_override)] })
            }
            Command::RestoreParameters { entries } => {
                let mut inverse = Vec::new();
                let mut defs = Vec::new();
                for (id, name, source, ov) in entries {
                    let k = key(id, &name);
                    let cur = doc.params.get(&k).map(|p| p.source.clone()).unwrap_or_default();
                    inverse.push((id, name.clone(), cur, doc.overrides.contains(&k)));
                    if ov {
                        doc.overrides.insert(k);
                    } else {
                        doc.overrides.remove(&k);
                    }
                    defs.push((id, name, source));
                }
                let ids: Vec<ObjectId> = defs.iter().map(|d| d.0).collect();
                doc.define_params(defs)?;
                for i in ids {
                    doc.mark_changed(i);
                }
                Ok(Command::RestoreParameters { entries: inverse })
            }
            Command::SetTransform { id, transform } => {
                doc.ensure_unlocked(id)?;
                // Capture raw position sources so undo restores expressions, not just numbers.
                let entries: Vec<(ObjectId, String, String, bool)> = ["x", "y", "z"]
                    .iter()
                    .filter_map(|n| {
                        let k = key(id, n);
                        doc.params.get(&k).map(|p| (id, n.to_string(), p.source.clone(), doc.overrides.contains(&k)))
                    })
                    .collect();
                let old = doc.set_transform_raw(id, transform)?;
                let rot = Command::SetTransform {
                    id,
                    transform: Transform3D { translation: transform.translation, rotation_deg: old.rotation_deg },
                };
                if entries.is_empty() {
                    return Ok(Command::SetTransform { id, transform: old });
                }
                Ok(Command::Batch {
                    label: "Move".into(),
                    commands: vec![rot, Command::RestoreParameters { entries }],
                })
            }
            Command::SetName { id, name } => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Err(CoreError::InvalidParameter { name: "name".into(), reason: "empty".into() });
                }
                let old = doc.set_name_raw(id, name)?;
                Ok(Command::SetName { id, name: old })
            }
            Command::SetVisible { id, visible } => {
                let old = doc.scene.set_visible(id, visible)?;
                for n in doc.scene.subtree(id) {
                    doc.mark_changed(n);
                }
                doc.mark_tree();
                Ok(Command::SetVisible { id, visible: old })
            }
            Command::SetLocked { id, locked } => {
                let old = doc.scene.set_locked(id, locked)?;
                for n in doc.scene.subtree(id) {
                    doc.mark_changed(n);
                }
                doc.mark_tree();
                Ok(Command::SetLocked { id, locked: old })
            }
            Command::Reparent { id, parent, index } => {
                doc.ensure_unlocked(id)?;
                if let Some(p) = parent {
                    if !doc.object(p)?.is_container() {
                        return Err(CoreError::InvalidReparent);
                    }
                }
                let old_parent = doc.scene.parent(id);
                let old_index = doc.scene.index_in_parent(id);
                // Generated parts belong to their cabinet's layout.
                if doc.generated.contains(&id) && old_parent != parent {
                    return Err(CoreError::InvalidReparent);
                }
                let before_world = doc.scene.world(id);
                doc.scene.reparent(id, parent, index)?;
                // Keep position params consistent with the new local transform.
                let local = doc.scene.node(id)?.local_transform;
                let _ = before_world;
                let defs: Vec<(ObjectId, String, String)> = ["x", "y", "z"]
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| doc.params.contains(&key(id, n)))
                    .map(|(i, n)| (id, n.to_string(), Document::format_number(local.translation[i])))
                    .collect();
                doc.define_params(defs)?;
                doc.mark_changed(id);
                doc.mark_tree();
                Ok(Command::Reparent { id, parent: old_parent, index: old_index })
            }
            Command::SetMaterial { id, material } => {
                doc.ensure_unlocked(id)?;
                if doc.material(&material).is_none() {
                    return Err(CoreError::UnknownMaterial { material: material.0 });
                }
                match doc.object(id)? {
                    DomainObject::Panel(_) => {
                        let old = doc.set_material_raw(id, material)?;
                        Ok(Command::SetMaterial { id, material: old })
                    }
                    DomainObject::Cabinet(c) => {
                        // Apply to the cabinet's carcass slot and every panel that used it.
                        let old_carcass = c.carcass_material.clone();
                        let children: Vec<ObjectId> = doc
                            .scene
                            .subtree(id)
                            .into_iter()
                            .filter(|c| doc.panel(*c).is_some_and(|p| p.material_id == old_carcass))
                            .collect();
                        doc.with_cabinet(id, |c| c.carcass_material = material.clone())?;
                        let mut inverse = vec![];
                        for c in children {
                            let old = doc.set_material_raw(c, material.clone())?;
                            inverse.push(Command::SetMaterial { id: c, material: old });
                        }
                        inverse.push(Command::SetCabinetSlot { id, slot: MaterialSlot::Carcass, material: old_carcass });
                        Ok(Command::Batch { label: "Material".into(), commands: inverse })
                    }
                    _ => Err(CoreError::InvalidParameter { name: "material".into(), reason: "object has no material".into() }),
                }
            }
            Command::SetCabinetSlot { id, slot, material } => {
                if doc.material(&material).is_none() {
                    return Err(CoreError::UnknownMaterial { material: material.0 });
                }
                let old = doc.with_cabinet(id, |c| {
                    let field = match slot {
                        MaterialSlot::Carcass => &mut c.carcass_material,
                        MaterialSlot::Front => &mut c.front_material,
                        MaterialSlot::Back => &mut c.back_material,
                    };
                    std::mem::replace(field, material)
                })?;
                Ok(Command::SetCabinetSlot { id, slot, material: old })
            }
            Command::SetEdgeBand { id, edge, band } => {
                doc.ensure_unlocked(id)?;
                let old = doc.with_panel(id, |p| {
                    let old = p.edge_band(edge).cloned();
                    p.set_edge_band(edge, band);
                    Ok(old)
                })?;
                Ok(Command::SetEdgeBand { id, edge, band: old })
            }
            Command::AddFeature { id, feature, index } => {
                doc.ensure_unlocked(id)?;
                let at = doc.with_panel(id, |p| {
                    feature.validate(p.width_mm, p.height_mm, p.thickness_mm).map_err(|reason| CoreError::InvalidFeature { reason })?;
                    let at = index.unwrap_or(p.features.len()).min(p.features.len());
                    p.features.insert(at, feature);
                    Ok(at)
                })?;
                Ok(Command::RemoveFeature { id, index: at })
            }
            Command::RemoveFeature { id, index } => {
                doc.ensure_unlocked(id)?;
                let f = doc.with_panel(id, |p| {
                    if index >= p.features.len() {
                        return Err(CoreError::InvalidFeature { reason: "no such feature".into() });
                    }
                    Ok(p.features.remove(index))
                })?;
                Ok(Command::AddFeature { id, feature: f, index: Some(index) })
            }
            Command::SetCabinet { id, cabinet, label } => {
                doc.ensure_unlocked(id)?;
                let old = doc.set_cabinet(id, *cabinet)?;
                Ok(Command::SetCabinet { id, cabinet: Box::new(old), label })
            }
            Command::Batch { label, commands } => {
                let mut inverses = Vec::new();
                for c in commands {
                    match c.execute(doc) {
                        Ok(inv) => inverses.push(inv),
                        Err(e) => {
                            for inv in inverses.into_iter().rev() {
                                let _ = inv.execute(doc);
                            }
                            return Err(e);
                        }
                    }
                }
                inverses.reverse();
                Ok(Command::Batch { label, commands: inverses })
            }
        }
    }
}

/// Copy a snapshot with fresh ids and an offset root position.
fn duplicate_snapshot(doc: &mut Document, snap: &Snapshot, offset: Option<[f64; 3]>) -> Result<Snapshot, CoreError> {
    let mut map: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    for n in &snap.nodes {
        map.insert(n.id, doc.ids.alloc());
    }
    let remap = |id: &ObjectId| *map.get(id).unwrap_or(id);
    let nodes: Vec<SceneNode> = snap
        .nodes
        .iter()
        .map(|n| SceneNode {
            id: remap(&n.id),
            object_id: remap(&n.object_id),
            parent: n.parent.map(|p| remap(&p)),
            children: n.children.iter().map(remap).collect(),
            ..n.clone()
        })
        .collect();
    let objects: Vec<DomainObject> = snap
        .objects
        .iter()
        .map(|o| {
            let mut o = o.clone();
            let new_id = remap(&o.id());
            o.set_id(new_id);
            let name = format!("{} copy", o.name());
            if new_id == remap(&snap.root) {
                o.set_name(name);
            }
            o
        })
        .collect();
    let rekey = |k: &str| -> String {
        match split_key(k) {
            Some((o, n)) => key(remap(&o), n),
            None => k.to_string(),
        }
    };
    let root = remap(&snap.root);
    // Offset: along the thinnest world axis of the object (e.g. shelves go up).
    let off = offset.unwrap_or_else(|| {
        let b = doc.world_aabb(snap.root);
        if b.is_empty() {
            return [100.0, 0.0, 0.0];
        }
        let s = b.size();
        if doc.scene.parent(snap.root).is_none() {
            [s[0] + 50.0, 0.0, 0.0]
        } else {
            let axis = (0..3).min_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap();
            let mut o = [0.0; 3];
            o[axis] = s[axis] + 100.0;
            o
        }
    });
    let local_root = doc.scene.node(snap.root)?.local_transform;
    let parent_rot = doc.scene.parent(snap.root).map(|p| doc.scene.world(p)).unwrap_or_default();
    let local_off = parent_rot.inverse().transform_vector(off);
    let mut params: Vec<(ParamKey, String)> = Vec::new();
    let mut overrides: Vec<ParamKey> = snap.overrides.iter().map(|k| rekey(k)).collect();
    for (k, s) in &snap.params {
        let nk = rekey(k);
        let (o, n) = split_key(&nk).unwrap();
        if o == root && ["x", "y", "z"].contains(&n) {
            let i = ["x", "y", "z"].iter().position(|a| *a == n).unwrap();
            params.push((nk.clone(), Document::format_number(local_root.translation[i] + local_off[i])));
            overrides.push(nk);
        } else {
            params.push((nk, s.clone()));
        }
    }
    let constraints = snap
        .constraints
        .iter()
        .map(|c| Constraint { id: rekey(&c.id), ..c.clone() })
        .collect();
    let root_is_generated = snap.generated.contains(&snap.root);
    Ok(Snapshot {
        root,
        index: snap.index.map(|i| i + 1),
        nodes,
        objects,
        params,
        constraints,
        overrides,
        // A duplicated generated part becomes a free part; its children stay generated.
        generated: snap.generated.iter().filter(|g| !(root_is_generated && **g == snap.root)).map(remap).collect(),
    })
}
