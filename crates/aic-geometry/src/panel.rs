//! Builders turning domain definitions into solids / render meshes.
//! Always built in the object's *definition space*; placement is applied by the
//! renderer through the world matrix, so a move never re-tessellates.

use crate::kernel::{GeometryKernel, Result};
use crate::MeshData;
use aic_domain::{Axis2, EdgeSide, FaceSide, HardwareKind, MachiningFeature, Room};
use aic_math::Transform3D;

/// Stable face ids of a panel's box faces.
pub mod face_ids {
    pub const LEFT: u32 = 0; // x = 0
    pub const RIGHT: u32 = 1; // x = width
    pub const BOTTOM: u32 = 2; // y = 0
    pub const TOP: u32 = 3; // y = height
    pub const SIDE_B: u32 = 4; // z = 0
    pub const SIDE_A: u32 = 5; // z = thickness
    /// Faces created by machining feature `i` get `FEATURE_BASE + i`.
    pub const FEATURE_BASE: u32 = 1000;

    pub fn name(id: u32) -> String {
        match id {
            LEFT => "EDGE_LEFT".into(),
            RIGHT => "EDGE_RIGHT".into(),
            BOTTOM => "EDGE_BOTTOM".into(),
            TOP => "EDGE_TOP".into(),
            SIDE_B => "FACE_B".into(),
            SIDE_A => "FACE_A".into(),
            n => format!("FEATURE_{}", n - FEATURE_BASE),
        }
    }
}

pub struct PanelGeometryInput<'a> {
    pub size: [f64; 3],
    pub features: &'a [MachiningFeature],
}

const OVERCUT: f64 = 0.05;

fn feature_tool<K: GeometryKernel>(k: &K, f: &MachiningFeature, size: [f64; 3]) -> Result<Option<K::Shape>> {
    let [w, h, t] = size;
    let z_for = |side: FaceSide, depth: f64| -> (f64, f64) {
        let d = depth.min(t + 2.0 * OVERCUT);
        match side {
            FaceSide::A => (t - d, d + OVERCUT),
            FaceSide::B => (-OVERCUT, d + OVERCUT),
        }
    };
    Ok(match f {
        MachiningFeature::Drill(d) => {
            let (z0, len) = z_for(d.side, d.depth);
            let c = k.make_cylinder(d.diameter / 2.0, len)?;
            Some(k.transform(&c, &Transform3D::from_translation(d.x, d.y, z0))?)
        }
        MachiningFeature::EdgeDrill(d) => {
            let c = k.make_cylinder(d.diameter / 2.0, d.depth + OVERCUT)?;
            // Cylinder axis is +Z; orient it into the panel from the edge.
            let t = match d.edge {
                EdgeSide::Left => Transform3D::new([-OVERCUT, d.offset, d.z], [0.0, 90.0, 0.0]),
                EdgeSide::Right => Transform3D::new([w + OVERCUT, d.offset, d.z], [0.0, -90.0, 0.0]),
                EdgeSide::Bottom => Transform3D::new([d.offset, -OVERCUT, d.z], [-90.0, 0.0, 0.0]),
                EdgeSide::Top => Transform3D::new([d.offset, h + OVERCUT, d.z], [90.0, 0.0, 0.0]),
            };
            Some(k.transform(&c, &t)?)
        }
        MachiningFeature::Pocket(p) => {
            let (z0, len) = z_for(p.side, p.depth);
            let b = k.make_box(p.width, p.height, len)?;
            Some(k.transform(&b, &Transform3D::from_translation(p.x, p.y, z0))?)
        }
        MachiningFeature::Groove(g) => {
            let (z0, len) = z_for(g.side, g.depth);
            let (bw, bh) = match g.direction {
                Axis2::X => (g.length, g.width),
                Axis2::Y => (g.width, g.length),
            };
            let b = k.make_box(bw, bh, len)?;
            Some(k.transform(&b, &Transform3D::from_translation(g.x, g.y, z0))?)
        }
        MachiningFeature::Contour(c) if c.inner => {
            let e = k.extrude(&c.polygon, t + 2.0 * OVERCUT)?;
            Some(k.transform(&e, &Transform3D::from_translation(0.0, 0.0, -OVERCUT))?)
        }
        MachiningFeature::Contour(_) => None,
    })
}

fn box_edges(m: &mut MeshData, size: [f64; 3]) {
    let [x, y, z] = size;
    let c = |i: u8| [if i & 1 != 0 { x } else { 0.0 }, if i & 2 != 0 { y } else { 0.0 }, if i & 4 != 0 { z } else { 0.0 }];
    let pairs = [(0, 1), (2, 3), (4, 5), (6, 7), (0, 2), (1, 3), (4, 6), (5, 7), (0, 4), (1, 5), (2, 6), (3, 7)];
    for (i, (a, b)) in pairs.iter().enumerate() {
        m.push_edge(c(*a), c(*b), i as u32);
    }
}

fn circle_edges(m: &mut MeshData, cx: f64, cy: f64, z: f64, r: f64, id: u32) {
    let n = 20;
    for i in 0..n {
        let a0 = i as f64 / n as f64 * std::f64::consts::TAU;
        let a1 = (i + 1) as f64 / n as f64 * std::f64::consts::TAU;
        m.push_edge([cx + r * a0.cos(), cy + r * a0.sin(), z], [cx + r * a1.cos(), cy + r * a1.sin(), z], id);
    }
}

fn rect_edges(m: &mut MeshData, x: f64, y: f64, w: f64, h: f64, z: f64, id: u32) {
    let p = [[x, y, z], [x + w, y, z], [x + w, y + h, z], [x, y + h, z]];
    for i in 0..4 {
        m.push_edge(p[i], p[(i + 1) % 4], id);
    }
}

/// Build a panel solid (box minus machining features) and tessellate it.
pub fn build_panel_mesh<K: GeometryKernel>(k: &K, input: &PanelGeometryInput) -> Result<MeshData> {
    let [w, h, t] = input.size;
    let mut solid = k.make_box(w, h, t)?;
    for (i, f) in input.features.iter().enumerate() {
        if let Some(tool) = feature_tool(k, f, input.size)? {
            let tool = k.tag_faces(&tool, face_ids::FEATURE_BASE + i as u32);
            solid = k.cut(&solid, &tool)?;
        }
    }
    let mut mesh = k.tessellate(&solid)?;
    box_edges(&mut mesh, input.size);
    for (i, f) in input.features.iter().enumerate() {
        let id = face_ids::FEATURE_BASE + i as u32;
        let zf = |s: FaceSide| if s == FaceSide::A { t } else { 0.0 };
        match f {
            MachiningFeature::Drill(d) => circle_edges(&mut mesh, d.x, d.y, zf(d.side), d.diameter / 2.0, id),
            MachiningFeature::Pocket(p) => rect_edges(&mut mesh, p.x, p.y, p.width, p.height, zf(p.side), id),
            MachiningFeature::Groove(g) => {
                let (bw, bh) = match g.direction {
                    Axis2::X => (g.length, g.width),
                    Axis2::Y => (g.width, g.length),
                };
                rect_edges(&mut mesh, g.x, g.y, bw, bh, zf(g.side), id)
            }
            _ => {}
        }
    }
    Ok(mesh)
}

pub fn build_hardware_mesh<K: GeometryKernel>(k: &K, kind: HardwareKind, size: [f64; 3]) -> Result<MeshData> {
    let [x, y, z] = size;
    let solid = match kind {
        HardwareKind::Rail => {
            let r = y.min(z) / 2.0;
            let c = k.make_cylinder(r, x.max(1.0))?;
            k.transform(&c, &Transform3D::new([0.0, r, r], [0.0, 90.0, 0.0]))?
        }
        HardwareKind::Leg => {
            let r = x.min(z) / 2.0;
            let c = k.make_cylinder(r, y.max(1.0))?;
            k.transform(&c, &Transform3D::new([r, 0.0, r], [-90.0, 0.0, 0.0]))?
        }
        HardwareKind::Handle | HardwareKind::Hinge | HardwareKind::Slide => k.make_box(x, y, z)?,
    };
    k.tessellate(&solid)
}

pub fn build_room_mesh<K: GeometryKernel>(k: &K, room: &Room) -> Result<MeshData> {
    let (w, d, h, t) = (room.width_mm, room.depth_mm, room.height_mm, room.wall_thickness_mm.max(1.0));
    let mut mesh = MeshData::default();
    let parts = [
        // floor
        ([w + t, 20.0, d + t], [-t, -20.0, -t], 0u32),
        // back wall
        ([w + t, h, t], [-t, 0.0, -t], 1),
        // left wall
        ([t, h, d], [-t, 0.0, 0.0], 2),
    ];
    for (size, pos, id) in parts {
        let b = k.make_box(size[0], size[1], size[2])?;
        let b = k.tag_faces(&k.transform(&b, &Transform3D::from_translation(pos[0], pos[1], pos[2]))?, id);
        mesh.append(&k.tessellate(&b)?);
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CsgKernel;
    use aic_domain::{DrillFeature, DrillPurpose, PocketFeature};

    #[test]
    fn panel_with_drill_and_pocket() {
        let k = CsgKernel::new();
        let features = vec![
            MachiningFeature::Drill(DrillFeature { x: 50.0, y: 50.0, diameter: 8.0, depth: 12.0, side: FaceSide::A, purpose: DrillPurpose::Dowel }),
            MachiningFeature::Pocket(PocketFeature { x: 100.0, y: 100.0, width: 60.0, height: 40.0, depth: 8.0, side: FaceSide::B, corner_radius: 0.0 }),
        ];
        let m = build_panel_mesh(&k, &PanelGeometryInput { size: [600.0, 500.0, 18.0], features: &features }).unwrap();
        let full = 600.0 * 500.0 * 18.0;
        let pocket = 60.0 * 40.0 * 8.0;
        let v = m.volume();
        assert!(v < full - pocket + 1.0 && v > full - pocket - 1000.0, "volume {v}");
        assert!(m.face_ids.contains(&face_ids::FEATURE_BASE));
        assert!(m.face_ids.contains(&(face_ids::FEATURE_BASE + 1)));
        assert_eq!(m.edge_ids.iter().filter(|e| **e < 12).count(), 12);
    }
}
