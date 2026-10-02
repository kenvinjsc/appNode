//! Snapping of a moving box against neighbours. The UI only proposes a raw
//! translation and displays the returned hints; the decision is made here.

use aic_domain::ObjectId;
use aic_math::Aabb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SnapKind {
    /// Face-to-face contact (moving.min == other.max or vice versa).
    Face,
    /// Coplanar alignment of faces (min == min / max == max).
    Align,
    Center,
    Grid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapHint {
    pub kind: SnapKind,
    /// 0 = X, 1 = Y, 2 = Z.
    pub axis: usize,
    /// World coordinate of the snapped plane.
    pub value: f64,
    pub target: Option<ObjectId>,
    /// Distance between the moving box and the target along the other axes
    /// (for dimension hints), if meaningful.
    pub distance: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SnapSettings {
    pub tolerance_mm: f64,
    pub grid_mm: Option<f64>,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self { tolerance_mm: 15.0, grid_mm: Some(10.0) }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapResult {
    pub delta: [f64; 3],
    pub hints: Vec<SnapHint>,
}

/// Snap `delta` applied to `moving` against `others`, axis by axis.
pub fn snap_translation(moving: &Aabb, delta: [f64; 3], others: &[(ObjectId, Aabb)], s: &SnapSettings) -> SnapResult {
    let mut out = delta;
    let mut hints = Vec::new();
    let moved = Aabb { min: [0, 1, 2].map(|i| moving.min[i] + delta[i]), max: [0, 1, 2].map(|i| moving.max[i] + delta[i]) };
    // `axis` chỉ số chung cho moved / others / out / hints, giữ vòng chỉ số cho dễ đọc.
    #[allow(clippy::needless_range_loop)]
    for axis in 0..3 {
        let (mn, mx) = (moved.min[axis], moved.max[axis]);
        let mc = 0.5 * (mn + mx);
        let mut best: Option<(f64, SnapHint)> = None;
        for (id, o) in others {
            // Only consider neighbours that overlap on at least one other axis (nearby).
            let near = (0..3).filter(|k| *k != axis).any(|k| moved.min[k] <= o.max[k] + s.tolerance_mm && o.min[k] <= moved.max[k] + s.tolerance_mm);
            if !near {
                continue;
            }
            let oc = 0.5 * (o.min[axis] + o.max[axis]);
            let cands = [
                (o.max[axis] - mn, SnapKind::Face, o.max[axis]),
                (o.min[axis] - mx, SnapKind::Face, o.min[axis]),
                (o.min[axis] - mn, SnapKind::Align, o.min[axis]),
                (o.max[axis] - mx, SnapKind::Align, o.max[axis]),
                (oc - mc, SnapKind::Center, oc),
            ];
            for (corr, kind, value) in cands {
                if corr.abs() <= s.tolerance_mm && best.as_ref().is_none_or(|b| corr.abs() < b.0.abs()) {
                    best = Some((corr, SnapHint { kind, axis, value, target: Some(*id), distance: None }));
                }
            }
        }
        match best {
            Some((corr, hint)) => {
                out[axis] += corr;
                hints.push(hint);
            }
            None => {
                if let Some(g) = s.grid_mm.filter(|g| *g > 0.0) {
                    let snapped = (mn / g).round() * g;
                    out[axis] += snapped - mn;
                    hints.push(SnapHint { kind: SnapKind::Grid, axis, value: snapped, target: None, distance: None });
                }
            }
        }
    }
    SnapResult { delta: out, hints }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_face_to_face() {
        let moving = Aabb { min: [0.0; 3], max: [100.0, 100.0, 100.0] };
        let other = Aabb { min: [200.0, 0.0, 0.0], max: [300.0, 100.0, 100.0] };
        let r = snap_translation(&moving, [95.0, 0.0, 0.0], &[(ObjectId(2), other)], &SnapSettings { tolerance_mm: 10.0, grid_mm: None });
        assert_eq!(r.delta[0], 100.0);
        assert_eq!(r.hints[0].kind, SnapKind::Face);
    }
}
