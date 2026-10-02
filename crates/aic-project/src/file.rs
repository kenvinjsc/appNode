//! Versioned project file (`{"format": "aic-project", "version": 1, ...}`).
//! Contains no UI state. Older versions are upgraded by [`migrate`].

use crate::document::split_key;
use crate::{CoreError, Document, ProjectMeta, ProjectSettings};
use aic_domain::cabinet::{generate, CabinetStructure};
use aic_domain::{DomainObject, IdAllocator, Material, ObjectId, Scene};
use aic_parametric::{ParamGraph, ParamKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FORMAT: &str = "aic-project";
pub const VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamRecord {
    pub owner: ObjectId,
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintRecord {
    pub owner: ObjectId,
    pub id: String,
    pub source: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub format: String,
    pub version: u64,
    pub metadata: ProjectMeta,
    pub settings: ProjectSettings,
    pub scene: Scene,
    pub objects: Vec<DomainObject>,
    pub parameters: Vec<ParamRecord>,
    pub constraints: Vec<ConstraintRecord>,
    pub overrides: Vec<ParamKey>,
    pub generated: Vec<ObjectId>,
    pub materials: Vec<Material>,
    pub ids: IdAllocator,
    /// Derived assembly relations cached at save time (recomputed on load).
    #[serde(default)]
    pub relations: Value,
}

impl ProjectFile {
    pub fn from_document(doc: &Document) -> Self {
        let parameters = doc
            .params
            .iter()
            .filter_map(|p| split_key(&p.key).map(|(o, n)| ParamRecord { owner: o, name: n.to_string(), source: p.source.clone() }))
            .collect();
        let constraints = doc
            .params
            .constraints()
            .filter_map(|c| {
                split_key(&c.id).map(|(o, local)| ConstraintRecord {
                    owner: o,
                    id: local.to_string(),
                    source: c.source.clone(),
                    code: c.code.clone(),
                    message: c.message.clone(),
                })
            })
            .collect();
        ProjectFile {
            format: FORMAT.into(),
            version: VERSION,
            metadata: doc.meta.clone(),
            settings: doc.settings.clone(),
            scene: doc.scene.clone(),
            objects: doc.objects.values().cloned().collect(),
            parameters,
            constraints,
            overrides: doc.overrides.iter().cloned().collect(),
            generated: doc.generated.iter().copied().collect(),
            materials: doc.materials.clone(),
            ids: doc.ids.clone(),
            relations: Value::Null,
        }
    }

    pub fn into_document(self) -> Result<Document, CoreError> {
        let mut doc = Document::new(self.metadata.name.clone());
        doc.meta = self.metadata;
        doc.settings = self.settings;
        doc.scene = self.scene;
        doc.scene.mark_all_dirty();
        doc.materials = self.materials;
        // Dự án cũ: bổ sung vật liệu mặc định mới (theo mã) để dùng được ngay.
        for m in aic_domain::default_materials() {
            if !doc.materials.iter().any(|x| x.id == m.id) {
                doc.materials.push(m);
            }
        }
        doc.ids = self.ids;
        doc.params = ParamGraph::new();
        for o in self.objects {
            doc.ids.reserve(o.id());
            doc.objects.insert(o.id(), o);
        }
        for n in doc.scene.nodes() {
            if !doc.objects.contains_key(&n.object_id) {
                return Err(CoreError::InvalidProject { reason: format!("node {} has no object", n.id) });
            }
        }
        doc.overrides = self.overrides.into_iter().collect();
        doc.generated = self.generated.into_iter().collect();
        let defs = self.parameters.into_iter().map(|p| (p.owner, p.name, p.source)).collect();
        doc.define_params(defs)?;
        let mut constraints = self.constraints;
        if constraints.is_empty() {
            // Older files: rebuild derived constraints from cabinet structure.
            for o in doc.objects.values() {
                if let DomainObject::Cabinet(c) = o {
                    let st = CabinetStructure {
                        kind: c.kind,
                        shelves: c.shelves,
                        doors: c.doors,
                        drawers: c.drawers,
                        back_panel: c.back_panel,
                        top_style: c.top_style,
                        bottom_style: c.bottom_style,
                    };
                    for (i, t) in generate(&st).constraints.into_iter().enumerate() {
                        constraints.push(ConstraintRecord {
                            owner: c.id,
                            id: format!("constraint.{i:02}.{}", t.code),
                            source: t.expr,
                            code: t.code,
                            message: t.message,
                        });
                    }
                }
            }
        }
        for c in constraints {
            doc.add_constraint(c.owner, &c.id, &c.source, &c.code, &c.message)?;
        }
        doc.take_changes();
        doc.mark_loaded();
        Ok(doc)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("project serialises")
    }

    pub fn from_json(text: &str) -> Result<Self, CoreError> {
        let v: Value = serde_json::from_str(text).map_err(|e| CoreError::InvalidProject { reason: e.to_string() })?;
        let v = migrate(v)?;
        serde_json::from_value(v).map_err(|e| CoreError::InvalidProject { reason: e.to_string() })
    }
}

/// Upgrade any supported older project JSON to the current version.
///
/// Version 0 (pre-release) stored `name` at the top level and parameters as a
/// `{"#id.name": "source"}` map, without constraints/overrides/generated sets.
pub fn migrate(mut v: Value) -> Result<Value, CoreError> {
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    if let Some(f) = v.get("format").and_then(Value::as_str) {
        if f != FORMAT {
            return Err(CoreError::InvalidProject { reason: format!("unknown format '{f}'") });
        }
    }
    if version > VERSION {
        return Err(CoreError::UnsupportedVersion { version });
    }
    if version == 0 {
        let obj = v.as_object_mut().ok_or(CoreError::InvalidProject { reason: "not an object".into() })?;
        let name = obj.remove("name").unwrap_or(Value::String("Untitled".into()));
        obj.insert("metadata".into(), serde_json::json!({ "name": name, "description": "" }));
        let params = obj.remove("params").unwrap_or(Value::Object(Default::default()));
        let mut records = Vec::new();
        if let Some(map) = params.as_object() {
            for (k, s) in map {
                let (owner, name) = split_key(k).ok_or(CoreError::InvalidProject { reason: format!("bad key {k}") })?;
                records.push(serde_json::json!({ "owner": owner.0, "name": name, "source": s }));
            }
        }
        obj.insert("parameters".into(), Value::Array(records));
        obj.entry("constraints").or_insert(Value::Array(vec![]));
        obj.entry("overrides").or_insert(Value::Array(vec![]));
        obj.entry("generated").or_insert(Value::Array(vec![]));
        obj.entry("settings").or_insert(serde_json::to_value(ProjectSettings::default()).unwrap());
        if !obj.contains_key("materials") {
            obj.insert("materials".into(), serde_json::to_value(aic_domain::default_materials()).unwrap());
        }
        if !obj.contains_key("ids") {
            let max = obj
                .get("objects")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|o| o.get("id").and_then(Value::as_u64)).max().unwrap_or(0))
                .unwrap_or(0);
            obj.insert("ids".into(), serde_json::json!({ "next": max + 1 }));
        }
        obj.insert("format".into(), Value::String(FORMAT.into()));
        obj.insert("version".into(), Value::from(1u64));
    }
    Ok(v)
}

