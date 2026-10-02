//! Dãy tủ (run): các tủ liền nhau cùng hướng. Dãy sinh phần chạy qua nhiều tủ —
//! mặt đá, len chân liền, tấm lấp hai đầu, tấm che trần — từ hộp bao của các tủ
//! trong khung tọa độ của dãy (x dọc dãy, y lên, z ra trước). Hàm thuần: core
//! gọi lại mỗi khi tủ trong dãy đổi kích thước / vị trí.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunRules {
    /// Mặt đá / mặt bàn phủ trên dãy.
    pub countertop: bool,
    pub top_thickness: f64,
    /// Nhô trước so với mặt thùng.
    pub overhang_front: f64,
    pub overhang_left: f64,
    pub overhang_right: f64,
    /// Mã vật liệu mặt (đá thạch anh 20 mặc định).
    pub top_material: String,
    /// Len chân liền suốt dãy (tủ trong dãy bỏ len riêng).
    pub continuous_plinth: bool,
    pub plinth_setback: f64,
    /// Tấm lấp đầu dãy (0 = không).
    pub filler_left: f64,
    pub filler_right: f64,
    /// Cao độ trần (mm từ sàn). > đỉnh dãy → tấm che trần.
    pub ceiling: f64,
    /// Khoét trên mặt đá (chậu rửa, bếp âm).
    pub cutouts: Vec<Cutout>,
}

/// Lỗ khoét trên mặt đá, đo từ mép trái mặt đá và mép trước.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cutout {
    /// `SINK` (chậu) / `HOB` (bếp).
    pub kind: String,
    pub x: f64,
    pub width: f64,
    pub depth: f64,
    pub from_front: f64,
    /// Bo góc lỗ khoét.
    pub radius: f64,
}

impl Default for Cutout {
    fn default() -> Self {
        Self { kind: "SINK".into(), x: 200.0, width: 780.0, depth: 430.0, from_front: 80.0, radius: 10.0 }
    }
}

/// Đa giác lỗ khoét (tọa độ local mặt đá: x từ trái, y từ mép trước), bo góc `r`.
pub fn cutout_polygon(c: &Cutout) -> aic_math::Polygon2D {
    use aic_math::Point2;
    let (x0, y0, x1, y1) = (c.x, c.from_front, c.x + c.width, c.from_front + c.depth);
    let r = c.radius.clamp(0.0, c.width.min(c.depth) / 2.0 - 0.01).max(0.0);
    if r < 0.5 {
        return aic_math::Polygon2D::new(vec![Point2::new(x0, y0), Point2::new(x1, y0), Point2::new(x1, y1), Point2::new(x0, y1)]);
    }
    let mut pts = Vec::new();
    let n = 6;
    for (cx, cy, a0) in [(x1 - r, y0 + r, -90.0f64), (x1 - r, y1 - r, 0.0), (x0 + r, y1 - r, 90.0), (x0 + r, y0 + r, 180.0)] {
        for i in 0..=n {
            let a = (a0 + 90.0 * i as f64 / n as f64).to_radians();
            pts.push(Point2::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    aic_math::Polygon2D::new(pts)
}

impl Default for RunRules {
    fn default() -> Self {
        Self {
            countertop: true,
            top_thickness: 20.0,
            overhang_front: 20.0,
            overhang_left: 0.0,
            overhang_right: 0.0,
            top_material: "STONE20-WHITE".into(),
            continuous_plinth: false,
            plinth_setback: 50.0,
            filler_left: 0.0,
            filler_right: 0.0,
            ceiling: 0.0,
            cutouts: Vec::new(),
        }
    }
}

/// Hộp bao một tủ trong khung dãy + cao chân của tủ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub plinth: f64,
}

/// Một tấm do dãy sinh ra (khung dãy; kích thước = rộng × cao × dày của tấm).
#[derive(Debug, Clone, PartialEq)]
pub struct RunPart {
    pub key: &'static str,
    pub name: &'static str,
    /// Kích thước tấm (W, H, T).
    pub size: [f64; 3],
    /// Góc min của tấm trong khung dãy.
    pub translation: [f64; 3],
    /// Xoay (độ) của tấm: nằm ngang = [-90, 0, 0], đứng dọc sâu = [0, 90, 0].
    pub rotation_deg: [f64; 3],
    pub material: Option<String>,
    /// Gia công (lỗ khoét mặt đá), tọa độ local của tấm.
    pub features: Vec<crate::MachiningFeature>,
}

const FLAT: [f64; 3] = [-90.0, 0.0, 0.0];
const SIDE: [f64; 3] = [0.0, 90.0, 0.0];

/// Sinh các tấm của dãy. `t` = dày ván thùng (len chân, tấm lấp, che trần).
pub fn build(rules: &RunRules, boxes: &[RunBox], t: f64) -> Vec<RunPart> {
    let mut out = Vec::new();
    if boxes.is_empty() {
        return out;
    }
    let x0 = boxes.iter().map(|b| b.min[0]).fold(f64::MAX, f64::min) - rules.filler_left.max(0.0);
    let x1 = boxes.iter().map(|b| b.max[0]).fold(f64::MIN, f64::max) + rules.filler_right.max(0.0);
    let top = boxes.iter().map(|b| b.max[1]).fold(f64::MIN, f64::max);
    let bottom = boxes.iter().map(|b| b.min[1]).fold(f64::MAX, f64::min);
    let back = boxes.iter().map(|b| b.min[2]).fold(f64::MAX, f64::min);
    let front = boxes.iter().map(|b| b.max[2]).fold(f64::MIN, f64::max);
    let plinth = boxes.iter().map(|b| b.plinth).fold(0.0, f64::max);
    let depth = front - back;
    if rules.countertop {
        let w = x1 - x0 + rules.overhang_left + rules.overhang_right;
        let d = depth + rules.overhang_front;
        // Tấm nằm ngang: local Y (chiều "cao" của tấm) chạy từ trước ra sau.
        out.push(RunPart { key: "top", name: "MặtĐá", size: [w, d, rules.top_thickness], translation: [x0 - rules.overhang_left, top, front + rules.overhang_front], rotation_deg: FLAT, material: Some(rules.top_material.clone()), features: rules.cutouts.iter().filter(|c| c.x > 0.0 && c.from_front > 0.0 && c.x + c.width < w && c.from_front + c.depth < d).map(|c| crate::MachiningFeature::Contour(crate::ContourFeature { polygon: cutout_polygon(c), inner: true, depth: rules.top_thickness })).collect() });
    }
    if rules.continuous_plinth && plinth > 1.0 {
        out.push(RunPart { key: "plinth", name: "LenChânDãy", size: [x1 - x0, plinth, t], translation: [x0, bottom, front - rules.plinth_setback - t], rotation_deg: [0.0; 3], material: None, features: Vec::new() });
    }
    let h = top - bottom;
    if rules.filler_left > 0.0 {
        out.push(RunPart { key: "filler_l", name: "TấmLấpTrái", size: [rules.filler_left, h, t], translation: [x0, bottom, front - t], rotation_deg: [0.0; 3], material: None, features: Vec::new() });
    }
    if rules.filler_right > 0.0 {
        out.push(RunPart { key: "filler_r", name: "TấmLấpPhải", size: [rules.filler_right, h, t], translation: [x1 - rules.filler_right, bottom, front - t], rotation_deg: [0.0; 3], material: None, features: Vec::new() });
    }
    let above = if rules.countertop { top + rules.top_thickness } else { top };
    if rules.ceiling > above + 1.0 {
        out.push(RunPart { key: "ceiling", name: "CheTrần", size: [x1 - x0, rules.ceiling - above, t], translation: [x0, above, front - t], rotation_deg: [0.0; 3], material: None, features: Vec::new() });
    }
    let _ = SIDE;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countertop_spans_the_run() {
        let b = |x0: f64, w: f64| RunBox { min: [x0, 0.0, 0.0], max: [x0 + w, 850.0, 600.0], plinth: 100.0 };
        let r = RunRules { continuous_plinth: true, filler_right: 50.0, ceiling: 0.0, ..Default::default() };
        let parts = build(&r, &[b(0.0, 800.0), b(800.0, 600.0), b(1400.0, 900.0)], 17.2);
        let top = parts.iter().find(|p| p.key == "top").unwrap();
        assert_eq!(top.size, [2350.0, 620.0, 20.0]);
        let pl = parts.iter().find(|p| p.key == "plinth").unwrap();
        assert_eq!(pl.size[0], 2350.0);
        assert!(parts.iter().any(|p| p.key == "filler_r"));
        let r = RunRules { cutouts: vec![Cutout::default(), Cutout { x: 5000.0, ..Default::default() }], ..Default::default() };
        let parts = build(&r, &[b(0.0, 1200.0)], 17.2);
        assert_eq!(parts[0].features.len(), 1, "out-of-top cutout skipped");
    }
}
