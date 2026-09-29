//! Render objects: presentation-only data for the viewport. Meshes are built in
//! definition space and shared by `geometry_key`; placement is a world matrix.

use crate::Engine;
use aic_domain::{DomainObject, ObjectId};
use aic_geometry::{build_hardware_mesh, build_panel_mesh, build_room_mesh, MeshData, PanelGeometryInput};
use aic_manufacturing::rule_features;
use aic_project::CoreError;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct RenderObject {
    pub id: ObjectId,
    pub kind: &'static str,
    pub role: Option<String>,
    pub name: String,
    pub geometry_key: String,
    /// Column-major world matrix.
    pub matrix: [f64; 16],
    pub color: String,
    pub material_id: Option<String>,
    pub visible: bool,
    pub locked: bool,
    pub parent: Option<ObjectId>,
    pub cabinet: Option<ObjectId>,
    /// Definition-space size (panels/hardware).
    pub size: Option<[f64; 3]>,
}

enum Job {
    Panel([f64; 3], Vec<aic_domain::MachiningFeature>),
    Hardware(aic_domain::HardwareKind, [f64; 3]),
    Room(aic_domain::Room),
}

fn hash_key(prefix: &str, v: &impl Serialize) -> String {
    let mut h = DefaultHasher::new();
    serde_json::to_string(v).unwrap_or_default().hash(&mut h);
    format!("{prefix}:{:016x}", h.finish())
}

impl Engine {
    pub(crate) fn render_objects(&mut self, ids: Option<Vec<ObjectId>>, known: &[String]) -> Result<Value, CoreError> {
        let ids: Vec<ObjectId> = match ids {
            Some(v) => v.into_iter().filter(|i| self.doc.objects.contains_key(i)).collect(),
            None => self.doc.objects.keys().copied().collect(),
        };
        let mut objects = Vec::new();
        let mut jobs: BTreeMap<String, Job> = BTreeMap::new();
        for id in ids {
            let obj = self.doc.object(id)?;
            let (kind, role, key, job, color, material, size) = match obj {
                DomainObject::Panel(p) => {
                    let mut feats = p.features.clone();
                    feats.extend(rule_features(p, self.doc.hinge_left(id)));
                    let key = hash_key("panel", &(p.size(), &feats));
                    let color = self.doc.material(&p.material_id).map(|m| m.color.clone()).unwrap_or_else(|| "#dddddd".into());
                    (
                        "PANEL",
                        Some(format!("{:?}", p.role)),
                        key,
                        Job::Panel(p.size(), feats),
                        color,
                        Some(p.material_id.0.clone()),
                        Some(p.size()),
                    )
                }
                DomainObject::Hardware(h) => {
                    let key = hash_key("hw", &(h.kind, h.size_mm));
                    ("HARDWARE", Some(format!("{:?}", h.kind)), key, Job::Hardware(h.kind, h.size_mm), "#8a8f96".into(), None, Some(h.size_mm))
                }
                DomainObject::Room(r) => {
                    let key = hash_key("room", &(r.width_mm, r.depth_mm, r.height_mm, r.wall_thickness_mm));
                    ("ROOM", None, key, Job::Room(r.clone()), "#e8e6e1".into(), None, None)
                }
                DomainObject::Cabinet(_) => continue,
            };
            if !known.contains(&key) && !self.mesh_cache.contains_key(&key) {
                jobs.insert(key.clone(), job);
            }
            let node = self.doc.scene.node(id).map_err(CoreError::from)?;
            objects.push(RenderObject {
                id,
                kind,
                role,
                name: obj.name().to_string(),
                geometry_key: key,
                matrix: self.world(id).to_matrix_col_major(),
                color,
                material_id: material,
                visible: self.doc.scene.is_effectively_visible(id),
                locked: self.doc.scene.is_effectively_locked(id),
                parent: node.parent,
                cabinet: self.doc.cabinet_of(id),
                size,
            });
        }
        // Tessellate missing geometry in parallel (background-friendly: pure function).
        let kernel = self.kernel;
        let built: Vec<(String, Result<MeshData, aic_geometry::GeometryError>)> = jobs
            .into_par_iter()
            .map(|(k, job)| {
                let m = match job {
                    Job::Panel(size, feats) => build_panel_mesh(&kernel, &PanelGeometryInput { size, features: &feats }),
                    Job::Hardware(kind, size) => build_hardware_mesh(&kernel, kind, size),
                    Job::Room(r) => build_room_mesh(&kernel, &r),
                };
                (k, m)
            })
            .collect();
        for (k, m) in built {
            let m = m.map_err(|e| CoreError::GeometryBooleanFailed { reason: e.to_string() })?;
            self.mesh_cache.insert(k, Arc::new(m));
        }
        // Bound the cache.
        if self.mesh_cache.len() > 5000 {
            let live: HashSet<String> = objects.iter().map(|o| o.geometry_key.clone()).collect();
            self.mesh_cache.retain(|k, _| live.contains(k));
        }
        let mut meshes = serde_json::Map::new();
        for o in &objects {
            if known.contains(&o.geometry_key) || meshes.contains_key(&o.geometry_key) {
                continue;
            }
            if let Some(m) = self.mesh_cache.get(&o.geometry_key) {
                meshes.insert(o.geometry_key.clone(), serde_json::to_value(&**m).unwrap());
            }
        }
        Ok(serde_json::json!({ "objects": objects, "meshes": meshes }))
    }
}
