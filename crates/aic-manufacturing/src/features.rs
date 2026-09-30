use aic_assembly::{AssemblyRelation, ContactType, OrientationType, RegionType};
use aic_domain::{DrillFeature, DrillPurpose, EdgeDrillFeature, EdgeSide, FaceSide, MachiningFeature, ObjectId, Panel, PanelRole};
use aic_math::Transform3D;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureOrigin {
    /// Stored on the panel by the user.
    User { index: usize },
    /// Derived from the panel's role (e.g. hinge cups on doors).
    Rule,
    /// Derived from an assembly contact with another panel.
    Joint { with: ObjectId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedFeature {
    pub feature: MachiningFeature,
    pub origin: FeatureOrigin,
}

#[derive(Debug, Clone, Copy)]
pub struct PanelPlacement<'a> {
    pub panel: &'a Panel,
    pub world: Transform3D,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JointSettings {
    pub dowel_diameter: f64,
    pub dowel_face_depth: f64,
    pub dowel_edge_depth: f64,
    pub shelf_pin_diameter: f64,
    pub shelf_pin_depth: f64,
    pub end_inset: f64,
    pub max_pitch: f64,
}

impl Default for JointSettings {
    fn default() -> Self {
        Self {
            dowel_diameter: 8.0,
            dowel_face_depth: 12.0,
            dowel_edge_depth: 25.0,
            shelf_pin_diameter: 5.0,
            shelf_pin_depth: 10.0,
            end_inset: 50.0,
            max_pitch: 300.0,
        }
    }
}

/// Role-based features that do not depend on neighbours.
/// `hinge_left`: for doors, whether hinges are on the left edge.
pub fn rule_features(panel: &Panel, hinge_left: Option<bool>) -> Vec<MachiningFeature> {
    // Parts built by the cabinet generator carry their own rule machining.
    if panel.gen_key.is_some() {
        return panel.gen_features.clone();
    }
    let mut out = Vec::new();
    if panel.role == PanelRole::Door {
        // Same hinge table as the cabinet generator (chuẩn xưởng mặc định).
        let sr = aic_domain::structure::ShopRules::default();
        let left = hinge_left.unwrap_or(true);
        let x = if left { sr.hinge_edge } else { panel.width_mm - sr.hinge_edge };
        let h = panel.height_mm;
        for y in sr.hinge_positions(h) {
            if y > 0.0 && y < h {
                out.push(MachiningFeature::Drill(DrillFeature {
                    x,
                    y,
                    diameter: sr.cup_d,
                    depth: sr.cup_depth,
                    side: FaceSide::B,
                    purpose: DrillPurpose::HingeCup,
                }));
            }
        }
    }
    out
}

fn edge_of(face: (usize, i8)) -> Option<EdgeSide> {
    match face {
        (0, s) if s < 0 => Some(EdgeSide::Left),
        (0, _) => Some(EdgeSide::Right),
        (1, s) if s < 0 => Some(EdgeSide::Bottom),
        (1, _) => Some(EdgeSide::Top),
        _ => None,
    }
}

fn joinable(role: PanelRole) -> bool {
    !matches!(
        role,
        PanelRole::Door | PanelRole::DrawerFront | PanelRole::Back | PanelRole::BackSub | PanelRole::DrawerBottom
    )
}

/// Positions along a joint of length `len` (local coordinate `0..len`).
fn joint_positions(len: f64, s: &JointSettings) -> Vec<f64> {
    if len < 40.0 {
        return vec![len / 2.0];
    }
    let inset = s.end_inset.min(len / 4.0);
    let span = len - 2.0 * inset;
    let n = ((span / s.max_pitch).ceil() as usize + 1).clamp(2, 8);
    (0..n).map(|i| inset + span * i as f64 / (n - 1) as f64).collect()
}

/// Dowel / shelf-pin features from FACE↔EDGE/END perpendicular touches.
pub fn derive_joint_features(
    panels: &HashMap<ObjectId, PanelPlacement>,
    relations: &[AssemblyRelation],
    s: &JointSettings,
) -> HashMap<ObjectId, Vec<DerivedFeature>> {
    let mut out: HashMap<ObjectId, Vec<DerivedFeature>> = HashMap::new();
    for rel in relations {
        if rel.contact != ContactType::Touch || rel.orientation != OrientationType::Perpendicular {
            continue;
        }
        // Normalise so that `face` is the panel contributing its big face.
        let (face_id, edge_id, face_face, edge_face) = match (rel.source_region, rel.target_region) {
            (RegionType::Face, RegionType::Edge | RegionType::End) => (rel.source, rel.target, rel.source_face, rel.target_face),
            (RegionType::Edge | RegionType::End, RegionType::Face) => (rel.target, rel.source, rel.target_face, rel.source_face),
            _ => continue,
        };
        let (Some(fp), Some(ep)) = (panels.get(&face_id), panels.get(&edge_id)) else { continue };
        if !joinable(fp.panel.role) || !joinable(ep.panel.role) {
            continue;
        }
        let Some(edge_side) = edge_of(edge_face) else { continue };
        let face_side = if face_face.1 > 0 { FaceSide::A } else { FaceSide::B };

        // Contact region in the face panel's local frame.
        let inv_f = fp.world.inverse();
        let local: Vec<[f64; 3]> = rel.contact_region.iter().map(|p| inv_f.transform_point(*p)).collect();
        if local.len() < 3 {
            continue;
        }
        let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for p in &local {
            minx = minx.min(p[0]);
            maxx = maxx.max(p[0]);
            miny = miny.min(p[1]);
            maxy = maxy.max(p[1]);
        }
        let along_x = (maxx - minx) >= (maxy - miny);
        let len = if along_x { maxx - minx } else { maxy - miny };
        let (cx, cy) = ((minx + maxx) / 2.0, (miny + maxy) / 2.0);
        let is_shelf = ep.panel.role == PanelRole::Shelf;
        let inv_e = ep.world.inverse();

        for t in joint_positions(len, s) {
            let (x, y) = if along_x { (minx + t, cy) } else { (cx, miny + t) };
            if is_shelf {
                // Shelf pins sit just below the shelf on the side panel.
                let below = -(s.shelf_pin_diameter);
                let (px, py) = if along_x { (x, miny + below) } else { (minx + below, y) };
                out.entry(face_id).or_default().push(DerivedFeature {
                    feature: MachiningFeature::Drill(DrillFeature {
                        x: px.clamp(0.0, fp.panel.width_mm),
                        y: py.clamp(0.0, fp.panel.height_mm),
                        diameter: s.shelf_pin_diameter,
                        depth: s.shelf_pin_depth,
                        side: face_side,
                        purpose: DrillPurpose::ShelfPin,
                    }),
                    origin: FeatureOrigin::Joint { with: edge_id },
                });
                continue;
            }
            out.entry(face_id).or_default().push(DerivedFeature {
                feature: MachiningFeature::Drill(DrillFeature {
                    x,
                    y,
                    diameter: s.dowel_diameter,
                    depth: s.dowel_face_depth.min(fp.panel.thickness_mm - 3.0),
                    side: face_side,
                    purpose: DrillPurpose::Dowel,
                }),
                origin: FeatureOrigin::Joint { with: edge_id },
            });
            // Same world point, in the edge panel's frame.
            let z_face = if face_side == FaceSide::A { fp.panel.thickness_mm } else { 0.0 };
            let world = fp.world.transform_point([x, y, z_face]);
            let le = inv_e.transform_point(world);
            let offset = match edge_side {
                EdgeSide::Left | EdgeSide::Right => le[1],
                EdgeSide::Top | EdgeSide::Bottom => le[0],
            };
            out.entry(edge_id).or_default().push(DerivedFeature {
                feature: MachiningFeature::EdgeDrill(EdgeDrillFeature {
                    edge: edge_side,
                    offset,
                    z: le[2].clamp(0.0, ep.panel.thickness_mm),
                    diameter: s.dowel_diameter,
                    depth: s.dowel_edge_depth,
                    purpose: DrillPurpose::Dowel,
                }),
                origin: FeatureOrigin::Joint { with: face_id },
            });
        }
    }
    out
}
