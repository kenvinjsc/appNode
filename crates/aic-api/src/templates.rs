//! Template tủ, rule preset, array và mirror (Phase 4–5).
//! A template is the cabinet's *logical* definition (zones with LOCK/AUTO/% bays,
//! fronts, part mods, rules, materials) plus its construction parameters; inserting
//! it with another W/H/D re-solves the structure instead of scaling geometry.

use crate::protocol::{CabinetOverrides, Request};
use crate::zones::bad;
use crate::Engine;
use aic_domain::{EdgeRule, JoinStyle, ObjectId};
use aic_project::{CabinetTemplate, Command, CoreError, RulePreset};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Cabinet parameters a template / preset carries (besides W/H/D for templates).
pub const RULE_PARAMS: [&str; 9] =
    ["thickness", "back_thickness", "back_groove", "back_offset", "door_gap", "door_thickness", "shelf_setback", "plinth_height", "rail_width"];

/// Built-in presets (always available, not stored in the project).
pub fn builtin_presets() -> Vec<RulePreset> {
    let v = |pairs: &[(&str, f64)]| pairs.iter().map(|(k, x)| (k.to_string(), *x)).collect::<BTreeMap<_, _>>();
    vec![
        RulePreset {
            name: "AIC Wardrobe Standard".into(),
            values: v(&[("thickness", 17.2), ("back_thickness", 8.6), ("back_groove", 13.0), ("door_gap", 2.0), ("shelf_setback", 30.0)]),
            top_style: Some(JoinStyle::Overlay),
            bottom_style: Some(JoinStyle::Inset),
            edge_rule: Some(EdgeRule::default()),
        },
        RulePreset {
            name: "AIC Bếp dưới".into(),
            values: v(&[("thickness", 17.2), ("back_thickness", 8.6), ("back_groove", 0.0), ("door_gap", 3.0), ("shelf_setback", 20.0), ("plinth_height", 0.0)]),
            top_style: Some(JoinStyle::Rails),
            bottom_style: Some(JoinStyle::Inset),
            edge_rule: Some(EdgeRule::default()),
        },
    ]
}

impl Engine {
    fn cabinet_params(&self, id: ObjectId, names: &[&str]) -> BTreeMap<String, f64> {
        names.iter().filter_map(|n| self.doc.param_value(id, n).map(|v| (n.to_string(), v))).collect()
    }

    /// SAVE_TEMPLATE: store the cabinet's logical model under `name` (replaces same name).
    pub(crate) fn save_template(&mut self, cab: ObjectId, name: &str) -> Result<Value, CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(bad("template", "name required"));
        }
        let mut def = self.cabinet_def(cab)?;
        def.room.clear();
        def.floor.clear();
        let mut names: Vec<&str> = vec!["width", "height", "depth"];
        names.extend(RULE_PARAMS);
        let t = CabinetTemplate { name: name.into(), kind: def.kind, params: self.cabinet_params(cab, &names), cabinet: def };
        self.exec_cmd(Command::SetTemplate { name: name.into(), template: Some(Box::new(t)) })?;
        Ok(json!({ "name": name }))
    }

    /// INSERT_TEMPLATE: new cabinet from a template, re-solved for W/H/D. One undo step;
    /// refused (nothing created) when the structure does not fit the new size.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn insert_template(
        &mut self,
        name: &str,
        size: [Option<f64>; 3],
        position: Option<[f64; 3]>,
        room: Option<String>,
        floor: Option<String>,
        after: Option<ObjectId>,
    ) -> Result<Value, CoreError> {
        let t = self.doc.settings.templates.iter().find(|t| t.name == name).cloned().ok_or_else(|| bad("template", "unknown template"))?;
        let mark = self.history.mark();
        let run = |e: &mut Engine| -> Result<ObjectId, CoreError> {
            let created = e.handle(Request::CreateCabinet {
                kind: t.kind,
                position,
                parent: None,
                overrides: CabinetOverrides::default(),
                name: None,
                room: room.clone().or(Some(String::new())),
                floor: floor.clone(),
                after,
            })?;
            let id: ObjectId = serde_json::from_value(created["id"].clone()).map_err(|_| bad("template", "create failed"))?;
            let mut params = t.params.clone();
            for (k, v) in ["width", "height", "depth"].iter().zip(size) {
                if let Some(v) = v {
                    params.insert(k.to_string(), v);
                }
            }
            for (k, v) in &params {
                e.exec_cmd(Command::SetParameter { id, name: k.clone(), value: format!("{v}") })?;
            }
            let mut def = t.cabinet.clone();
            let current = e.cabinet_def(id)?;
            def.id = current.id;
            def.name = current.name;
            def.room = current.room;
            def.floor = current.floor;
            let problems = aic_domain::build_cabinet(&def, e.doc.cabinet_values(id)).problems;
            if !problems.is_empty() {
                return Err(CoreError::ConstraintViolated { constraint: "ZONE_TOO_SMALL".into(), message: format!("zones {problems:?}") });
            }
            e.exec_cmd(Command::SetCabinet { id, cabinet: Box::new(def), label: "Chèn template".into() })?;
            Ok(id)
        };
        match run(self) {
            Ok(id) => {
                self.history.squash(mark, "Chèn template");
                Ok(json!({ "id": id }))
            }
            Err(e) => {
                self.history.rollback(&mut self.doc, mark);
                Err(e)
            }
        }
    }

    pub(crate) fn delete_template(&mut self, name: &str) -> Result<(), CoreError> {
        self.exec_cmd(Command::SetTemplate { name: name.into(), template: None })
    }

    pub(crate) fn save_rule_preset(&mut self, cab: ObjectId, name: &str) -> Result<(), CoreError> {
        let name = name.trim();
        if name.is_empty() || builtin_presets().iter().any(|p| p.name == name) {
            return Err(bad("preset", "name required and not a built-in preset"));
        }
        let def = self.cabinet_def(cab)?;
        let p = RulePreset {
            name: name.into(),
            values: self.cabinet_params(cab, &RULE_PARAMS),
            top_style: Some(def.top_style),
            bottom_style: Some(def.bottom_style),
            edge_rule: Some(def.edge_rule.clone()),
        };
        self.exec_cmd(Command::SetRulePreset { name: name.into(), preset: Some(p) })
    }

    pub(crate) fn delete_rule_preset(&mut self, name: &str) -> Result<(), CoreError> {
        self.exec_cmd(Command::SetRulePreset { name: name.into(), preset: None })
    }

    /// APPLY_RULE_PRESET on cabinets: one undo step, all or nothing.
    pub(crate) fn apply_rule_preset(&mut self, ids: &[ObjectId], name: &str) -> Result<(), CoreError> {
        let p = builtin_presets()
            .into_iter()
            .chain(self.doc.settings.presets.iter().cloned())
            .find(|p| p.name == name)
            .ok_or_else(|| bad("preset", "unknown preset"))?;
        let mark = self.history.mark();
        let run = |e: &mut Engine| -> Result<(), CoreError> {
            for &id in ids {
                e.cabinet_def(id)?;
                for (k, v) in &p.values {
                    e.exec_cmd(Command::SetParameter { id, name: k.clone(), value: format!("{v}") })?;
                }
                let (ts, bs, er) = (p.top_style, p.bottom_style, p.edge_rule.clone());
                e.edit_cabinet_checked(id, "Rule preset", |c| {
                    if let Some(s) = ts {
                        c.top_style = s;
                    }
                    if let Some(s) = bs {
                        c.bottom_style = s;
                    }
                    if let Some(r) = er {
                        c.edge_rule = r;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        };
        match run(self) {
            Ok(()) => {
                self.history.squash(mark, "Áp rule preset");
                Ok(())
            }
            Err(e) => {
                self.history.rollback(&mut self.doc, mark);
                Err(e)
            }
        }
    }

    pub(crate) fn templates_info(&self) -> Value {
        let templates: Vec<Value> = self
            .doc
            .settings
            .templates
            .iter()
            .map(|t| {
                let g = |k: &str| t.params.get(k).copied().unwrap_or(0.0);
                json!({ "name": t.name, "kind": t.kind, "frame": t.kind.frame_name(), "size": [g("width"), g("height"), g("depth")],
                        "zones": t.cabinet.zones.zones().len() })
            })
            .collect();
        let presets: Vec<Value> = builtin_presets()
            .into_iter()
            .map(|p| (p, true))
            .chain(self.doc.settings.presets.iter().cloned().map(|p| (p, false)))
            .map(|(p, b)| json!({ "name": p.name, "builtin": b, "values": p.values, "top_style": p.top_style, "bottom_style": p.bottom_style }))
            .collect();
        json!({ "templates": templates, "presets": presets })
    }

    /// Array a split panel: `count` more of the same kind in its split, all bays equal.
    pub(crate) fn array_split_panel(&mut self, id: ObjectId, count: u32) -> Result<Value, CoreError> {
        let (cab, key) = self.part_ref(id).ok_or_else(|| bad("part", "not a cabinet part"))?;
        let uid: aic_domain::zone::Uid = key.strip_prefix("p:").and_then(|u| u.parse().ok()).ok_or_else(|| bad("split", "only shelves / dividers"))?;
        let mark = self.history.mark();
        let res = self.edit_cabinet_checked(cab, "Nhân tấm", |c| {
            let (s, i) = c.zones.split_of_panel_mut(uid).ok_or_else(|| bad("split", "panel not found"))?;
            let proto = s.panels[i].clone();
            let _ = s;
            let zone = c.zones.zones().into_iter().find(|z| z.split.as_ref().is_some_and(|s| s.panels.iter().any(|p| p.uid == uid))).map(|z| z.id).unwrap();
            c.zones.add_panels(zone, proto.kind, count, proto.thickness, aic_domain::zone::Lock::Even, 0.0).map_err(|e| bad("split", e))?;
            let s = c.zones.split_mut(zone).unwrap();
            s.equalize();
            for p in &mut s.panels {
                p.lock = aic_domain::zone::Lock::Even;
            }
            Ok(())
        });
        if let Err(e) = res {
            self.history.rollback(&mut self.doc, mark);
            return Err(e);
        }
        Ok(json!({}))
    }

    /// Array cabinets: `count` copies along an axis with a gap (clones the full definition).
    pub(crate) fn array_cabinet(&mut self, id: ObjectId, count: u32, axis: usize, gap: f64) -> Result<Value, CoreError> {
        if !(1..=50).contains(&count) || axis > 2 {
            return Err(bad("array", "count 1–50, axis 0–2"));
        }
        let size = [self.doc.param_value(id, "width"), self.doc.param_value(id, "height"), self.doc.param_value(id, "depth")][axis].ok_or_else(|| bad("array", "not a cabinet"))?;
        let cmds = (1..=count)
            .map(|k| {
                let mut off = [0.0; 3];
                off[axis] = (size + gap) * k as f64;
                Command::DuplicateObject { id, offset: Some(off) }
            })
            .collect();
        self.exec_cmd(Command::Batch { label: "Nhân dãy tủ".into(), commands: cmds })?;
        Ok(json!({}))
    }

    /// Lật gương trái ↔ phải (zone tree, hinges, left/right mods).
    pub(crate) fn mirror_cabinet(&mut self, id: ObjectId) -> Result<(), CoreError> {
        self.edit_cabinet(id, "Lật gương", |c| {
            c.mirror_x();
            Ok(())
        })
    }
}

