//! Dãy tủ: tạo / sửa / xóa và sinh lại các tấm dãy (mặt đá, len chân liền, tấm lấp,
//! che trần) mỗi khi tủ trong dãy đổi kích thước hoặc vị trí. Mọi thay đổi là các
//! Command trong một bước undo.

use crate::zones::bad;
use crate::Engine;
use aic_domain::run::{build, RunBox, RunRules};
use aic_domain::{MaterialId, ObjectId};
use aic_math::Transform3D;
use aic_project::RunDef;
use aic_project::{Command, CoreError};
use serde_json::{json, Value};

impl Engine {
    fn run_boxes(&self, cabs: &[ObjectId]) -> Result<(Transform3D, Vec<RunBox>), CoreError> {
        let first = *cabs.first().ok_or_else(|| bad("run", "select cabinets"))?;
        let r = self.doc.scene.world(first);
        let inv = r.inverse();
        let mut boxes = Vec::new();
        for &c in cabs {
            let p = |n: &str| self.doc.param_value(c, n).unwrap_or(0.0);
            let (w, h, d) = (p("width"), p("height"), p("depth"));
            let l = inv.compose(&self.doc.scene.world(c));
            let mut min = [f64::MAX; 3];
            let mut max = [f64::MIN; 3];
            for i in 0..8 {
                let q = l.transform_point([if i & 1 != 0 { w } else { 0.0 }, if i & 2 != 0 { h } else { 0.0 }, if i & 4 != 0 { d } else { 0.0 }]);
                for k in 0..3 {
                    min[k] = min[k].min(q[k]);
                    max[k] = max[k].max(q[k]);
                }
            }
            boxes.push(RunBox { min, max, plinth: p("plinth_height") });
        }
        Ok((r, boxes))
    }

    /// Xóa tấm dãy cũ và sinh lại theo luật + vị trí tủ hiện tại (trong bước undo đang mở).
    fn regenerate_run(&mut self, mut run: RunDef) -> Result<(), CoreError> {
        run.cabinets.retain(|c| self.doc.objects.contains_key(c));
        for id in std::mem::take(&mut run.parts) {
            if self.doc.objects.contains_key(&id) {
                self.exec_cmd(Command::DeleteObject { id })?;
            }
        }
        run.sig.clear();
        if !run.cabinets.is_empty() {
            let (r, boxes) = self.run_boxes(&run.cabinets)?;
            run.sig = boxes.iter().map(|b| [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]]).collect();
            let t = self.doc.param_value(run.cabinets[0], "thickness").unwrap_or(17.2);
            let parent = self.doc.scene.parent(run.cabinets[0]);
            let carcass = self.doc.settings.default_carcass_material.0.clone();
            for part in build(&run.rules, &boxes, t) {
                let id = self.doc.ids.clone().alloc();
                self.exec_cmd(Command::CreatePanel {
                    id: Some(id),
                    name: part.name.into(),
                    size: part.size,
                    material: MaterialId::new(part.material.unwrap_or_else(|| carcass.clone())),
                    transform: self.local_of(parent, r.compose(&Transform3D::new(part.translation, part.rotation_deg))),
                    parent,
                })?;
                for feature in part.features {
                    self.exec_cmd(Command::AddFeature { id, feature, index: None })?;
                }
                run.parts.push(id);
            }
        }
        let name = run.name.clone();
        self.exec_cmd(Command::SetRun { name, run: Some(run) })
    }

    fn local_of(&self, parent: Option<ObjectId>, world: Transform3D) -> Transform3D {
        match parent {
            Some(p) => self.doc.scene.world(p).inverse().compose(&world),
            None => world,
        }
    }

    fn set_run_plinths(&mut self, cabs: &[ObjectId], continuous: bool) -> Result<(), CoreError> {
        // Len chân liền: tủ trong dãy bỏ len chân riêng (giữ cao chân).
        for &c in cabs {
            let v = if continuous { "NONE" } else { "AUTO" };
            self.set_parameter_pub(c, "base_type", v)?;
        }
        Ok(())
    }

    pub(crate) fn create_run(&mut self, ids: Vec<ObjectId>, rules: Option<RunRules>) -> Result<Value, CoreError> {
        let cabs: Vec<ObjectId> = ids.into_iter().filter(|i| self.doc.objects.get(i).is_some_and(|o| o.as_cabinet().is_some())).collect();
        if cabs.is_empty() {
            return Err(bad("run", "select cabinets"));
        }
        let mut n = self.doc.settings.runs.len() + 1;
        while self.doc.settings.runs.iter().any(|r| r.name == format!("Dãy {n}")) {
            n += 1;
        }
        let run = RunDef { name: format!("Dãy {n}"), cabinets: cabs.clone(), rules: rules.unwrap_or_default(), parts: Vec::new(), sig: Vec::new() };
        let mark = self.history.mark();
        let res = self.set_run_plinths(&cabs, run.rules.continuous_plinth).and_then(|_| self.regenerate_run(run.clone()));
        if let Err(e) = res {
            self.history.rollback(&mut self.doc, mark);
            return Err(e);
        }
        self.history.squash(mark, "Tạo dãy tủ");
        Ok(json!({ "name": run.name }))
    }

    pub(crate) fn update_run(&mut self, name: &str, rules: RunRules) -> Result<(), CoreError> {
        let mut run = self.doc.settings.runs.iter().find(|r| r.name == name).cloned().ok_or_else(|| bad("run", "unknown run"))?;
        let plinth_changed = run.rules.continuous_plinth != rules.continuous_plinth;
        run.rules = rules;
        let mark = self.history.mark();
        let cabs = run.cabinets.clone();
        let res = (if plinth_changed { self.set_run_plinths(&cabs, run.rules.continuous_plinth) } else { Ok(()) }).and_then(|_| self.regenerate_run(run));
        if let Err(e) = res {
            self.history.rollback(&mut self.doc, mark);
            return Err(e);
        }
        self.history.squash(mark, "Sửa dãy tủ");
        Ok(())
    }

    pub(crate) fn delete_run(&mut self, name: &str) -> Result<(), CoreError> {
        let run = self.doc.settings.runs.iter().find(|r| r.name == name).cloned().ok_or_else(|| bad("run", "unknown run"))?;
        let mark = self.history.mark();
        let res = (|| {
            for id in &run.parts {
                if self.doc.objects.contains_key(id) {
                    self.exec_cmd(Command::DeleteObject { id: *id })?;
                }
            }
            if run.rules.continuous_plinth {
                let cabs: Vec<ObjectId> = run.cabinets.iter().copied().filter(|c| self.doc.objects.contains_key(c)).collect();
                self.set_run_plinths(&cabs, false)?;
            }
            self.exec_cmd(Command::SetRun { name: name.into(), run: None })
        })();
        if let Err(e) = res {
            self.history.rollback(&mut self.doc, mark);
            return Err(e);
        }
        self.history.squash(mark, "Xóa dãy tủ");
        Ok(())
    }

    pub(crate) fn get_runs(&self) -> Value {
        json!({ "runs": self.doc.settings.runs })
    }

    /// Sinh lại các dãy có tủ đổi kích thước / vị trí / cao chân (so với lần sinh trước).
    pub(crate) fn sync_runs(&mut self) -> Result<(), CoreError> {
        let runs: Vec<RunDef> = self.doc.settings.runs.clone();
        for r in runs {
            let alive: Vec<ObjectId> = r.cabinets.iter().copied().filter(|c| self.doc.objects.contains_key(c)).collect();
            let sig: Vec<[f64; 6]> = if alive.is_empty() {
                Vec::new()
            } else {
                self.run_boxes(&alive)?.1.iter().map(|b| [b.min[0], b.min[1], b.min[2], b.max[0], b.max[1], b.max[2]]).collect()
            };
            let same = sig.len() == r.sig.len() && sig.iter().zip(r.sig.iter()).all(|(a, b): (&[f64; 6], &[f64; 6])| a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 0.01));
            if !same || alive.len() != r.cabinets.len() {
                self.regenerate_run(r)?;
            }
        }
        Ok(())
    }

    pub(crate) fn has_runs(&self) -> bool {
        !self.doc.settings.runs.is_empty()
    }
}
