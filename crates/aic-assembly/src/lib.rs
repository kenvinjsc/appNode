//! Assembly graph: contact relations between parts.
//!
//! Pipeline: AABB sort-and-sweep → candidate pairs → SAT narrow phase (exact
//! for face contacts) → contact region by convex clipping → [`AssemblyRelation`].
//! Relations are stored once (source id < target id); the reverse view is derived.

use aic_domain::ObjectId;
use aic_math::{Obb, Point2, Polygon2D, Vector3};
use aic_spatial::{obb_separation, SpatialIndex, SpatialItem};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContactType {
    Touch,
    Gap,
    Penetrate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrientationType {
    Parallel,
    Perpendicular,
    Oblique,
}

/// Which part of a panel takes part in the contact: its big FACE, a long EDGE
/// or a short END.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegionType {
    Face,
    Edge,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyRelation {
    pub source: ObjectId,
    pub target: ObjectId,

    pub contact: ContactType,
    pub orientation: OrientationType,
    pub source_region: RegionType,
    pub target_region: RegionType,

    pub gap_mm: f64,
    pub penetration_mm: f64,
    pub contact_area_mm2: f64,
    pub angle_deg: f64,
    /// Contact normal in world space, pointing from source to target.
    pub normal: [f64; 3],
    /// Contact region polygon in world space.
    pub contact_region: Vec<[f64; 3]>,
    /// Local axis (0=X,1=Y,2=Z) and side (+1/-1) of the source/target box faces in contact.
    pub source_face: (usize, i8),
    pub target_face: (usize, i8),
}

impl AssemblyRelation {
    /// View of this relation from the target's side (derived, never stored).
    pub fn reversed(&self) -> AssemblyRelation {
        AssemblyRelation {
            source: self.target,
            target: self.source,
            source_region: self.target_region,
            target_region: self.source_region,
            normal: self.normal.map(|v| -v),
            source_face: self.target_face,
            target_face: self.source_face,
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RelationSettings {
    /// Gaps larger than this are not recorded.
    pub max_gap_mm: f64,
    /// |distance| below this is a TOUCH.
    pub touch_tolerance_mm: f64,
    /// Parallel/perpendicular tolerance.
    pub angle_tolerance_deg: f64,
}

impl Default for RelationSettings {
    fn default() -> Self {
        Self { max_gap_mm: 5.0, touch_tolerance_mm: 0.01, angle_tolerance_deg: 0.5 }
    }
}

/// Region classification of face `axis` of a box with `size`.
fn region_of(size: [f64; 3], axis: usize) -> RegionType {
    match axis {
        2 => RegionType::Face,
        // The x-faces span height; the y-faces span width.
        0 => {
            if size[1] >= size[0] {
                RegionType::Edge
            } else {
                RegionType::End
            }
        }
        _ => {
            if size[0] >= size[1] {
                RegionType::Edge
            } else {
                RegionType::End
            }
        }
    }
}

/// Face of `obb` whose outward normal is best aligned with `dir`.
fn facing(obb: &Obb, dir: &Vector3<f64>) -> (usize, i8, f64) {
    let axes = obb.axes();
    let mut best = (0, 1i8, -1.0);
    for (i, a) in axes.iter().enumerate() {
        let d = a.dot(dir);
        if d.abs() > best.2 {
            best = (i, if d >= 0.0 { 1 } else { -1 }, d.abs());
        }
    }
    best
}

/// Corners of face (axis, side) of a box, world space.
fn face_corners(obb: &Obb, axis: usize, side: i8) -> Vec<Vector3<f64>> {
    let s = obb.size;
    let (u, v) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let fixed = if side > 0 { s[axis] } else { 0.0 };
    let iso = obb.transform.to_isometry();
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .iter()
        .map(|(a, b)| {
            let mut p = [0.0; 3];
            p[axis] = fixed;
            p[u] = a * s[u];
            p[v] = b * s[v];
            (iso * aic_math::Point3::new(p[0], p[1], p[2])).coords
        })
        .collect()
}

fn plane_basis(n: &Vector3<f64>) -> (Vector3<f64>, Vector3<f64>) {
    let helper = if n.x.abs() < 0.9 { Vector3::x() } else { Vector3::y() };
    let u = n.cross(&helper).normalize();
    let v = n.cross(&u);
    (u, v)
}

/// Classify a pair of boxes. Returns `None` when they are too far apart or only
/// touch along a line/point.
pub fn classify_pair(
    (ida, a): (ObjectId, &Obb),
    (idb, b): (ObjectId, &Obb),
    s: &RelationSettings,
) -> Option<AssemblyRelation> {
    // Canonical direction: smaller id is the source.
    let ((ida, a), (idb, b)) = if ida <= idb { ((ida, a), (idb, b)) } else { ((idb, b), (ida, a)) };
    let sep = obb_separation(a, b);
    if sep.distance > s.max_gap_mm {
        return None;
    }
    let n = sep.axis;
    let contact = if sep.distance > s.touch_tolerance_mm {
        ContactType::Gap
    } else if sep.distance < -s.touch_tolerance_mm {
        ContactType::Penetrate
    } else {
        ContactType::Touch
    };

    let (fa_axis, fa_side, _) = facing(a, &n);
    let (fb_axis, fb_side, _) = facing(b, &-n);
    let (u, v) = plane_basis(&n);
    let to2d = |p: &Vector3<f64>| Point2::new(p.dot(&u), p.dot(&v));
    let pa = Polygon2D::new(face_corners(a, fa_axis, fa_side).iter().map(to2d).collect());
    let pb = Polygon2D::new(face_corners(b, fb_axis, fb_side).iter().map(to2d).collect());
    let region = pa.clone().ensure_ccw().clip_convex(&pb);
    let area = region.area();
    if contact != ContactType::Penetrate && area < 0.01 {
        return None;
    }
    // Place the region on A's contact plane.
    let plane_d = face_corners(a, fa_axis, fa_side)[0].dot(&n);
    let contact_region = region.points.iter().map(|p| {
        let w = u * p.x + v * p.y + n * plane_d;
        [w.x, w.y, w.z]
    }).collect();

    let za = a.axes()[2];
    let zb = b.axes()[2];
    let angle_deg = za.dot(&zb).abs().clamp(0.0, 1.0).acos().to_degrees();
    let orientation = if angle_deg <= s.angle_tolerance_deg {
        OrientationType::Parallel
    } else if (90.0 - angle_deg).abs() <= s.angle_tolerance_deg {
        OrientationType::Perpendicular
    } else {
        OrientationType::Oblique
    };

    let r = |x: f64| (x * 1e6).round() / 1e6;
    Some(AssemblyRelation {
        source: ida,
        target: idb,
        contact,
        orientation,
        source_region: region_of(a.size, fa_axis),
        target_region: region_of(b.size, fb_axis),
        gap_mm: if contact == ContactType::Gap { r(sep.distance) } else { 0.0 },
        penetration_mm: if contact == ContactType::Penetrate { r(-sep.distance) } else { 0.0 },
        contact_area_mm2: r(area),
        angle_deg: r(angle_deg),
        normal: [n.x, n.y, n.z],
        contact_region,
        source_face: (fa_axis, fa_side),
        target_face: (fb_axis, fb_side),
    })
}

/// Directed relation store with a derived reverse index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssemblyGraph {
    relations: Vec<AssemblyRelation>,
    #[serde(skip)]
    index: HashMap<ObjectId, Vec<usize>>,
}

impl AssemblyGraph {
    pub fn from_relations(mut relations: Vec<AssemblyRelation>) -> Self {
        relations.sort_by_key(|r| (r.source, r.target));
        let mut index: HashMap<ObjectId, Vec<usize>> = HashMap::new();
        for (i, r) in relations.iter().enumerate() {
            index.entry(r.source).or_default().push(i);
            index.entry(r.target).or_default().push(i);
        }
        Self { relations, index }
    }

    pub fn relations(&self) -> &[AssemblyRelation] {
        &self.relations
    }

    pub fn len(&self) -> usize {
        self.relations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    /// All relations of `id`, oriented so that `id` is the source.
    pub fn relations_of(&self, id: ObjectId) -> Vec<AssemblyRelation> {
        self.index
            .get(&id)
            .map(|v| {
                v.iter()
                    .map(|i| {
                        let r = &self.relations[*i];
                        if r.source == id { r.clone() } else { r.reversed() }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn between(&self, a: ObjectId, b: ObjectId) -> Option<AssemblyRelation> {
        let (s, t) = if a <= b { (a, b) } else { (b, a) };
        let r = self.relations.iter().find(|r| r.source == s && r.target == t)?;
        Some(if r.source == a { r.clone() } else { r.reversed() })
    }
}

/// Compute all relations among `items` (parallel narrow phase).
pub fn compute_relations(items: Vec<SpatialItem>, s: &RelationSettings) -> AssemblyGraph {
    let index = SpatialIndex::build(items);
    let pairs = index.candidate_pairs(s.max_gap_mm);
    let items = index.items();
    let rels: Vec<AssemblyRelation> = pairs
        .par_iter()
        .filter_map(|(i, j)| classify_pair((items[*i].id, &items[*i].obb), (items[*j].id, &items[*j].obb), s))
        .collect();
    AssemblyGraph::from_relations(rels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aic_math::Transform3D;

    fn side(x: f64) -> Obb {
        // A side panel: 560 deep, 720 high, 18 thick, thickness along world X.
        Obb::new([560.0, 720.0, 18.0], Transform3D::new([x, 0.0, 560.0], [0.0, 90.0, 0.0]))
    }

    fn bottom(x: f64, y: f64) -> Obb {
        // Horizontal panel 764 wide, 560 deep, thickness along Y.
        Obb::new([764.0, 560.0, 18.0], Transform3D::new([x, y, 560.0], [-90.0, 0.0, 0.0]))
    }

    #[test]
    fn touching_panels_is_touch() {
        let s = RelationSettings::default();
        let r = classify_pair((ObjectId(1), &side(0.0)), (ObjectId(2), &bottom(18.0, 0.0)), &s).unwrap();
        assert_eq!(r.contact, ContactType::Touch);
        assert_eq!(r.orientation, OrientationType::Perpendicular);
        assert_eq!(r.source_region, RegionType::Face);
        assert_eq!(r.target_region, RegionType::End);
        assert!((r.contact_area_mm2 - 560.0 * 18.0).abs() < 1e-6, "{}", r.contact_area_mm2);
    }

    #[test]
    fn two_mm_clearance_is_gap_2() {
        let s = RelationSettings::default();
        let r = classify_pair((ObjectId(1), &side(0.0)), (ObjectId(2), &bottom(20.0, 0.0)), &s).unwrap();
        assert_eq!(r.contact, ContactType::Gap);
        assert!((r.gap_mm - 2.0).abs() < 1e-9);
    }

    #[test]
    fn ten_mm_penetration_is_penetrate_10() {
        let s = RelationSettings::default();
        let r = classify_pair((ObjectId(1), &side(0.0)), (ObjectId(2), &bottom(8.0, 0.0)), &s).unwrap();
        assert_eq!(r.contact, ContactType::Penetrate);
        assert!((r.penetration_mm - 10.0).abs() < 1e-9);
    }

    #[test]
    fn stored_once_reverse_derived() {
        let items = vec![
            SpatialItem::new(ObjectId(2), bottom(18.0, 0.0)),
            SpatialItem::new(ObjectId(1), side(0.0)),
            SpatialItem::new(ObjectId(3), side(782.0)),
        ];
        let g = compute_relations(items, &RelationSettings::default());
        assert_eq!(g.len(), 2);
        assert!(g.relations().iter().all(|r| r.source < r.target));
        let of_bottom = g.relations_of(ObjectId(2));
        assert_eq!(of_bottom.len(), 2);
        assert!(of_bottom.iter().all(|r| r.source == ObjectId(2)));
        assert_eq!(g.between(ObjectId(3), ObjectId(2)).unwrap().source, ObjectId(3));
    }

    #[test]
    fn far_apart_is_none() {
        let s = RelationSettings::default();
        assert!(classify_pair((ObjectId(1), &side(0.0)), (ObjectId(2), &bottom(100.0, 0.0)), &s).is_none());
    }
}
