//! Derived manufacturing data: assembly relations, joint features, flattened
//! panels, part lists, nesting jobs and CNC programs. All cached per revision.

use crate::Engine;
use aic_assembly::{compute_relations, AssemblyGraph};
use aic_domain::{DomainObject, GrainDirection, ObjectId};
use aic_manufacturing::{default_tools, derive_joint_features, flatten, generate_program, rule_features, DerivedFeature, FeatureOrigin, FlatPanel, JointSettings, PanelPlacement, SheetPart};
use aic_nesting::{GrainConstraint, MaxRectsNester, Nester, NestingJob, NestingPart, NestingResult, NestingSettings, SheetSpec};
use aic_project::CoreError;
use aic_spatial::SpatialItem;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct PartRow {
    pub id: ObjectId,
    pub name: String,
    pub cabinet: Option<String>,
    pub role: String,
    pub material_id: String,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    pub grain: GrainDirection,
    pub edge_bands: Vec<String>,
    pub drills: usize,
    pub grooves: usize,
    pub pockets: usize,
}

impl Engine {
    fn panel_ids(&self) -> Vec<ObjectId> {
        self.doc
            .objects
            .iter()
            .filter(|(id, o)| o.as_panel().is_some() && self.doc.scene.is_effectively_visible(**id))
            .map(|(id, _)| *id)
            .collect()
    }

    pub(crate) fn relations(&mut self) -> Arc<AssemblyGraph> {
        let rev = self.doc.revision;
        if let Some((r, g)) = &self.relations {
            if *r == rev {
                return g.clone();
            }
        }
        let items: Vec<SpatialItem> = self
            .panel_ids()
            .into_iter()
            .filter_map(|id| self.doc.world_obb(id).map(|o| SpatialItem::new(id, o)))
            .collect();
        let mut settings = self.relation_settings;
        settings.max_gap_mm = self.doc.settings.max_relation_gap_mm;
        let g = Arc::new(compute_relations(items, &settings));
        self.relations = Some((rev, g.clone()));
        g
    }

    fn joint_features(&mut self) -> Arc<HashMap<ObjectId, Vec<DerivedFeature>>> {
        let rev = self.doc.revision;
        if let Some((r, j)) = &self.joints {
            if *r == rev {
                return j.clone();
            }
        }
        let rel = self.relations();
        let placements: HashMap<ObjectId, PanelPlacement> = self
            .doc
            .objects
            .iter()
            .filter_map(|(id, o)| o.as_panel().map(|p| (*id, PanelPlacement { panel: p, world: self.doc.scene.world(*id) })))
            .collect();
        let j = Arc::new(derive_joint_features(&placements, rel.relations(), &JointSettings::default()));
        self.joints = Some((rev, j.clone()));
        j
    }

    pub(crate) fn flat_panel(&mut self, id: ObjectId) -> Result<FlatPanel, CoreError> {
        let joints = self.joint_features();
        let p = self.doc.panel(id).ok_or(CoreError::NotFound { id })?;
        let mut derived: Vec<DerivedFeature> = rule_features(p, self.doc.hinge_left(id))
            .into_iter()
            .map(|f| DerivedFeature { feature: f, origin: FeatureOrigin::Rule })
            .collect();
        derived.extend(joints.get(&id).cloned().unwrap_or_default());
        Ok(flatten(p, derived))
    }

    pub(crate) fn parts(&mut self) -> Value {
        let ids = self.panel_ids();
        let mut rows = Vec::new();
        for id in ids {
            let Ok(flat) = self.flat_panel(id) else { continue };
            let cabinet = self.doc.cabinet_of(id).and_then(|c| self.doc.objects.get(&c)).map(|c| c.name().to_string());
            let s = &flat.summary;
            rows.push(PartRow {
                id,
                name: flat.name.clone(),
                cabinet,
                role: format!("{:?}", flat.role),
                material_id: flat.material_id.0.clone(),
                length: flat.height.max(flat.width),
                width: flat.height.min(flat.width),
                thickness: flat.thickness,
                grain: flat.grain,
                edge_bands: s.edge_bands.clone(),
                drills: s.drills + s.edge_drills,
                grooves: s.grooves,
                pockets: s.pockets,
            });
        }
        let mut by_material: BTreeMap<String, (usize, f64)> = BTreeMap::new();
        for r in &rows {
            let e = by_material.entry(r.material_id.clone()).or_default();
            e.0 += 1;
            e.1 += r.length * r.width / 1e6;
        }
        let materials: Vec<Value> = by_material
            .into_iter()
            .map(|(m, (n, area))| {
                let name = self.doc.materials.iter().find(|x| x.id.0 == m).map(|x| x.name.clone()).unwrap_or_else(|| m.clone());
                json!({ "material_id": m, "name": name, "count": n, "area_m2": (area * 1000.0).round() / 1000.0 })
            })
            .collect();
        json!({ "parts": rows, "materials": materials })
    }

    fn nesting_job(&mut self, material: &str, settings: NestingSettings) -> Result<(NestingJob, Vec<FlatPanel>), CoreError> {
        let mat = self
            .doc
            .materials
            .iter()
            .find(|m| m.id.0 == material)
            .cloned()
            .ok_or(CoreError::UnknownMaterial { material: material.into() })?;
        let ids: Vec<ObjectId> = self
            .panel_ids()
            .into_iter()
            .filter(|id| self.doc.panel(*id).is_some_and(|p| p.material_id.0 == material))
            .collect();
        let mut flats = Vec::new();
        let mut parts = Vec::new();
        for id in ids {
            let flat = self.flat_panel(id)?;
            parts.push(NestingPart {
                id,
                name: flat.name.clone(),
                contour: flat.outer.clone(),
                holes: flat.inner.clone(),
                quantity: 1,
                grain: match (mat.has_grain, flat.grain) {
                    (true, GrainDirection::AlongHeight) => GrainConstraint::AlongSheetLength,
                    (true, GrainDirection::AlongWidth) => GrainConstraint::Fixed,
                    _ => GrainConstraint::Free,
                },
                material_id: mat.id.clone(),
            });
            flats.push(flat);
        }
        let job = NestingJob {
            parts,
            sheet: SheetSpec {
                material_id: mat.id.clone(),
                width_mm: mat.sheet_width_mm,
                height_mm: mat.sheet_height_mm,
                thickness_mm: mat.thickness_mm,
                has_grain: mat.has_grain,
            },
            settings,
        };
        Ok((job, flats))
    }

    pub(crate) fn run_nesting(&mut self, material: Option<String>, settings: NestingSettings) -> Result<Value, CoreError> {
        let materials: Vec<String> = match material {
            Some(m) => vec![m],
            None => {
                let mut v: Vec<String> = self
                    .doc
                    .objects
                    .values()
                    .filter_map(|o| match o {
                        DomainObject::Panel(p) => Some(p.material_id.0.clone()),
                        _ => None,
                    })
                    .collect();
                v.sort();
                v.dedup();
                v
            }
        };
        let mut results = Vec::new();
        for m in materials {
            let (job, _) = self.nesting_job(&m, settings)?;
            let result = MaxRectsNester.nest(&job);
            let names: HashMap<ObjectId, String> = job.parts.iter().map(|p| (p.id, p.name.clone())).collect();
            results.push(json!({
                "material_id": m,
                "sheet": job.sheet,
                "result": result,
                "names": names,
            }));
            self.nesting.insert(m.clone(), (self.doc.revision, result));
        }
        Ok(json!({ "jobs": results }))
    }

    pub(crate) fn generate_cnc(&mut self, material: &str, sheet_id: u32) -> Result<Value, CoreError> {
        let fresh = matches!(self.nesting.get(material), Some((rev, _)) if *rev == self.doc.revision);
        if !fresh {
            self.run_nesting(Some(material.to_string()), NestingSettings::default())?;
        }
        let (job, flats) = self.nesting_job(material, NestingSettings::default())?;
        let result: NestingResult = self.nesting.get(material).map(|r| r.1.clone()).ok_or(CoreError::NotFound { id: ObjectId(0) })?;
        let sheet = result
            .sheets
            .iter()
            .find(|s| s.id == sheet_id)
            .ok_or(CoreError::InvalidParameter { name: "sheet_id".into(), reason: "no such sheet".into() })?;
        let by_id: HashMap<ObjectId, &FlatPanel> = flats.iter().map(|f| (f.id, f)).collect();
        let parts: Vec<SheetPart> = result
            .placements
            .iter()
            .filter(|p| p.sheet_id == sheet_id)
            .filter_map(|p| by_id.get(&p.part_id).map(|f| SheetPart { flat: f, placement: p }))
            .collect();
        let prog = generate_program(sheet_id, sheet.width_mm, sheet.height_mm, job.sheet.thickness_mm, &parts, &default_tools());
        let placements: Vec<_> = result.placements.iter().filter(|p| p.sheet_id == sheet_id).collect();
        Ok(json!({ "program": prog, "placements": placements, "sheet_count": result.sheets.len() }))
    }
}
