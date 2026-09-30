//! Nesting interface. The core defines the job/result contract; any engine
//! (the built-in MaxRects packer or an existing external engine) implements
//! [`Nester`]. The UI only sends jobs and draws placements.

use aic_domain::{MaterialId, ObjectId};
use aic_math::Polygon2D;
use serde::{Deserialize, Serialize};

pub type PartId = ObjectId;
pub type SheetId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrainConstraint {
    /// Part may be rotated freely (0/90).
    Free,
    /// Part's local Y must stay along the sheet's grain (sheet X axis) → placed at 90°.
    AlongSheetLength,
    /// Part's local X must stay along the sheet's grain → placed at 0°.
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestingPart {
    pub id: PartId,
    pub name: String,
    pub contour: Polygon2D,
    pub holes: Vec<Polygon2D>,
    pub quantity: u32,
    pub grain: GrainConstraint,
    pub material_id: MaterialId,
    /// Nối vân (D32): các tấm cùng nhóm được xếp liền nhau, cùng hướng, trên cùng một tấm ván.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain_group: Option<GrainGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrainGroup {
    pub name: String,
    /// Thứ tự trong nhóm (trái → phải / dưới → trên).
    pub order: u32,
    /// Vân chạy liên tục ngang (tấm cạnh nhau) hay dọc (tấm chồng nhau).
    pub vertical: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetSpec {
    pub material_id: MaterialId,
    pub width_mm: f64,
    pub height_mm: f64,
    pub thickness_mm: f64,
    /// Sheet grain runs along X (length) when true.
    pub has_grain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NestingSettings {
    /// Spacing between parts (>= tool diameter for through cuts).
    pub spacing_mm: f64,
    /// Unusable border around the sheet.
    pub margin_mm: f64,
    pub allow_rotation: bool,
}

impl Default for NestingSettings {
    fn default() -> Self {
        Self { spacing_mm: 12.0, margin_mm: 10.0, allow_rotation: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestingJob {
    pub parts: Vec<NestingPart>,
    pub sheet: SheetSpec,
    pub settings: NestingSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestingPlacement {
    pub part_id: PartId,
    /// Instance number for parts with quantity > 1.
    pub instance: u32,
    pub sheet_id: SheetId,
    /// Position of the part's local origin after rotation (lower-left of its bbox).
    pub x_mm: f64,
    pub y_mm: f64,
    pub rotation_deg: f64,
    /// Placed footprint size (after rotation).
    pub width_mm: f64,
    pub height_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetResult {
    pub id: SheetId,
    pub width_mm: f64,
    pub height_mm: f64,
    pub used_area_mm2: f64,
    pub utilization: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NestingResult {
    pub material_id: MaterialId,
    pub sheets: Vec<SheetResult>,
    pub placements: Vec<NestingPlacement>,
    /// Parts that do not fit on an empty sheet.
    pub unplaced: Vec<PartId>,
    pub waste_ratio: f64,
}

pub trait Nester {
    fn nest(&self, job: &NestingJob) -> NestingResult;
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x - 1e-9 && o.y >= self.y - 1e-9 && o.x + o.w <= self.x + self.w + 1e-9 && o.y + o.h <= self.y + self.h + 1e-9
    }
    fn intersects(&self, o: &Rect) -> bool {
        o.x < self.x + self.w - 1e-9 && self.x < o.x + o.w - 1e-9 && o.y < self.y + self.h - 1e-9 && self.y < o.y + o.h - 1e-9
    }
}

/// MaxRects (best short side fit) rectangular packer using part bounding boxes.
#[derive(Debug, Clone, Copy, Default)]
pub struct MaxRectsNester;

struct Bin {
    free: Vec<Rect>,
    used: f64,
}

impl Bin {
    fn new(w: f64, h: f64) -> Self {
        Self { free: vec![Rect { x: 0.0, y: 0.0, w, h }], used: 0.0 }
    }

    fn find(&self, w: f64, h: f64) -> Option<(Rect, f64)> {
        let mut best: Option<(Rect, f64)> = None;
        for f in &self.free {
            if w <= f.w + 1e-9 && h <= f.h + 1e-9 {
                let score = (f.w - w).min(f.h - h);
                if best.is_none_or(|b| score < b.1 || (score == b.1 && (f.y, f.x) < (b.0.y, b.0.x))) {
                    best = Some((Rect { x: f.x, y: f.y, w, h }, score));
                }
            }
        }
        best
    }

    fn place(&mut self, r: Rect) {
        let mut next = Vec::new();
        for f in self.free.drain(..) {
            if !f.intersects(&r) {
                next.push(f);
                continue;
            }
            if r.x > f.x {
                next.push(Rect { x: f.x, y: f.y, w: r.x - f.x, h: f.h });
            }
            if r.x + r.w < f.x + f.w {
                next.push(Rect { x: r.x + r.w, y: f.y, w: f.x + f.w - r.x - r.w, h: f.h });
            }
            if r.y > f.y {
                next.push(Rect { x: f.x, y: f.y, w: f.w, h: r.y - f.y });
            }
            if r.y + r.h < f.y + f.h {
                next.push(Rect { x: f.x, y: r.y + r.h, w: f.w, h: f.y + f.h - r.y - r.h });
            }
        }
        // Prune rectangles contained in others.
        let mut pruned: Vec<Rect> = Vec::new();
        for (i, a) in next.iter().enumerate() {
            let contained = next.iter().enumerate().any(|(j, b)| i != j && b.contains(a) && !(a.contains(b) && j > i));
            if !contained {
                pruned.push(*a);
            }
        }
        self.free = pruned;
    }
}

/// Khối nối vân: một part chữ nhật thay cả nhóm; `members` = (part, offset trong khối, w, h).
struct Block {
    id: PartId,
    vertical: bool,
    members: Vec<(PartId, f64, f64, f64)>,
}

/// Gộp mỗi nhóm nối vân thành một part chữ nhật (tấm liền nhau, cách nhau `spacing`).
fn merge_groups(job: &NestingJob) -> (NestingJob, Vec<Block>) {
    let sp = job.settings.spacing_mm;
    let mut out = job.clone();
    let mut blocks = Vec::new();
    let mut names: Vec<&str> = job.parts.iter().filter_map(|p| p.grain_group.as_ref().map(|g| g.name.as_str())).collect();
    names.sort();
    names.dedup();
    for name in names {
        let mut members: Vec<&NestingPart> = job.parts.iter().filter(|p| p.grain_group.as_ref().is_some_and(|g| g.name == name)).collect();
        if members.len() < 2 {
            continue;
        }
        members.sort_by_key(|p| p.grain_group.as_ref().map(|g| g.order).unwrap_or(0));
        let vertical = members[0].grain_group.as_ref().is_some_and(|g| g.vertical);
        let mut at = 0.0;
        let mut cross: f64 = 0.0;
        let mut list = Vec::new();
        for m in &members {
            let (mn, mx) = m.contour.bounds();
            let (w, h) = (mx.x - mn.x, mx.y - mn.y);
            list.push((m.id, at, w, h));
            at += if vertical { h } else { w } + sp;
            cross = cross.max(if vertical { w } else { h });
        }
        let len = at - sp;
        let (bw, bh) = if vertical { (cross, len) } else { (len, cross) };
        // Khối không vừa một tấm ván (cả khi xoay) → xếp rời như thường, không bỏ sót tấm.
        let (uw, uh) = (job.sheet.width_mm - 2.0 * job.settings.margin_mm, job.sheet.height_mm - 2.0 * job.settings.margin_mm);
        if !((bw <= uw && bh <= uh) || (bh <= uw && bw <= uh)) {
            continue;
        }
        let first = members[0];
        let ids: Vec<PartId> = members.iter().map(|m| m.id).collect();
        out.parts.retain(|p| !ids.contains(&p.id));
        out.parts.push(NestingPart {
            id: first.id,
            name: format!("Nối vân {name}"),
            contour: Polygon2D::rect(0.0, 0.0, bw, bh),
            holes: Vec::new(),
            quantity: 1,
            grain: first.grain,
            material_id: first.material_id.clone(),
            grain_group: None,
        });
        blocks.push(Block { id: first.id, vertical, members: list });
    }
    (out, blocks)
}

impl Nester for MaxRectsNester {
    fn nest(&self, job: &NestingJob) -> NestingResult {
        let (merged, blocks) = merge_groups(job);
        let mut r = self.nest_rects(&merged);
        if blocks.is_empty() {
            return r;
        }
        // Tách khối về từng tấm: cùng sheet, cùng góc xoay, liền nhau theo thứ tự nhóm.
        let mut placements = Vec::new();
        for p in r.placements.drain(..) {
            let Some(b) = blocks.iter().find(|b| b.id == p.part_id) else {
                placements.push(p);
                continue;
            };
            let rotated = (p.rotation_deg - 90.0).abs() < 1e-6;
            for &(id, off, w, h) in &b.members {
                // Không xoay: dọc trục khối; xoay 90°: trục khối thành trục y của ván (và ngược lại).
                let along_x = b.vertical == rotated;
                let (x, y) = if along_x { (p.x_mm + off, p.y_mm) } else { (p.x_mm, p.y_mm + off) };
                let (fw, fh) = if rotated { (h, w) } else { (w, h) };
                placements.push(NestingPlacement { part_id: id, instance: 0, sheet_id: p.sheet_id, x_mm: x, y_mm: y, rotation_deg: p.rotation_deg, width_mm: fw, height_mm: fh });
            }
        }
        if r.unplaced.iter().any(|u| blocks.iter().any(|b| b.id == *u)) {
            let extra: Vec<PartId> = blocks.iter().filter(|b| r.unplaced.contains(&b.id)).flat_map(|b| b.members.iter().map(|m| m.0)).collect();
            r.unplaced.retain(|u| !blocks.iter().any(|b| b.id == *u));
            r.unplaced.extend(extra);
        }
        r.placements = placements;
        r
    }
}

impl MaxRectsNester {
    fn nest_rects(&self, job: &NestingJob) -> NestingResult {
        let s = &job.settings;
        let sp = s.spacing_mm;
        // Usable area; each part is inflated by the spacing on its right/top.
        let uw = job.sheet.width_mm - 2.0 * s.margin_mm + sp;
        let uh = job.sheet.height_mm - 2.0 * s.margin_mm + sp;

        let mut instances: Vec<(&NestingPart, u32, f64, f64)> = Vec::new();
        for p in &job.parts {
            let (mn, mx) = p.contour.bounds();
            for k in 0..p.quantity {
                instances.push((p, k, mx.x - mn.x, mx.y - mn.y));
            }
        }
        // Largest area first, then longest side.
        instances.sort_by(|a, b| (b.2 * b.3).total_cmp(&(a.2 * a.3)).then(b.2.max(b.3).total_cmp(&a.2.max(a.3))));

        let mut bins: Vec<Bin> = Vec::new();
        let mut placements = Vec::new();
        let mut unplaced = Vec::new();
        for (part, inst, w, h) in instances {
            let can_rotate = s.allow_rotation && (part.grain == GrainConstraint::Free || !job.sheet.has_grain);
            let can_rotate = can_rotate && !(job.sheet.has_grain && part.grain == GrainConstraint::Fixed);
            // Grain: part local Y (height) along sheet X => rotate by 90 when grained.
            let grained = job.sheet.has_grain && part.grain == GrainConstraint::AlongSheetLength;
            let mut options = vec![if grained { (h, w, 90.0) } else { (w, h, 0.0) }];
            if can_rotate {
                options.push((h, w, 90.0));
            }
            let mut done = false;
            'bins: for (bi, bin) in bins.iter_mut().enumerate() {
                let mut best: Option<(Rect, f64, f64)> = None;
                for (ow, oh, rot) in &options {
                    if let Some((r, score)) = bin.find(ow + sp, oh + sp) {
                        if best.is_none_or(|b| score < b.1) {
                            best = Some((r, score, *rot));
                        }
                    }
                }
                if let Some((r, _, rot)) = best {
                    bin.place(r);
                    bin.used += (r.w - sp) * (r.h - sp);
                    placements.push(NestingPlacement {
                        part_id: part.id,
                        instance: inst,
                        sheet_id: bi as u32,
                        x_mm: r.x + s.margin_mm,
                        y_mm: r.y + s.margin_mm,
                        rotation_deg: rot,
                        width_mm: r.w - sp,
                        height_mm: r.h - sp,
                    });
                    done = true;
                    break 'bins;
                }
            }
            if !done {
                let mut bin = Bin::new(uw, uh);
                let mut placed = false;
                for (ow, oh, rot) in &options {
                    if let Some((r, _)) = bin.find(ow + sp, oh + sp) {
                        bin.place(r);
                        bin.used += ow * oh;
                        placements.push(NestingPlacement {
                            part_id: part.id,
                            instance: inst,
                            sheet_id: bins.len() as u32,
                            x_mm: r.x + s.margin_mm,
                            y_mm: r.y + s.margin_mm,
                            rotation_deg: *rot,
                            width_mm: *ow,
                            height_mm: *oh,
                        });
                        placed = true;
                        break;
                    }
                }
                if placed {
                    bins.push(bin);
                } else {
                    unplaced.push(part.id);
                }
            }
        }
        let sheet_area = job.sheet.width_mm * job.sheet.height_mm;
        let sheets: Vec<SheetResult> = bins
            .iter()
            .enumerate()
            .map(|(i, b)| SheetResult {
                id: i as u32,
                width_mm: job.sheet.width_mm,
                height_mm: job.sheet.height_mm,
                used_area_mm2: b.used,
                utilization: b.used / sheet_area,
            })
            .collect();
        let used: f64 = sheets.iter().map(|s| s.used_area_mm2).sum();
        let total = sheet_area * sheets.len() as f64;
        NestingResult {
            material_id: job.sheet.material_id.clone(),
            sheets,
            placements,
            unplaced,
            waste_ratio: if total > 0.0 { 1.0 - used / total } else { 0.0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(id: u64, w: f64, h: f64, q: u32) -> NestingPart {
        NestingPart {
            id: ObjectId(id),
            name: format!("P{id}"),
            contour: Polygon2D::rect(0.0, 0.0, w, h),
            holes: vec![],
            quantity: q,
            grain: GrainConstraint::Free,
            material_id: MaterialId::new("M"),
            grain_group: None,
        }
    }

    #[test]
    fn grain_group_parts_are_placed_adjacent_in_order() {
        let mut parts: Vec<NestingPart> = (1..=4).map(|i| part(i, 450.0, 2000.0, 1)).collect();
        for (k, p) in parts.iter_mut().enumerate() {
            p.grain_group = Some(GrainGroup { name: "A".into(), order: k as u32, vertical: false });
        }
        parts.push(part(9, 300.0, 300.0, 3));
        let job = NestingJob { parts, sheet: SheetSpec { material_id: MaterialId::new("M"), width_mm: 2440.0, height_mm: 2440.0, thickness_mm: 18.0, has_grain: false }, settings: NestingSettings { allow_rotation: false, ..Default::default() } };
        let r = MaxRectsNester.nest(&job);
        let mut g: Vec<&NestingPlacement> = r.placements.iter().filter(|p| p.part_id.0 <= 4).collect();
        assert_eq!(g.len(), 4);
        g.sort_by_key(|p| p.part_id.0);
        assert!(g.iter().all(|p| p.sheet_id == g[0].sheet_id && p.rotation_deg == g[0].rotation_deg && (p.y_mm - g[0].y_mm).abs() < 1e-9));
        for w in g.windows(2) {
            assert!((w[1].x_mm - (w[0].x_mm + 450.0 + 12.0)).abs() < 1e-6, "adjacent in order: {:?}", g);
        }
    }

    #[test]
    fn packs_without_overlap() {
        let job = NestingJob {
            parts: vec![part(1, 560.0, 720.0, 4), part(2, 764.0, 540.0, 6), part(3, 400.0, 300.0, 5)],
            sheet: SheetSpec { material_id: MaterialId::new("M"), width_mm: 2440.0, height_mm: 1220.0, thickness_mm: 18.0, has_grain: false },
            settings: NestingSettings::default(),
        };
        let r = MaxRectsNester.nest(&job);
        assert_eq!(r.placements.len(), 15);
        assert!(r.unplaced.is_empty());
        for (i, a) in r.placements.iter().enumerate() {
            assert!(a.x_mm >= 10.0 - 1e-9 && a.x_mm + a.width_mm <= 2430.0 + 1e-9);
            assert!(a.y_mm >= 10.0 - 1e-9 && a.y_mm + a.height_mm <= 1210.0 + 1e-9);
            for b in &r.placements[i + 1..] {
                if a.sheet_id != b.sheet_id {
                    continue;
                }
                let sep = a.x_mm + a.width_mm + 12.0 <= b.x_mm + 1e-6
                    || b.x_mm + b.width_mm + 12.0 <= a.x_mm + 1e-6
                    || a.y_mm + a.height_mm + 12.0 <= b.y_mm + 1e-6
                    || b.y_mm + b.height_mm + 12.0 <= a.y_mm + 1e-6;
                assert!(sep, "overlap {a:?} {b:?}");
            }
        }
        assert!(r.waste_ratio > 0.0 && r.waste_ratio < 1.0);
    }

    #[test]
    fn oversize_part_is_unplaced() {
        let job = NestingJob {
            parts: vec![part(1, 3000.0, 2000.0, 1)],
            sheet: SheetSpec { material_id: MaterialId::new("M"), width_mm: 2440.0, height_mm: 1220.0, thickness_mm: 18.0, has_grain: false },
            settings: NestingSettings::default(),
        };
        assert_eq!(MaxRectsNester.nest(&job).unplaced, vec![ObjectId(1)]);
    }
}
