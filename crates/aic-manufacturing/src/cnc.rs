//! CNC toolpath and G-code generation for one nested sheet.
//!
//! Sheet frame: X/Y on the sheet, Z = 0 at the top surface (face A up),
//! negative Z into the material. Features on face B or in narrow edges cannot
//! be machined on a flat-bed router in this setup and are reported as warnings.

use crate::flatten::FlatPanel;
use crate::polygon_ops::{inside_toolpath, outside_toolpath};
use aic_domain::{Axis2, FaceSide, MachiningFeature, ObjectId};
use aic_math::Polygon2D;
use aic_nesting::NestingPlacement;
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolKind {
    Drill,
    EndMill,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub id: u32,
    pub name: String,
    pub kind: ToolKind,
    pub diameter: f64,
    pub rpm: u32,
    /// Cutting feed, mm/min.
    pub feed: f64,
    /// Plunge feed, mm/min.
    pub plunge: f64,
    /// Max depth per pass (end mills).
    pub step_down: f64,
}

pub fn default_tools() -> Vec<Tool> {
    let t = |id, name: &str, kind, d, rpm, feed, plunge, step| Tool {
        id,
        name: name.into(),
        kind,
        diameter: d,
        rpm,
        feed,
        plunge,
        step_down: step,
    };
    vec![
        t(1, "Phay ngón 12 mm", ToolKind::EndMill, 12.0, 18000, 9000.0, 3000.0, 10.0),
        t(2, "Phay ngón 6 mm", ToolKind::EndMill, 6.0, 20000, 5000.0, 2000.0, 5.0),
        t(3, "Mũi khoan 5 mm", ToolKind::Drill, 5.0, 6000, 2000.0, 2000.0, 100.0),
        t(4, "Mũi khoan 8 mm", ToolKind::Drill, 8.0, 6000, 2000.0, 2000.0, 100.0),
        t(5, "Mũi khoét bản lề 35 mm", ToolKind::Drill, 35.0, 4000, 1500.0, 1500.0, 100.0),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Move {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rapid: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub index: usize,
    pub tool_id: u32,
    pub kind: String,
    pub part_id: ObjectId,
    pub moves: Vec<Move>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProgramStats {
    pub tool_changes: usize,
    pub cut_length_mm: f64,
    pub rapid_length_mm: f64,
    pub estimated_time_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CncProgram {
    pub sheet_id: u32,
    pub sheet_width: f64,
    pub sheet_height: f64,
    pub thickness: f64,
    pub tools: Vec<Tool>,
    pub operations: Vec<Operation>,
    pub gcode: String,
    pub warnings: Vec<String>,
    pub stats: ProgramStats,
}

pub struct SheetPart<'a> {
    pub flat: &'a FlatPanel,
    pub placement: &'a NestingPlacement,
}

const SAFE_Z: f64 = 20.0;
const THROUGH_EXTRA: f64 = 0.3;

/// Map a part-local point into sheet coordinates.
fn to_sheet(p: &SheetPart, x: f64, y: f64) -> (f64, f64) {
    let pl = p.placement;
    if (pl.rotation_deg - 90.0).abs() < 1e-6 {
        (pl.x_mm + (p.flat.height - y), pl.y_mm + x)
    } else {
        (pl.x_mm + x, pl.y_mm + y)
    }
}

fn poly_to_sheet(p: &SheetPart, poly: &Polygon2D) -> Polygon2D {
    Polygon2D::new(poly.points.iter().map(|q| {
        let (x, y) = to_sheet(p, q.x, q.y);
        aic_math::Point2::new(x, y)
    }).collect())
}

fn depth_passes(total: f64, step: f64) -> Vec<f64> {
    let n = (total / step).ceil().max(1.0) as usize;
    (1..=n).map(|i| -(total * i as f64 / n as f64)).collect()
}

fn closed_loop(poly: &Polygon2D, depths: &[f64]) -> Vec<Move> {
    let mut m = Vec::new();
    let Some(first) = poly.points.first() else { return m };
    m.push(Move { x: first.x, y: first.y, z: SAFE_Z, rapid: true });
    for &z in depths {
        m.push(Move { x: first.x, y: first.y, z, rapid: false });
        for q in poly.points.iter().skip(1).chain(std::iter::once(first)) {
            m.push(Move { x: q.x, y: q.y, z, rapid: false });
        }
    }
    m.push(Move { x: first.x, y: first.y, z: SAFE_Z, rapid: true });
    m
}

fn circle(cx: f64, cy: f64, r: f64) -> Polygon2D {
    let n = 24;
    Polygon2D::new(
        (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                aic_math::Point2::new(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect(),
    )
}

/// Zig-zag raster inside a tool-centre boundary rectangle.
fn raster(bound: &Polygon2D, step: f64, depths: &[f64]) -> Vec<Move> {
    let (mn, mx) = bound.bounds();
    let mut m = Vec::new();
    m.push(Move { x: mn.x, y: mn.y, z: SAFE_Z, rapid: true });
    for &z in depths {
        let mut y = mn.y;
        let mut left = true;
        m.push(Move { x: mn.x, y: mn.y, z, rapid: false });
        loop {
            let (a, b) = if left { (mn.x, mx.x) } else { (mx.x, mn.x) };
            m.push(Move { x: a, y, z, rapid: false });
            m.push(Move { x: b, y, z, rapid: false });
            if y >= mx.y - 1e-9 {
                break;
            }
            y = (y + step).min(mx.y);
            left = !left;
        }
        // Finishing pass along the boundary.
        m.extend(closed_loop(bound, &[z]).into_iter().filter(|mv| !mv.rapid));
    }
    m.push(Move { x: mn.x, y: mn.y, z: SAFE_Z, rapid: true });
    m
}

pub fn generate_program(sheet_id: u32, sheet_w: f64, sheet_h: f64, thickness: f64, parts: &[SheetPart], tools: &[Tool]) -> CncProgram {
    let mut ops: Vec<Operation> = Vec::new();
    let mut warnings = Vec::new();
    let find_drill = |d: f64| tools.iter().find(|t| t.kind == ToolKind::Drill && (t.diameter - d).abs() < 1e-6);
    let mills: Vec<&Tool> = {
        let mut v: Vec<&Tool> = tools.iter().filter(|t| t.kind == ToolKind::EndMill).collect();
        v.sort_by(|a, b| b.diameter.total_cmp(&a.diameter));
        v
    };
    let contour_tool = mills.first().copied();
    let fine_mill = mills.last().copied();

    let mut add = |tool: &Tool, kind: &str, part: ObjectId, moves: Vec<Move>| {
        ops.push(Operation { index: 0, tool_id: tool.id, kind: kind.into(), part_id: part, moves });
    };

    for p in parts {
        let pid = p.flat.id;
        for f in &p.flat.features {
            match &f.feature {
                MachiningFeature::Drill(d) => {
                    if d.side == FaceSide::B {
                        warnings.push(format!("{}: khoan Ø{} mặt B cần lật tấm", p.flat.name, d.diameter));
                        continue;
                    }
                    let (x, y) = to_sheet(p, d.x, d.y);
                    let z = if d.depth >= thickness { -(thickness + THROUGH_EXTRA) } else { -d.depth };
                    if let Some(tool) = find_drill(d.diameter) {
                        add(tool, "DRILL", pid, vec![
                            Move { x, y, z: SAFE_Z, rapid: true },
                            Move { x, y, z, rapid: false },
                            Move { x, y, z: SAFE_Z, rapid: true },
                        ]);
                    } else if let Some(m) = mills.iter().rev().find(|m| m.diameter < d.diameter) {
                        // Circular interpolation with a smaller end mill.
                        let path = circle(x, y, (d.diameter - m.diameter) / 2.0);
                        add(m, "DRILL", pid, closed_loop(&path, &depth_passes(-z, m.step_down)));
                    } else {
                        warnings.push(format!("{}: không có dao cho lỗ Ø{}", p.flat.name, d.diameter));
                    }
                }
                MachiningFeature::EdgeDrill(d) => {
                    warnings.push(format!("{}: khoan cạnh {:?} Ø{} cần máy khoan ngang", p.flat.name, d.edge, d.diameter));
                }
                MachiningFeature::Pocket(pk) => {
                    let Some(m) = fine_mill else { continue };
                    if pk.side == FaceSide::B {
                        warnings.push(format!("{}: pocket mặt B cần lật tấm", p.flat.name));
                        continue;
                    }
                    let rect = poly_to_sheet(p, &Polygon2D::rect(pk.x, pk.y, pk.width, pk.height));
                    for b in inside_toolpath(&rect, m.diameter) {
                        add(m, "POCKET", pid, raster(&b, m.diameter * 0.6, &depth_passes(pk.depth, m.step_down)));
                    }
                }
                MachiningFeature::Groove(g) => {
                    let Some(m) = fine_mill else { continue };
                    if g.side == FaceSide::B {
                        warnings.push(format!("{}: rãnh mặt B cần lật tấm", p.flat.name));
                        continue;
                    }
                    let (w, h) = match g.direction {
                        Axis2::X => (g.length, g.width),
                        Axis2::Y => (g.width, g.length),
                    };
                    let rect = poly_to_sheet(p, &Polygon2D::rect(g.x, g.y, w, h));
                    let bounds = inside_toolpath(&rect, m.diameter);
                    if bounds.is_empty() {
                        // Groove as wide as the tool: single centre line.
                        let (mn, mx) = rect.bounds();
                        let (cx, cy) = ((mn.x + mx.x) / 2.0, (mn.y + mx.y) / 2.0);
                        let line = if mx.x - mn.x >= mx.y - mn.y {
                            Polygon2D::new(vec![aic_math::Point2::new(mn.x, cy), aic_math::Point2::new(mx.x, cy)])
                        } else {
                            Polygon2D::new(vec![aic_math::Point2::new(cx, mn.y), aic_math::Point2::new(cx, mx.y)])
                        };
                        add(m, "GROOVE", pid, closed_loop(&line, &depth_passes(g.depth, m.step_down)));
                    } else {
                        for b in bounds {
                            add(m, "GROOVE", pid, raster(&b, m.diameter * 0.6, &depth_passes(g.depth, m.step_down)));
                        }
                    }
                }
                MachiningFeature::Contour(_) => {}
            }
        }
    }
    // Inner contours then outer contours last so parts stay held until the end.
    if let Some(m) = contour_tool {
        for p in parts {
            for inner in &p.flat.inner {
                for b in inside_toolpath(&poly_to_sheet(p, inner), m.diameter) {
                    add(m, "CONTOUR", p.flat.id, closed_loop(&b, &depth_passes(thickness + THROUGH_EXTRA, m.step_down)));
                }
            }
        }
        for p in parts {
            for b in outside_toolpath(&poly_to_sheet(p, &p.flat.outer), m.diameter) {
                add(m, "CONTOUR", p.flat.id, closed_loop(&b, &depth_passes(thickness + THROUGH_EXTRA, m.step_down)));
            }
        }
    }

    // Group by tool to minimise tool changes, keeping contours last.
    let rank = |o: &Operation| if o.kind == "CONTOUR" { 1 } else { 0 };
    ops.sort_by_key(|o| (rank(o), o.tool_id));
    for (i, o) in ops.iter_mut().enumerate() {
        o.index = i;
    }
    let (gcode, stats) = emit_gcode(sheet_id, &ops, tools);
    CncProgram {
        sheet_id,
        sheet_width: sheet_w,
        sheet_height: sheet_h,
        thickness,
        tools: tools.to_vec(),
        operations: ops,
        gcode,
        warnings,
        stats,
    }
}

fn emit_gcode(sheet_id: u32, ops: &[Operation], tools: &[Tool]) -> (String, ProgramStats) {
    let mut g = String::new();
    let mut st = ProgramStats::default();
    let _ = writeln!(g, "(AIC CAD - sheet {})", sheet_id + 1);
    let _ = writeln!(g, "G21 G90 G17");
    let _ = writeln!(g, "G0 Z{SAFE_Z:.3}");
    let mut current: Option<u32> = None;
    let mut last = (0.0, 0.0, SAFE_Z);
    for op in ops {
        let Some(tool) = tools.iter().find(|t| t.id == op.tool_id) else { continue };
        if current != Some(tool.id) {
            if current.is_some() {
                let _ = writeln!(g, "M5");
            }
            let _ = writeln!(g, "(Tool change: {})", tool.name);
            let _ = writeln!(g, "T{} M6", tool.id);
            let _ = writeln!(g, "S{} M3", tool.rpm);
            let _ = writeln!(g, "G0 Z{SAFE_Z:.3}");
            current = Some(tool.id);
            st.tool_changes += 1;
        }
        let _ = writeln!(g, "({} part {})", op.kind, op.part_id.0);
        for m in &op.moves {
            let d = ((m.x - last.0).powi(2) + (m.y - last.1).powi(2) + (m.z - last.2).powi(2)).sqrt();
            if m.rapid {
                st.rapid_length_mm += d;
                st.estimated_time_s += d / 30000.0 * 60.0;
                let _ = writeln!(g, "G0 X{:.3} Y{:.3} Z{:.3}", m.x, m.y, m.z);
            } else {
                let plunging = m.z < last.2 - 1e-9 && (m.x - last.0).abs() < 1e-9 && (m.y - last.1).abs() < 1e-9;
                let f = if plunging { tool.plunge } else { tool.feed };
                st.cut_length_mm += d;
                st.estimated_time_s += d / f * 60.0;
                let _ = writeln!(g, "G1 X{:.3} Y{:.3} Z{:.3} F{:.0}", m.x, m.y, m.z, f);
            }
            last = (m.x, m.y, m.z);
        }
    }
    let _ = writeln!(g, "M5");
    let _ = writeln!(g, "G0 Z{SAFE_Z:.3}");
    let _ = writeln!(g, "M30");
    st.estimated_time_s += st.tool_changes as f64 * 15.0;
    (g, st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{DerivedFeature, FeatureOrigin};
    use crate::flatten::flatten;
    use aic_domain::{DrillFeature, DrillPurpose, MaterialId, Panel, PanelRole};

    #[test]
    fn program_for_one_part() {
        let mut panel = Panel::new(ObjectId(7), "Side", PanelRole::LeftSide, [560.0, 720.0, 18.0], MaterialId::new("M"));
        panel.features.push(MachiningFeature::Drill(DrillFeature { x: 37.0, y: 100.0, diameter: 5.0, depth: 10.0, side: FaceSide::A, purpose: DrillPurpose::ShelfPin }));
        let flat = flatten(&panel, vec![DerivedFeature {
            feature: MachiningFeature::Drill(DrillFeature { x: 50.0, y: 9.0, diameter: 8.0, depth: 12.0, side: FaceSide::A, purpose: DrillPurpose::Dowel }),
            origin: FeatureOrigin::Rule,
        }]);
        let pl = NestingPlacement { part_id: ObjectId(7), instance: 0, sheet_id: 0, x_mm: 10.0, y_mm: 10.0, rotation_deg: 90.0, width_mm: 720.0, height_mm: 560.0 };
        let prog = generate_program(0, 2440.0, 1220.0, 18.0, &[SheetPart { flat: &flat, placement: &pl }], &default_tools());
        assert_eq!(prog.operations.len(), 3);
        assert_eq!(prog.operations.last().unwrap().kind, "CONTOUR");
        assert!(prog.gcode.contains("T3 M6") && prog.gcode.contains("T4 M6") && prog.gcode.contains("M30"));
        assert_eq!(prog.stats.tool_changes, 3);
        // Rotated placement keeps every move on the sheet (contour offset by the tool radius).
        for op in &prog.operations {
            for m in &op.moves {
                assert!(m.x >= 4.0 - 1e-6 && m.x <= 10.0 + 720.0 + 6.0 + 1e-6, "{m:?}");
                assert!(m.y >= 4.0 - 1e-6 && m.y <= 10.0 + 560.0 + 6.0 + 1e-6, "{m:?}");
            }
        }
    }
}
