use serde::{Deserialize, Serialize};

/// Render mesh handed to the UI. Flat-shaded triangles in object-local space.
/// `face_ids[i]` is the CAD face of triangle `i` (for face selection);
/// `edges` are line segments (pairs of points) with `edge_ids` per segment.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshData {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub edges: Vec<f32>,
    pub edge_ids: Vec<u32>,
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Append a convex planar polygon as a triangle fan.
    pub fn push_polygon(&mut self, pts: &[[f64; 3]], normal: [f64; 3], face_id: u32) {
        if pts.len() < 3 {
            return;
        }
        let base = self.vertex_count() as u32;
        for p in pts {
            self.positions.extend(p.iter().map(|v| *v as f32));
            self.normals.extend(normal.iter().map(|v| *v as f32));
        }
        for i in 1..pts.len() as u32 - 1 {
            self.indices.extend([base, base + i, base + i + 1]);
            self.face_ids.push(face_id);
        }
    }

    pub fn push_edge(&mut self, a: [f64; 3], b: [f64; 3], edge_id: u32) {
        self.edges.extend(a.iter().chain(b.iter()).map(|v| *v as f32));
        self.edge_ids.push(edge_id);
    }

    pub fn append(&mut self, other: &MeshData) {
        let base = self.vertex_count() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices.extend(other.indices.iter().map(|i| i + base));
        self.face_ids.extend_from_slice(&other.face_ids);
        self.edges.extend_from_slice(&other.edges);
        self.edge_ids.extend_from_slice(&other.edge_ids);
    }

    /// Signed volume via the divergence theorem (closed meshes only).
    pub fn volume(&self) -> f64 {
        let p = |i: u32| {
            let i = i as usize * 3;
            [self.positions[i] as f64, self.positions[i + 1] as f64, self.positions[i + 2] as f64]
        };
        self.indices
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
    }
}
