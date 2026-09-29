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

impl Nester for MaxRectsNester {
    fn nest(&self, job: &NestingJob) -> NestingResult {
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
