//! Panel outline tools (P15): 04 Cắt tự do, 09 Cắt theo tấm, 10 Bo/Vác góc, 06 Hợp tấm.
//! The result is stored as machining in the panel's local frame: an outer
//! `Contour` (shape override), an inner through contour or a pocket. Generated
//! parts keep it in their cabinet's part mods; free panels in their features.

use crate::zones::bad;
use crate::Engine;
use aic_domain::{ContourFeature, DomainObject, FaceSide, MachiningFeature, ObjectId, PocketFeature};
use aic_manufacturing::polygon_ops;
use aic_math::{Point2, Polygon2D, Transform3D};
use aic_project::{Command, CoreError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const TOOL_FREE_CUT: &str = "04. Cắt tự do";
pub const TOOL_CUT_BY_PANEL: &str = "09. Cắt theo tấm";
pub const TOOL_CORNERS: &str = "10. Bo/Vác góc";
pub const TOOL_MERGE: &str = "06. Hợp tấm";
/// Tools whose result is the outer shape (removed together by "bỏ hình dạng").
pub const SHAPE_TOOLS: [&str; 3] = [TOOL_FREE_CUT, TOOL_CUT_BY_PANEL, TOOL_CORNERS];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Corner {
    BottomLeft,
    BottomRight,
    TopRight,
    TopLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Keep {
    /// Keep the larger piece.
    #[default]
    Auto,
    /// Keep the piece left of the directed line a → b.
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShapeOp {
    /// Bo góc (radius) or vác góc (chamfer) on corners of the panel's outline.
    Corners { corners: Vec<Corner>, size: f64, #[serde(default)] chamfer: bool },
    /// Straight cut along the line a → b (local mm, may extend past the panel).
    CutLine { a: [f64; 2], b: [f64; 2], #[serde(default)] keep: Keep },
    /// Remove where the cutter panel passes through (+ clearance on every side).
    CutByPanel { cutter: ObjectId, #[serde(default)] clearance: f64 },
}

/// Current outer outline of a panel (last outer contour, else its rectangle).
fn outline(features: &[MachiningFeature], w: f64, h: f64) -> Polygon2D {
    features
        .iter()
        .rev()
        .find_map(|f| match f {
            MachiningFeature::Contour(c) if !c.inner => Some(c.polygon.clone()),
            _ => None,
        })
        .unwrap_or_else(|| Polygon2D::rect(0.0, 0.0, w, h))
        .ensure_ccw()
}

fn near(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}

/// Round or chamfer the polygon vertices lying on the chosen bounding-box corners.
pub fn round_corners(poly: &Polygon2D, corners: &[Corner], size: f64, chamfer: bool) -> Result<Polygon2D, String> {
    if size <= 0.0 || !size.is_finite() {
        return Err("size must be > 0".into());
    }
    let (mn, mx) = poly.bounds();
    let targets: Vec<Point2> = corners
        .iter()
        .map(|c| match c {
            Corner::BottomLeft => Point2::new(mn.x, mn.y),
            Corner::BottomRight => Point2::new(mx.x, mn.y),
            Corner::TopRight => Point2::new(mx.x, mx.y),
            Corner::TopLeft => Point2::new(mn.x, mx.y),
        })
        .collect();
    let p = &poly.points;
    let n = p.len();
    let mut out = Vec::with_capacity(n + corners.len() * 12);
    let mut hit = 0;
    for i in 0..n {
        let v = p[i];
        if !targets.iter().any(|t| near(*t, v)) {
            out.push(v);
            continue;
        }
        hit += 1;
        let (u, w) = (p[(i + n - 1) % n], p[(i + 1) % n]);
        let (l1, l2) = (((u.x - v.x).powi(2) + (u.y - v.y).powi(2)).sqrt(), ((w.x - v.x).powi(2) + (w.y - v.y).powi(2)).sqrt());
        let d1 = Point2::new((u.x - v.x) / l1, (u.y - v.y) / l1);
        let d2 = Point2::new((w.x - v.x) / l2, (w.y - v.y) / l2);
        let cos = (d1.x * d2.x + d1.y * d2.y).clamp(-1.0, 1.0);
        let theta = cos.acos(); // interior angle
        if theta < 1e-3 || theta > std::f64::consts::PI - 1e-3 {
            out.push(v);
            continue;
        }
        // Tangent distance from the vertex along each edge.
        let tl = if chamfer { size } else { size / (theta / 2.0).tan() };
        if tl > l1.min(l2) * 0.999 {
            return Err(format!("corner size {size} too large for the panel"));
        }
        let a = Point2::new(v.x + d1.x * tl, v.y + d1.y * tl);
        let b = Point2::new(v.x + d2.x * tl, v.y + d2.y * tl);
        if chamfer {
            out.push(a);
            out.push(b);
            continue;
        }
        // Arc centre on the bisector.
        let bis = Point2::new(d1.x + d2.x, d1.y + d2.y);
        let bl = (bis.x * bis.x + bis.y * bis.y).sqrt();
        let dist = size / (theta / 2.0).sin();
        let c = Point2::new(v.x + bis.x / bl * dist, v.y + bis.y / bl * dist);
        let a0 = (a.y - c.y).atan2(a.x - c.x);
        let mut a1 = (b.y - c.y).atan2(b.x - c.x);
        // Take the short way round.
        let mut sweep = a1 - a0;
        while sweep > std::f64::consts::PI {
            sweep -= std::f64::consts::TAU;
        }
        while sweep < -std::f64::consts::PI {
            sweep += std::f64::consts::TAU;
        }
        a1 = a0 + sweep;
        let segs = ((sweep.abs() / (std::f64::consts::PI / 2.0)) * 12.0).ceil().max(2.0) as usize;
        for s in 0..=segs {
            let t = a0 + (a1 - a0) * s as f64 / segs as f64;
            out.push(Point2::new(c.x + size * t.cos(), c.y + size * t.sin()));
        }
    }
    if hit == 0 {
        return Err("no matching corner".into());
    }
    Ok(Polygon2D::new(out))
}

/// Keep one side of the line a → b.
pub fn cut_line(poly: &Polygon2D, a: [f64; 2], b: [f64; 2], keep: Keep) -> Result<Polygon2D, String> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return Err("cut line needs two distinct points".into());
    }
    let (ux, uy) = (dx / len, dy / len);
    let big = 1e6;
    let half = |left: bool| {
        let s = if left { 1.0 } else { -1.0 };
        let (nx, ny) = (-uy * s, ux * s);
        let p0 = Point2::new(a[0] - ux * big, a[1] - uy * big);
        let p1 = Point2::new(a[0] + ux * big, a[1] + uy * big);
        Polygon2D::new(vec![p0, p1, Point2::new(p1.x + nx * big, p1.y + ny * big), Point2::new(p0.x + nx * big, p0.y + ny * big)])
    };
    let l = poly.clip_convex(&half(true));
    let r = poly.clip_convex(&half(false));
    let pick = match keep {
        Keep::Left => l,
        Keep::Right => r,
        Keep::Auto => {
            if l.area() >= r.area() {
                l
            } else {
                r
            }
        }
    };
    if pick.points.len() < 3 || pick.area() < 1.0 {
        return Err("the cut removes the whole panel".into());
    }
    if (pick.area() - poly.area()).abs() < 1e-6 {
        return Err("the line does not cross the panel".into());
    }
    Ok(pick.ensure_ccw())
}

/// Result of cutting by another panel.
pub enum CutResult {
    Outline(Polygon2D),
    Feature(MachiningFeature),
}

/// Cut `poly` (panel `w × h × t`) by a box given as its 8 corners in the panel's local frame.
pub fn cut_by_box(poly: &Polygon2D, t: f64, corners: &[[f64; 3]; 8], clearance: f64) -> Result<CutResult, String> {
    let (mut x0, mut y0, mut z0) = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1, mut z1) = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for c in corners {
        x0 = x0.min(c[0]);
        y0 = y0.min(c[1]);
        z0 = z0.min(c[2]);
        x1 = x1.max(c[0]);
        y1 = y1.max(c[1]);
        z1 = z1.max(c[2]);
    }
    let eps = 0.01;
    if z1 <= eps || z0 >= t - eps {
        return Err("the panels do not intersect".into());
    }
    let cl = clearance.max(0.0);
    let rect = Polygon2D::rect(x0 - cl, y0 - cl, x1 - x0 + 2.0 * cl, y1 - y0 + 2.0 * cl);
    let (mn, mx) = poly.bounds();
    if x1 + cl <= mn.x + eps || x0 - cl >= mx.x - eps || y1 + cl <= mn.y + eps || y0 - cl >= mx.y - eps {
        return Err("the panels do not intersect".into());
    }
    let through = z0 <= eps && z1 >= t - eps;
    if !through {
        // Partial depth: a pocket from the face the cutter enters.
        let side = if z1 >= t - eps { FaceSide::A } else { FaceSide::B };
        let depth = if side == FaceSide::A { t - z0 } else { z1 };
        let (px0, py0) = ((x0 - cl).max(mn.x), (y0 - cl).max(mn.y));
        let (px1, py1) = ((x1 + cl).min(mx.x), (y1 + cl).min(mx.y));
        return Ok(CutResult::Feature(MachiningFeature::Pocket(PocketFeature {
            x: px0,
            y: py0,
            width: px1 - px0,
            height: py1 - py0,
            depth: depth + 0.0,
            side,
            corner_radius: 0.0,
        })));
    }
    let inside = x0 - cl > mn.x + eps && x1 + cl < mx.x - eps && y0 - cl > mn.y + eps && y1 + cl < mx.y - eps;
    if inside {
        return Ok(CutResult::Feature(MachiningFeature::Contour(ContourFeature { polygon: rect, inner: true, depth: t })));
    }
    let mut pieces = polygon_ops::difference(std::slice::from_ref(poly), &[rect]);
    pieces.sort_by(|a, b| b.area().total_cmp(&a.area()));
    let Some(best) = pieces.into_iter().next() else { return Err("the cut removes the whole panel".into()) };
    Ok(CutResult::Outline(best.ensure_ccw()))
}

fn outer_feature(p: Polygon2D, t: f64) -> MachiningFeature {
    MachiningFeature::Contour(ContourFeature { polygon: p, inner: false, depth: t })
}

impl Engine {
    fn panel_info(&self, id: ObjectId) -> Result<([f64; 3], Vec<MachiningFeature>), CoreError> {
        let p = self.doc.panel(id).ok_or(CoreError::NotFound { id })?;
        // Generated parts carry their machining (incl. tool results) in gen_features.
        let mut f = p.features.clone();
        f.extend(p.gen_features.iter().cloned());
        Ok(([p.width_mm, p.height_mm, p.thickness_mm], f))
    }

    /// Store new machining on a panel: `outline` replaces the outer shape, `extra` is appended.
    fn store_shape(&mut self, id: ObjectId, outline: Option<Polygon2D>, extra: Vec<MachiningFeature>, tool: &str) -> Result<Command, CoreError> {
        let (size, features) = self.panel_info(id)?;
        let t = size[2];
        if let Some((cab, key)) = self.part_ref(id) {
            let mut def = self.cabinet_def(cab)?;
            let m = def.mods.entry(key).or_default();
            if let Some(o) = outline {
                m.features.retain(|f| !matches!(f, MachiningFeature::Contour(c) if !c.inner));
                m.features.push(outer_feature(o, t));
            }
            m.features.extend(extra);
            if !m.tools.iter().any(|x| x == tool) {
                m.tools.push(tool.to_string());
            }
            return Ok(Command::SetCabinet { id: cab, cabinet: Box::new(def), label: tool.into() });
        }
        if !matches!(self.doc.objects.get(&id), Some(DomainObject::Panel(_))) {
            return Err(bad("part", "not a panel"));
        }
        let mut cmds = Vec::new();
        if let Some(o) = outline {
            // Remove old outer contours from the end so indices stay valid.
            for (i, f) in features.iter().enumerate().rev() {
                if matches!(f, MachiningFeature::Contour(c) if !c.inner) {
                    cmds.push(Command::RemoveFeature { id, index: i });
                }
            }
            cmds.push(Command::AddFeature { id, feature: outer_feature(o, t), index: None });
        }
        cmds.extend(extra.into_iter().map(|f| Command::AddFeature { id, feature: f, index: None }));
        Ok(Command::Batch { label: tool.into(), commands: cmds })
    }

    /// Box corners of `cutter` expressed in `target`'s local frame.
    fn corners_in(&self, cutter: ObjectId, target: ObjectId) -> Result<[[f64; 3]; 8], CoreError> {
        let (size, _) = self.panel_info(cutter)?;
        let to_target: Transform3D = self.doc.scene.world(target).inverse().compose(&self.doc.scene.world(cutter));
        let mut out = [[0.0; 3]; 8];
        for (i, o) in out.iter_mut().enumerate() {
            let c = [
                if i & 1 != 0 { size[0] } else { 0.0 },
                if i & 2 != 0 { size[1] } else { 0.0 },
                if i & 4 != 0 { size[2] } else { 0.0 },
            ];
            *o = to_target.transform_point(c);
        }
        Ok(out)
    }

    pub(crate) fn shape_tool(&mut self, ids: &[ObjectId], op: &ShapeOp) -> Result<Value, CoreError> {
        if ids.is_empty() {
            return Err(bad("shape", "select at least one panel"));
        }
        let mut cmds = Vec::new();
        let mut skipped = Vec::new();
        for &id in ids {
            if let ShapeOp::CutByPanel { cutter, .. } = op {
                if *cutter == id {
                    continue;
                }
            }
            let ([w, h, t], features) = self.panel_info(id)?;
            let poly = outline(&features, w, h);
            let (outline_new, extra, tool) = match op {
                ShapeOp::Corners { corners, size, chamfer } => {
                    match round_corners(&poly, corners, *size, *chamfer) {
                        Ok(p) => (Some(p), vec![], TOOL_CORNERS),
                        Err(e) => {
                            skipped.push(json!({ "id": id, "reason": e }));
                            continue;
                        }
                    }
                }
                ShapeOp::CutLine { a, b, keep } => match cut_line(&poly, *a, *b, *keep) {
                    Ok(p) => (Some(p), vec![], TOOL_FREE_CUT),
                    Err(e) => {
                        skipped.push(json!({ "id": id, "reason": e }));
                        continue;
                    }
                },
                ShapeOp::CutByPanel { cutter, clearance } => {
                    let corners = self.corners_in(*cutter, id)?;
                    match cut_by_box(&poly, t, &corners, *clearance) {
                        Ok(CutResult::Outline(p)) => (Some(p), vec![], TOOL_CUT_BY_PANEL),
                        Ok(CutResult::Feature(f)) => (None, vec![f], TOOL_CUT_BY_PANEL),
                        Err(e) => {
                            skipped.push(json!({ "id": id, "reason": e }));
                            continue;
                        }
                    }
                }
            };
            cmds.push(self.store_shape(id, outline_new, extra, tool)?);
        }
        if cmds.is_empty() {
            return Err(bad("shape", skipped.first().and_then(|s| s["reason"].as_str()).unwrap_or("nothing to do").to_string()));
        }
        let n = cmds.len();
        let cmd = if n == 1 { cmds.pop().unwrap() } else { Command::Batch { label: "Tool hình dạng".into(), commands: cmds } };
        self.exec_cmd(cmd)?;
        Ok(json!({ "changed": n, "skipped": skipped }))
    }

    /// Hợp tấm: extend the first panel over the others (same cabinet, same plane and
    /// thickness) and delete them. A non-rectangular union keeps its outline.
    pub(crate) fn merge_panels(&mut self, ids: &[ObjectId]) -> Result<Value, CoreError> {
        if ids.len() < 2 {
            return Err(bad("merge", "select two or more panels"));
        }
        let a = ids[0];
        let (cab, key_a) = self.part_ref(a).ok_or_else(|| bad("part", "merge works on cabinet parts"))?;
        let ([w, h, t], _) = self.panel_info(a)?;
        let mut rects = vec![Polygon2D::rect(0.0, 0.0, w, h)];
        let mut keys = Vec::new();
        for &b in &ids[1..] {
            let (cb, key_b) = self.part_ref(b).ok_or_else(|| bad("part", "merge works on cabinet parts"))?;
            if cb != cab {
                return Err(bad("merge", "panels belong to different cabinets"));
            }
            let (sb, _) = self.panel_info(b)?;
            if (sb[2] - t).abs() > 0.05 {
                return Err(bad("merge", "panels have different thickness"));
            }
            let c = self.corners_in(b, a)?;
            let zs: Vec<f64> = c.iter().map(|p| p[2]).collect();
            let (z0, z1) = (zs.iter().cloned().fold(f64::INFINITY, f64::min), zs.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            if z0.abs() > 0.5 || (z1 - t).abs() > 0.5 {
                return Err(bad("merge", "panels are not in the same plane"));
            }
            let (x0, x1) = (c.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min), c.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max));
            let (y0, y1) = (c.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min), c.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max));
            rects.push(Polygon2D::rect(x0, y0, x1 - x0, y1 - y0));
            keys.push(key_b);
        }
        let union = polygon_ops::union(&rects[..1], &rects[1..]);
        if union.len() != 1 {
            return Err(bad("merge", "panels do not touch"));
        }
        let u = union.into_iter().next().unwrap().ensure_ccw();
        let (mn, mx) = u.bounds();
        let mut def = self.cabinet_def(cab)?;
        for k in keys {
            def.mods.entry(k).or_default().deleted = true;
        }
        let m = def.mods.entry(key_a).or_default();
        let (dl, db) = (-mn.x, -mn.y);
        for f in &mut m.features {
            aic_domain::layout::shift_feature(f, dl, db);
        }
        m.extend[0] += dl;
        m.extend[1] += mx.x - w;
        m.extend[2] += db;
        m.extend[3] += mx.y - h;
        let bbox = (mx.x - mn.x) * (mx.y - mn.y);
        if u.area() < bbox - 1.0 {
            let mut shifted = u.clone();
            for p in &mut shifted.points {
                p.x += dl;
                p.y += db;
            }
            m.features.retain(|f| !matches!(f, MachiningFeature::Contour(c) if !c.inner));
            m.features.push(outer_feature(shifted, t));
        }
        if !m.tools.iter().any(|x| x == TOOL_MERGE) {
            m.tools.push(TOOL_MERGE.into());
        }
        self.exec_cmd(Command::SetCabinet { id: cab, cabinet: Box::new(def), label: TOOL_MERGE.into() })?;
        Ok(json!({ "id": a, "size": [mx.x - mn.x, mx.y - mn.y] }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_corners_keep_area_close() {
        let r = Polygon2D::rect(0.0, 0.0, 600.0, 400.0);
        let p = round_corners(&r, &[Corner::TopRight, Corner::TopLeft], 50.0, false).unwrap();
        let expect = 600.0 * 400.0 - 2.0 * (50.0 * 50.0 - std::f64::consts::PI * 50.0 * 50.0 / 4.0);
        assert!((p.area() - expect).abs() < 30.0, "{} vs {}", p.area(), expect);
        let c = round_corners(&r, &[Corner::BottomLeft], 30.0, true).unwrap();
        assert!((c.area() - (240000.0 - 450.0)).abs() < 1e-6);
        assert!(round_corners(&r, &[Corner::BottomLeft], 500.0, false).is_err());
    }

    #[test]
    fn cut_line_keeps_larger_piece() {
        let r = Polygon2D::rect(0.0, 0.0, 600.0, 400.0);
        let p = cut_line(&r, [500.0, 0.0], [600.0, 100.0], Keep::Auto).unwrap();
        assert!((p.area() - (240000.0 - 5000.0)).abs() < 1e-6);
        assert!(cut_line(&r, [700.0, 0.0], [700.0, 10.0], Keep::Auto).is_err());
    }

    #[test]
    fn cut_by_box_notch_hole_and_pocket() {
        let r = Polygon2D::rect(0.0, 0.0, 600.0, 400.0);
        let bx = |x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64| {
            let mut c = [[0.0; 3]; 8];
            for (i, o) in c.iter_mut().enumerate() {
                *o = [if i & 1 != 0 { x1 } else { x0 }, if i & 2 != 0 { y1 } else { y0 }, if i & 4 != 0 { z1 } else { z0 }];
            }
            c
        };
        // Notch at the edge → outline.
        match cut_by_box(&r, 18.0, &bx(580.0, 100.0, -5.0, 700.0, 118.0, 30.0), 0.0).unwrap() {
            CutResult::Outline(p) => assert!((p.area() - (240000.0 - 20.0 * 18.0)).abs() < 1e-3),
            _ => panic!("expected outline"),
        }
        // Through the middle → inner contour.
        assert!(matches!(cut_by_box(&r, 18.0, &bx(100.0, 100.0, -5.0, 200.0, 150.0, 30.0), 1.0).unwrap(), CutResult::Feature(MachiningFeature::Contour(c)) if c.inner));
        // Half depth → pocket on face A.
        assert!(matches!(cut_by_box(&r, 18.0, &bx(100.0, 100.0, 10.0, 200.0, 150.0, 30.0), 0.0).unwrap(), CutResult::Feature(MachiningFeature::Pocket(p)) if p.side == FaceSide::A && (p.depth - 8.0).abs() < 1e-9));
        assert!(cut_by_box(&r, 18.0, &bx(100.0, 100.0, 20.0, 200.0, 150.0, 30.0), 0.0).is_err());
    }
}
