//! Edge-banding rule evaluation (luật dán cạnh) and the costing report
//! (Costing Report · Danh sách cắt · Cabinet List). All quantities are
//! computed here from the model; the UI only displays and edits unit prices.

use crate::Engine;
use aic_assembly::ContactType;
use aic_domain::{DomainObject, DrillPurpose, EdgeBand, EdgeMode, EdgeSide, MachiningFeature, ObjectId, PanelRole};
use aic_manufacturing::FeatureOrigin;
use aic_project::CoreError;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn face_of(edge: EdgeSide) -> (usize, i8) {
    match edge {
        EdgeSide::Left => (0, -1),
        EdgeSide::Right => (0, 1),
        EdgeSide::Bottom => (1, -1),
        EdgeSide::Top => (1, 1),
    }
}

#[derive(Debug, Serialize)]
pub struct CutRow {
    pub id: ObjectId,
    pub cabinet: String,
    pub room: String,
    pub name: String,
    pub full_name: String,
    pub role: String,
    pub material_id: String,
    pub material: String,
    pub length: f64,
    pub width: f64,
    pub thickness: f64,
    /// Cut size = finished size minus edge band on banded edges.
    pub cut_length: f64,
    pub cut_width: f64,
    pub qty: u32,
    /// Banded edges (TOP/BOTTOM/LEFT/RIGHT) with band code.
    pub edges: Vec<(EdgeSide, String)>,
    pub edge_m: f64,
    /// Edge metres per band code.
    #[serde(skip)]
    pub edge_codes: BTreeMap<String, f64>,
    pub machining: Vec<String>,
    pub note: String,
    /// Mã tấm (in nhãn, lắp đặt): `<phòng>-<tủ>-<số>`.
    pub code: String,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct Line {
    pub key: String,
    pub name: String,
    pub qty: f64,
    pub unit: String,
    pub price: f64,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub factor: Option<f64>,
}

impl Engine {
    /// Effective edge bands of a panel: cabinet rule + manual overrides for
    /// generated parts; stored bands for free panels.
    pub(crate) fn effective_edges(&mut self, id: ObjectId) -> Vec<EdgeBand> {
        let Some(p) = self.doc.panel(id).cloned() else { return Vec::new() };
        let Some((cab, key)) = self.part_ref(id) else { return p.edge_bands };
        let Ok(def) = self.cabinet_def(cab) else { return p.edge_bands };
        let rule = &def.edge_rule;
        let overrides = def.mods.get(&key).map(|m| m.edges.clone()).unwrap_or_default();
        // Luật theo nhóm tấm; hậu / đáy hộc mặc định không dán (theo vai trò, không theo độ dày).
        let group = aic_domain::edge_group(p.role);
        let g = rule.groups.get(group);
        let (mode, code, band_t) = match g {
            Some(g) => (g.mode, g.band_code.clone(), g.band_thickness),
            None => (rule.mode, rule.band_code.clone(), rule.band_thickness),
        };
        let role_skip = g.is_none() && matches!(p.role, PanelRole::Back | PanelRole::BackSub | PanelRole::DrawerBottom);
        let skip = role_skip || (g.is_none() && rule.skip_thicknesses.iter().any(|s| (s - p.thickness_mm).abs() < 0.05));
        let rels = self.relations().relations_of(id);
        let mut out = Vec::new();
        for edge in EdgeSide::ALL {
            let len = match edge {
                EdgeSide::Left | EdgeSide::Right => p.height_mm,
                _ => p.width_mm,
            };
            let hidden = || {
                rels.iter().any(|r| {
                    let target_front = matches!(self.doc.panel(r.target).map(|t| t.role), Some(PanelRole::Door | PanelRole::DrawerFront));
                    r.source_face == face_of(edge)
                        && !target_front
                        && r.contact_area_mm2 > 0.0
                        && (r.contact != ContactType::Gap || r.gap_mm <= rule.threshold)
                })
            };
            let auto = match mode {
                EdgeMode::None => false,
                EdgeMode::All => !skip && len > rule.min_length,
                EdgeMode::ExposedOnly => !skip && len > rule.min_length && !hidden(),
            };
            if overrides.get(&edge).copied().unwrap_or(auto) {
                out.push(EdgeBand { edge, material_code: code.clone(), thickness_mm: band_t });
            }
        }
        out
    }

    fn price(&self, key: &str) -> f64 {
        self.doc.settings.prices.get(key).copied().unwrap_or(0.0)
    }

    fn material_name(&self, id: &str) -> String {
        self.doc.materials.iter().find(|m| m.id.0 == id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string())
    }

    fn cut_rows(&mut self, only: Option<ObjectId>) -> Result<Vec<CutRow>, CoreError> {
        let ids: Vec<ObjectId> = self
            .doc
            .objects
            .iter()
            .filter(|(id, o)| o.as_panel().is_some() && self.doc.scene.is_effectively_visible(**id))
            .filter(|(id, _)| only.is_none() || self.doc.cabinet_of(**id) == only)
            .map(|(id, _)| *id)
            .collect();
        let mut rows = Vec::new();
        for id in ids {
            let edges = self.effective_edges(id);
            let flat = self.flat_panel(id)?;
            let p = self.doc.panel(id).unwrap().clone();
            let cab = self.doc.cabinet_of(id).and_then(|c| self.doc.objects.get(&c)).and_then(|o| o.as_cabinet()).cloned();
            let (cname, room) = cab.map(|c| (c.name.clone(), c.room.clone())).unwrap_or_default();
            let full_name = match (cname.is_empty(), room.is_empty()) {
                (true, _) => p.name.clone(),
                (false, true) => format!("[{cname}] {}", p.name),
                (false, false) => format!("[{room} - {cname}] {}", p.name),
            };
            let band = |e: EdgeSide| edges.iter().find(|b| b.edge == e).map(|b| b.thickness_mm).unwrap_or(0.0);
            // Local X = width, Y = height; report the longer side as length.
            let (lx, ly) = (p.width_mm - band(EdgeSide::Left) - band(EdgeSide::Right), p.height_mm - band(EdgeSide::Bottom) - band(EdgeSide::Top));
            let (length, width, cut_length, cut_width) = if p.width_mm >= p.height_mm { (p.width_mm, p.height_mm, lx, ly) } else { (p.height_mm, p.width_mm, ly, lx) };
            let mut edge_codes: BTreeMap<String, f64> = BTreeMap::new();
            for b in &edges {
                let len = match b.edge {
                    EdgeSide::Left | EdgeSide::Right => p.height_mm,
                    _ => p.width_mm,
                };
                *edge_codes.entry(b.material_code.clone()).or_default() += len / 1000.0;
            }
            let edge_m: f64 = edge_codes.values().sum();
            let mut machining: BTreeMap<String, u32> = BTreeMap::new();
            for f in &flat.features {
                let k = match &f.feature {
                    MachiningFeature::Drill(d) => format!("Khoan Ø{}", d.diameter),
                    MachiningFeature::EdgeDrill(d) => format!("Khoan cạnh Ø{}", d.diameter),
                    MachiningFeature::Pocket(_) => "Hốc".into(),
                    MachiningFeature::Groove(g) => format!("Rãnh {}×{}", g.width, g.depth),
                    MachiningFeature::Contour(_) => "Đường bao".into(),
                };
                *machining.entry(k).or_default() += 1;
            }
            let tools = self
                .part_ref(id)
                .and_then(|(c, k)| self.cabinet_def(c).ok().and_then(|d| d.mods.get(&k).map(|m| m.tools.clone())))
                .unwrap_or_default();
            rows.push(CutRow {
                id,
                cabinet: cname,
                room,
                name: p.name.clone(),
                full_name,
                role: format!("{:?}", p.role),
                material_id: p.material_id.0.clone(),
                material: self.material_name(&p.material_id.0),
                length,
                width,
                thickness: p.thickness_mm,
                cut_length,
                cut_width,
                qty: 1,
                edges: edges.iter().map(|b| (b.edge, b.material_code.clone())).collect(),
                edge_m,
                edge_codes,
                machining: machining.into_iter().map(|(k, n)| format!("{k} ×{n}")).collect(),
                note: tools.join(", "),
                code: String::new(),
            });
        }
        rows.sort_by(|a, b| (&a.room, &a.cabinet, &a.name).cmp(&(&b.room, &b.cabinet, &b.name)));
        // Mã tấm: số thứ tự trong tủ (theo tên tấm), ổn định khi thứ tự tấm không đổi.
        let mut seq: BTreeMap<(String, String), u32> = BTreeMap::new();
        for r in rows.iter_mut() {
            let n = seq.entry((r.room.clone(), r.cabinet.clone())).or_default();
            *n += 1;
            let slug = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
            let head = [slug(&r.room), slug(&r.cabinet)].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join("-");
            r.code = if head.is_empty() { format!("T-{n:03}") } else { format!("{head}-{n:03}") };
        }
        Ok(rows)
    }

    /// Costing for one cabinet (or everything when `cabinet` is None).
    fn costing_for(&mut self, cabinet: Option<ObjectId>) -> Result<(Vec<Line>, Vec<Line>, Vec<Line>, Vec<CutRow>), CoreError> {
        let rows = self.cut_rows(cabinet)?;
        // Panels by (material, thickness).
        let mut panels: BTreeMap<(String, String), f64> = BTreeMap::new();
        for r in &rows {
            *panels.entry((r.material_id.clone(), format!("{}", r.thickness))).or_default() += r.length * r.width / 1e6;
        }
        let panel_lines: Vec<Line> = panels
            .into_iter()
            .map(|((m, t), area)| {
                let key = format!("panel:{m}|{t}");
                let price = self.price(&key);
                let area = (area * 100.0).round() / 100.0;
                Line { name: format!("{} | {t}mm", self.material_name(&m)), key, qty: area, unit: "m²".into(), price, amount: area * price, factor: None }
            })
            .collect();
        // Edge banding by code.
        let mut edges: BTreeMap<String, f64> = BTreeMap::new();
        for r in &rows {
            for (code, m) in &r.edge_codes {
                *edges.entry(code.clone()).or_default() += m;
            }
        }
        let edge_lines: Vec<Line> = edges
            .into_iter()
            .map(|(code, m)| {
                let factor = self.doc.settings.prices.get(&format!("edge_factor:{code}")).copied().unwrap_or(1.0);
                let key = format!("edge:{code}");
                let price = self.price(&key);
                let meters = (m * factor * 10.0).round() / 10.0;
                let name = match code.as_str() {
                    "DON-0.5" => "Đơn 0.5mm",
                    "DON-2" => "Đơn 2mm",
                    "KEP-1" => "Kép 1mm",
                    _ => "Đơn 1mm",
                };
                Line { key, name: name.into(), qty: meters, unit: "m".into(), price, amount: meters * price, factor: Some(factor) }
            })
            .collect();
        // Fittings from cabinet layouts + joint machining.
        let cabs: Vec<ObjectId> = self
            .doc
            .objects
            .iter()
            .filter(|(id, o)| matches!(o, DomainObject::Cabinet(_)) && (cabinet.is_none() || cabinet == Some(**id)))
            .map(|(id, _)| *id)
            .collect();
        let mut fit: BTreeMap<String, (String, f64, &'static str)> = BTreeMap::new();
        let mut add = |k: String, name: String, n: f64, unit: &'static str| {
            let e = fit.entry(k).or_insert((name, 0.0, unit));
            e.1 += n;
        };
        let (mut hinges, mut cups, mut slides, mut handles) = (0.0, 0.0, 0.0, 0.0);
        for c in &cabs {
            let Some(l) = self.doc.cabinet_layout(*c) else { continue };
            let f = l.fittings;
            if f.hinges > 0 {
                add("hinge".into(), "Bản lề giảm chấn".into(), f.hinges as f64, "Bộ");
                hinges += f.hinges as f64;
            }
            if f.shelf_pins > 0 {
                add("shelf_pin".into(), "Chốt tầng".into(), f.shelf_pins as f64, "Bộ");
            }
            for (len, n) in &f.slides {
                add(format!("slide:{len}"), format!("RayBi, Dài {len} mm"), *n as f64, "Bộ");
                slides += *n as f64;
            }
            for (len, n) in &f.undermount {
                add(format!("slide_um:{len}"), format!("Ray âm giảm chấn, Dài {len} mm"), *n as f64, "Bộ");
                slides += *n as f64;
            }
            for (len, n) in &f.tandem {
                add(format!("tandem:{len}"), format!("Hộp tandem, Dài {len} mm"), *n as f64, "Bộ");
                slides += *n as f64;
            }
            if f.knobs > 0 {
                add("knob".into(), "Núm tay nắm".into(), f.knobs as f64, "Cái");
                handles += f.knobs as f64;
            }
            if f.legs > 0 {
                add("leg".into(), "Chân nhựa tăng chỉnh".into(), f.legs as f64, "Cái");
            }
            if f.hangers > 0 {
                add("hanger".into(), "Ke treo tủ".into(), f.hangers as f64, "Cái");
            }
            if f.push_latches > 0 {
                add("push_open".into(), "Nhấn mở (push-open)".into(), f.push_latches as f64, "Bộ");
            }
            for (len, n) in &f.oval_rails {
                add(format!("oval_rail:{len}"), format!("Thanh Oval dài {len}"), *n as f64, "Cây");
            }
            if f.oval_cups > 0 {
                add("oval_cup".into(), "Chén oval".into(), f.oval_cups as f64, "Cái");
                cups += f.oval_cups as f64;
            }
            if f.handles > 0 {
                add("handle".into(), "Tay nắm".into(), f.handles as f64, "Cái");
                handles += f.handles as f64;
            }
            if f.sliding_tracks > 0 {
                add("sliding_track".into(), "Bộ ray cửa lùa".into(), f.sliding_tracks as f64, "Bộ");
            }
        }
        // Liên kết thùng: đếm theo lỗ thật (chốt gỗ, cam, vít) và ke góc.
        let joints = self.joint_features_all();
        let (mut dowels, mut cams, mut joint_screws) = (0usize, 0usize, 0usize);
        for (id, fs) in joints.iter() {
            if cabinet.is_some() && self.doc.cabinet_of(*id) != cabinet {
                continue;
            }
            for f in fs.iter().filter(|f| matches!(f.origin, FeatureOrigin::Joint { .. })) {
                match &f.feature {
                    MachiningFeature::Drill(d) if d.purpose == DrillPurpose::Dowel => dowels += 1,
                    MachiningFeature::Drill(d) if d.purpose == DrillPurpose::CamLock => cams += 1,
                    MachiningFeature::Drill(d) if d.purpose == DrillPurpose::Connector && d.diameter <= 5.0 && d.depth >= 15.0 => joint_screws += 1,
                    _ => {}
                }
            }
        }
        let brackets: u32 = self.brackets_all().iter().filter(|(id, _)| cabinet.is_none() || self.doc.cabinet_of(**id) == cabinet).map(|(_, n)| *n).sum();
        if dowels > 0 {
            add("dowel".into(), "Chốt gỗ".into(), dowels as f64, "Cái");
        }
        if cams > 0 {
            add("cam".into(), "Cam (minifix) + chốt cam".into(), cams as f64, "Bộ");
        }
        if joint_screws > 0 {
            add("joint_screw".into(), "Vít liên kết thùng".into(), joint_screws as f64, "Cái");
        }
        if brackets > 0 {
            add("bracket".into(), "Ke góc".into(), brackets as f64, "Cái");
        }
        let sr = self.doc.settings.screws.clone();
        let screws = hinges * sr.per_hinge as f64 + slides * sr.per_slide_set as f64 + cups * sr.per_oval_cup as f64 + handles * sr.per_handle as f64;
        if screws > 0.0 {
            add("screw".into(), "Vít".into(), screws, "Bộ");
        }
        let fit_lines: Vec<Line> = fit
            .into_iter()
            .map(|(k, (name, qty, unit))| {
                // Prices of length-specific items fall back to the generic key.
                let generic = k.split(':').next().unwrap_or(&k).to_string();
                let key = format!("fit:{k}");
                let price = self.doc.settings.prices.get(&key).or_else(|| self.doc.settings.prices.get(&format!("fit:{generic}"))).copied().unwrap_or(0.0);
                Line { key, name, qty, unit: unit.into(), price, amount: qty * price, factor: None }
            })
            .collect();
        Ok((panel_lines, edge_lines, fit_lines, rows))
    }

    pub(crate) fn costing(&mut self) -> Result<Value, CoreError> {
        let (panels, edges, fittings, cut) = self.costing_for(None)?;
        let sum = |v: &[Line]| v.iter().map(|l| l.amount).sum::<f64>();
        let total = sum(&panels) + sum(&edges) + sum(&fittings);
        // Cabinet list with each cabinet's own amount.
        let cabs: Vec<(ObjectId, aic_domain::Cabinet)> = self
            .doc
            .objects
            .iter()
            .filter_map(|(id, o)| o.as_cabinet().map(|c| (*id, c.clone())))
            .collect();
        let mut cabinets = Vec::new();
        for (id, c) in cabs {
            let (p, e, f, rows) = self.costing_for(Some(id))?;
            let v = |n: &str| self.doc.param_value(id, n).unwrap_or(0.0);
            cabinets.push(json!({
                "id": id,
                "room": c.room,
                "floor": c.floor,
                "name": c.name,
                "frame": c.kind.frame_name(),
                "size": [v("width"), v("height"), v("depth")],
                "panels": rows.len(),
                "amount": sum(&p) + sum(&e) + sum(&f),
                "kind": format!("{:?}", c.kind),
                "pricing": c.rules.pricing,
            }));
        }
        let groups = cut_groups(&cut);
        let quote = self.quote(&cabinets)?;
        Ok(json!({
            "panels": panels,
            "edges": edges,
            "fittings": fittings,
            "totals": { "panels": sum(&panels), "edges": sum(&edges), "fittings": sum(&fittings), "total": total },
            "cut_list": cut,
            "cut_groups": groups,
            "cabinets": cabinets,
            "quote": quote,
        }))
    }
}

/// Danh sách cắt gộp: các tấm cùng vật liệu, dày, kích thước cắt, dán cạnh và gia công.
fn cut_groups(rows: &[CutRow]) -> Vec<Value> {
    let mut groups: BTreeMap<String, (usize, Vec<String>, Vec<ObjectId>)> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let mut edges: Vec<String> = r.edges.iter().map(|(e, c)| format!("{e:?}:{c}")).collect();
        edges.sort();
        let key = format!("{}|{}|{:.1}|{:.1}|{}|{}", r.material_id, r.thickness, r.cut_length, r.cut_width, edges.join(","), r.machining.join(","));
        let g = groups.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            (i, Vec::new(), Vec::new())
        });
        g.1.push(r.code.clone());
        g.2.push(r.id);
    }
    order
        .iter()
        .map(|k| {
            let (i, codes, ids) = &groups[k];
            let r = &rows[*i];
            json!({
                "name": r.name,
                "material": r.material,
                "length": r.length,
                "width": r.width,
                "thickness": r.thickness,
                "cut_length": r.cut_length,
                "cut_width": r.cut_width,
                "qty": codes.len(),
                "edges": r.edges,
                "machining": r.machining,
                "codes": codes,
                "ids": ids,
            })
        })
        .collect()
}

impl Engine {
    fn setting(&self, key: &str, default: f64) -> f64 {
        self.doc.settings.prices.get(key).copied().unwrap_or(default)
    }

    /// Báo giá khách: mét dài (tủ bếp), m² mặt đứng (tủ áo, kệ…) hoặc bóc chi tiết, theo phòng.
    fn quote(&self, cabinets: &[Value]) -> Result<Value, CoreError> {
        use aic_domain::structure::PricingMode;
        let waste = self.setting("quote:waste_pct", 10.0) / 100.0;
        let labor = self.setting("quote:labor_pct", 15.0) / 100.0;
        let margin = self.setting("quote:margin_pct", 0.0) / 100.0;
        let vat = self.setting("quote:vat_pct", 8.0) / 100.0;
        let mut rows = Vec::new();
        let mut rooms: BTreeMap<(String, String), f64> = BTreeMap::new();
        for c in cabinets {
            let id: ObjectId = serde_json::from_value(c["id"].clone()).map_err(|_| CoreError::NotFound { id: ObjectId(0) })?;
            let kind = c["kind"].as_str().unwrap_or("");
            let mode: PricingMode = serde_json::from_value(c["pricing"].clone()).unwrap_or_default();
            let mode = match mode {
                PricingMode::Auto => {
                    if matches!(kind, "Base" | "Wall" | "Drawer") {
                        PricingMode::LinearM
                    } else {
                        PricingMode::FacadeM2
                    }
                }
                m => m,
            };
            let w = c["size"][0].as_f64().unwrap_or(0.0) / 1000.0;
            let h = c["size"][1].as_f64().unwrap_or(0.0) / 1000.0;
            let material = c["amount"].as_f64().unwrap_or(0.0);
            let (qty, unit, price, key) = match mode {
                PricingMode::LinearM => {
                    let key = format!("quote:linear:{}", kind.to_ascii_uppercase());
                    let def = match kind {
                        "Wall" => 3_500_000.0,
                        _ => 4_500_000.0,
                    };
                    (w, "m", self.setting(&key, def), key)
                }
                PricingMode::FacadeM2 => {
                    let key = format!("quote:facade:{}", kind.to_ascii_uppercase());
                    (w * h, "m²", self.setting(&key, 3_200_000.0), key)
                }
                _ => (1.0, "bộ", material * (1.0 + waste) * (1.0 + labor), String::new()),
            };
            let amount = qty * price;
            let (floor, room) = (c["floor"].as_str().unwrap_or("").to_string(), c["room"].as_str().unwrap_or("").to_string());
            *rooms.entry((floor.clone(), room.clone())).or_default() += amount;
            rows.push(json!({
                "id": id, "floor": floor, "room": room, "name": c["name"], "mode": mode,
                "qty": (qty * 1000.0).round() / 1000.0, "unit": unit, "price": price, "price_key": key, "amount": amount,
                "material_cost": material,
            }));
        }
        let subtotal: f64 = rooms.values().sum();
        let with_margin = subtotal * (1.0 + margin);
        let vat_amount = with_margin * vat;
        Ok(json!({
            "rows": rows,
            "rooms": rooms.iter().map(|((f, r), a)| json!({ "floor": f, "room": r, "amount": a })).collect::<Vec<_>>(),
            "settings": { "waste_pct": waste * 100.0, "labor_pct": labor * 100.0, "margin_pct": margin * 100.0, "vat_pct": vat * 100.0 },
            "subtotal": subtotal,
            "margin": with_margin - subtotal,
            "vat": vat_amount,
            "total": with_margin + vat_amount,
        }))
    }
}
