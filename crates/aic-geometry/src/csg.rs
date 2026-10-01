//! Pure-Rust BSP-tree constructive solid geometry (after Evan Wallace's csg.js).
//! Solids are closed sets of convex planar polygons; each polygon carries a
//! face id so selection survives boolean operations.

use crate::kernel::{GeometryError, GeometryKernel, Result};
use crate::MeshData;
use aic_math::{Polygon2D, Transform3D};

type V3 = [f64; 3];

const EPS: f64 = 1e-6;

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    if l == 0.0 {
        a
    } else {
        scale(a, 1.0 / l)
    }
}
fn lerp(a: V3, b: V3, t: f64) -> V3 {
    add(a, scale(sub(b, a), t))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Plane {
    n: V3,
    w: f64,
}

impl Plane {
    fn from_points(a: V3, b: V3, c: V3) -> Option<Plane> {
        let n = cross(sub(b, a), sub(c, a));
        if dot(n, n) < 1e-24 {
            return None;
        }
        let n = norm(n);
        Some(Plane { n, w: dot(n, a) })
    }

    fn flip(&mut self) {
        self.n = scale(self.n, -1.0);
        self.w = -self.w;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsgPolygon {
    pub vertices: Vec<V3>,
    plane: Plane,
    pub face_id: u32,
}

impl CsgPolygon {
    fn new(vertices: Vec<V3>, face_id: u32) -> Option<Self> {
        // Use Newell's method for a robust plane.
        let mut n = [0.0; 3];
        let k = vertices.len();
        for i in 0..k {
            let a = vertices[i];
            let b = vertices[(i + 1) % k];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        let plane = if dot(n, n) > 1e-24 {
            let n = norm(n);
            Plane { n, w: dot(n, vertices[0]) }
        } else {
            Plane::from_points(vertices[0], vertices[1], vertices[2])?
        };
        Some(Self { vertices, plane, face_id })
    }

    fn flip(&mut self) {
        self.vertices.reverse();
        self.plane.flip();
    }

    pub fn normal(&self) -> V3 {
        self.plane.n
    }
}

const COPLANAR: u8 = 0;
const FRONT: u8 = 1;
const BACK: u8 = 2;
const SPANNING: u8 = 3;

fn split_polygon(
    plane: &Plane,
    poly: &CsgPolygon,
    cop_front: &mut Vec<CsgPolygon>,
    cop_back: &mut Vec<CsgPolygon>,
    front: &mut Vec<CsgPolygon>,
    back: &mut Vec<CsgPolygon>,
) {
    let mut ptype = 0u8;
    let types: Vec<u8> = poly
        .vertices
        .iter()
        .map(|v| {
            let t = dot(plane.n, *v) - plane.w;
            let ty = if t < -EPS {
                BACK
            } else if t > EPS {
                FRONT
            } else {
                COPLANAR
            };
            ptype |= ty;
            ty
        })
        .collect();
    match ptype {
        COPLANAR => {
            if dot(plane.n, poly.plane.n) > 0.0 {
                cop_front.push(poly.clone())
            } else {
                cop_back.push(poly.clone())
            }
        }
        FRONT => front.push(poly.clone()),
        BACK => back.push(poly.clone()),
        _ => {
            let mut f = Vec::new();
            let mut b = Vec::new();
            let n = poly.vertices.len();
            for i in 0..n {
                let j = (i + 1) % n;
                let (ti, tj) = (types[i], types[j]);
                let (vi, vj) = (poly.vertices[i], poly.vertices[j]);
                if ti != BACK {
                    f.push(vi);
                }
                if ti != FRONT {
                    b.push(vi);
                }
                if (ti | tj) == SPANNING {
                    let t = (plane.w - dot(plane.n, vi)) / dot(plane.n, sub(vj, vi));
                    let v = lerp(vi, vj, t);
                    f.push(v);
                    b.push(v);
                }
            }
            if f.len() >= 3 {
                front.push(CsgPolygon { vertices: f, plane: poly.plane, face_id: poly.face_id });
            }
            if b.len() >= 3 {
                back.push(CsgPolygon { vertices: b, plane: poly.plane, face_id: poly.face_id });
            }
        }
    }
}

#[derive(Default)]
struct Node {
    plane: Option<Plane>,
    front: Option<Box<Node>>,
    back: Option<Box<Node>>,
    polygons: Vec<CsgPolygon>,
}

impl Node {
    fn new(polys: Vec<CsgPolygon>) -> Node {
        let mut n = Node::default();
        n.build(polys);
        n
    }

    fn invert(&mut self) {
        for p in &mut self.polygons {
            p.flip();
        }
        if let Some(pl) = &mut self.plane {
            pl.flip();
        }
        if let Some(f) = &mut self.front {
            f.invert();
        }
        if let Some(b) = &mut self.back {
            b.invert();
        }
        std::mem::swap(&mut self.front, &mut self.back);
    }

    fn clip_polygons(&self, polys: Vec<CsgPolygon>) -> Vec<CsgPolygon> {
        let Some(plane) = &self.plane else { return polys };
        let mut front = Vec::new();
        let mut back = Vec::new();
        for p in &polys {
            let (mut cf, mut cb) = (Vec::new(), Vec::new());
            split_polygon(plane, p, &mut cf, &mut cb, &mut front, &mut back);
            front.extend(cf);
            back.extend(cb);
        }
        let front = match &self.front {
            Some(f) => f.clip_polygons(front),
            None => front,
        };
        let back = match &self.back {
            Some(b) => b.clip_polygons(back),
            None => Vec::new(),
        };
        let mut out = front;
        out.extend(back);
        out
    }

    fn clip_to(&mut self, bsp: &Node) {
        self.polygons = bsp.clip_polygons(std::mem::take(&mut self.polygons));
        if let Some(f) = &mut self.front {
            f.clip_to(bsp);
        }
        if let Some(b) = &mut self.back {
            b.clip_to(bsp);
        }
    }

    fn all_polygons(&self) -> Vec<CsgPolygon> {
        let mut out = self.polygons.clone();
        if let Some(f) = &self.front {
            out.extend(f.all_polygons());
        }
        if let Some(b) = &self.back {
            out.extend(b.all_polygons());
        }
        out
    }

    fn build(&mut self, polys: Vec<CsgPolygon>) {
        if polys.is_empty() {
            return;
        }
        let plane = *self.plane.get_or_insert(polys[0].plane);
        let mut front = Vec::new();
        let mut back = Vec::new();
        let mut cf = Vec::new();
        let mut cb = Vec::new();
        for p in &polys {
            split_polygon(&plane, p, &mut cf, &mut cb, &mut front, &mut back);
        }
        self.polygons.extend(cf);
        self.polygons.extend(cb);
        if !front.is_empty() {
            self.front.get_or_insert_with(Default::default).build(front);
        }
        if !back.is_empty() {
            self.back.get_or_insert_with(Default::default).build(back);
        }
    }
}

/// A closed polyhedral solid.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CsgShape {
    pub polygons: Vec<CsgPolygon>,
}

/// Planar face (all polygons sharing a face id).
#[derive(Debug, Clone, PartialEq)]
pub struct CsgFace {
    pub face_id: u32,
    pub area: f64,
    pub normal: V3,
}

fn polygon_area(p: &CsgPolygon) -> f64 {
    let mut a = [0.0; 3];
    let n = p.vertices.len();
    for i in 0..n {
        a = add(a, cross(p.vertices[i], p.vertices[(i + 1) % n]));
    }
    0.5 * dot(a, p.plane.n).abs()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CsgKernel {
    /// Segments used to approximate circles.
    pub circle_segments: u32,
}

impl CsgKernel {
    pub fn new() -> Self {
        Self { circle_segments: 24 }
    }

    fn run_boolean(a: &CsgShape, b: &CsgShape, op: u8) -> Result<CsgShape> {
        // Run on a thread with a larger stack: BSP recursion depth grows with polygon count.
        let (a, b) = (a.polygons.clone(), b.polygons.clone());
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                let mut na = Node::new(a);
                let mut nb = Node::new(b);
                match op {
                    0 => {
                        // union
                        na.clip_to(&nb);
                        nb.clip_to(&na);
                        nb.invert();
                        nb.clip_to(&na);
                        nb.invert();
                        na.build(nb.all_polygons());
                        na.all_polygons()
                    }
                    1 => {
                        // subtract
                        na.invert();
                        na.clip_to(&nb);
                        nb.clip_to(&na);
                        nb.invert();
                        nb.clip_to(&na);
                        nb.invert();
                        na.build(nb.all_polygons());
                        na.invert();
                        na.all_polygons()
                    }
                    _ => {
                        // intersect
                        na.invert();
                        nb.clip_to(&na);
                        nb.invert();
                        na.clip_to(&nb);
                        nb.clip_to(&na);
                        na.build(nb.all_polygons());
                        na.invert();
                        na.all_polygons()
                    }
                }
            })
            .map_err(|e| GeometryError::BooleanFailed(e.to_string()))?;
        let polygons = handle.join().map_err(|_| GeometryError::BooleanFailed("kernel panic".into()))?;
        Ok(CsgShape { polygons })
    }
}

impl GeometryKernel for CsgKernel {
    type Shape = CsgShape;
    type Face = CsgFace;
    type Edge = (V3, V3);

    fn make_box(&self, width: f64, depth: f64, height: f64) -> Result<CsgShape> {
        if !(width > 0.0 && depth > 0.0 && height > 0.0 && (width + depth + height).is_finite()) {
            return Err(GeometryError::InvalidInput(format!("box {width} x {depth} x {height}")));
        }
        let (x, y, z) = (width, depth, height);
        // Face ids follow `panel::face_ids`: 0 -X, 1 +X, 2 -Y, 3 +Y, 4 -Z, 5 +Z.
        let faces: [(u32, [V3; 4]); 6] = [
            (0, [[0., 0., 0.], [0., 0., z], [0., y, z], [0., y, 0.]]),
            (1, [[x, 0., 0.], [x, y, 0.], [x, y, z], [x, 0., z]]),
            (2, [[0., 0., 0.], [x, 0., 0.], [x, 0., z], [0., 0., z]]),
            (3, [[0., y, 0.], [0., y, z], [x, y, z], [x, y, 0.]]),
            (4, [[0., 0., 0.], [0., y, 0.], [x, y, 0.], [x, 0., 0.]]),
            (5, [[0., 0., z], [x, 0., z], [x, y, z], [0., y, z]]),
        ];
        let polygons = faces.iter().filter_map(|(id, v)| CsgPolygon::new(v.to_vec(), *id)).collect();
        Ok(CsgShape { polygons })
    }

    fn make_cylinder(&self, radius: f64, height: f64) -> Result<CsgShape> {
        if !(radius > 0.0 && height > 0.0) {
            return Err(GeometryError::InvalidInput(format!("cylinder r={radius} h={height}")));
        }
        let n = self.circle_segments.max(6) as usize;
        let ring: Vec<[f64; 2]> = (0..n)
            .map(|i| {
                let a = i as f64 / n as f64 * std::f64::consts::TAU;
                [radius * a.cos(), radius * a.sin()]
            })
            .collect();
        let mut polys = Vec::new();
        let bottom: Vec<V3> = ring.iter().rev().map(|p| [p[0], p[1], 0.0]).collect();
        let top: Vec<V3> = ring.iter().map(|p| [p[0], p[1], height]).collect();
        polys.extend(CsgPolygon::new(bottom, 0));
        polys.extend(CsgPolygon::new(top, 0));
        for i in 0..n {
            let a = ring[i];
            let b = ring[(i + 1) % n];
            polys.extend(CsgPolygon::new(
                vec![[a[0], a[1], 0.0], [b[0], b[1], 0.0], [b[0], b[1], height], [a[0], a[1], height]],
                0,
            ));
        }
        Ok(CsgShape { polygons: polys })
    }

    fn extrude(&self, profile: &Polygon2D, height: f64) -> Result<CsgShape> {
        let prof = profile.clone().ensure_ccw();
        let n = prof.points.len();
        if n < 3 || height <= 0.0 {
            return Err(GeometryError::InvalidInput("extrude profile".into()));
        }
        // Caps by ear clipping (concave profiles such as notched or cut panels).
        let mut polys = Vec::new();
        let p = &prof.points;
        for [a, b, c] in prof.triangulate() {
            polys.extend(CsgPolygon::new(vec![[p[a].x, p[a].y, 0.0], [p[c].x, p[c].y, 0.0], [p[b].x, p[b].y, 0.0]], 0));
            polys.extend(CsgPolygon::new(vec![[p[a].x, p[a].y, height], [p[b].x, p[b].y, height], [p[c].x, p[c].y, height]], 0));
        }
        for i in 0..n {
            let a = p[i];
            let b = p[(i + 1) % n];
            polys.extend(CsgPolygon::new(vec![[a.x, a.y, 0.0], [b.x, b.y, 0.0], [b.x, b.y, height], [a.x, a.y, height]], 0));
        }
        Ok(CsgShape { polygons: polys })
    }

    fn transform(&self, shape: &CsgShape, t: &Transform3D) -> Result<CsgShape> {
        if !t.is_finite() {
            return Err(GeometryError::InvalidInput("transform".into()));
        }
        let polygons = shape
            .polygons
            .iter()
            .filter_map(|p| CsgPolygon::new(p.vertices.iter().map(|v| t.transform_point(*v)).collect(), p.face_id))
            .collect();
        Ok(CsgShape { polygons })
    }

    fn tag_faces(&self, shape: &CsgShape, face_id: u32) -> CsgShape {
        let mut s = shape.clone();
        for p in &mut s.polygons {
            p.face_id = face_id;
        }
        s
    }

    fn cut(&self, a: &CsgShape, b: &CsgShape) -> Result<CsgShape> {
        Self::run_boolean(a, b, 1)
    }

    fn fuse(&self, a: &CsgShape, b: &CsgShape) -> Result<CsgShape> {
        Self::run_boolean(a, b, 0)
    }

    fn intersect(&self, a: &CsgShape, b: &CsgShape) -> Result<CsgShape> {
        Self::run_boolean(a, b, 2)
    }

    fn volume(&self, shape: &CsgShape) -> f64 {
        shape
            .polygons
            .iter()
            .map(|p| {
                let v0 = p.vertices[0];
                (1..p.vertices.len() - 1)
                    .map(|i| dot(v0, cross(p.vertices[i], p.vertices[i + 1])) / 6.0)
                    .sum::<f64>()
            })
            .sum()
    }

    fn faces(&self, shape: &CsgShape) -> Vec<CsgFace> {
        let mut out: Vec<CsgFace> = Vec::new();
        for p in &shape.polygons {
            let a = polygon_area(p);
            match out.iter_mut().find(|f| f.face_id == p.face_id) {
                Some(f) => f.area += a,
                None => out.push(CsgFace { face_id: p.face_id, area: a, normal: p.plane.n }),
            }
        }
        out.sort_by_key(|f| f.face_id);
        out
    }

    fn tessellate(&self, shape: &CsgShape) -> Result<MeshData> {
        let mut m = MeshData::default();
        for p in &shape.polygons {
            m.push_polygon(&p.vertices, p.plane.n, p.face_id);
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_600x500x18_volume_is_exact() {
        let k = CsgKernel::new();
        let b = k.make_box(600.0, 500.0, 18.0).unwrap();
        assert_eq!(k.volume(&b), 600.0 * 500.0 * 18.0);
        let m = k.tessellate(&b).unwrap();
        assert!((m.volume() - 5_400_000.0).abs() < 1.0);
        assert_eq!(m.triangle_count(), 12);
    }

    #[test]
    fn booleans_on_boxes() {
        let k = CsgKernel::new();
        let a = k.make_box(100.0, 100.0, 100.0).unwrap();
        let b = k.transform(&k.make_box(100.0, 100.0, 100.0).unwrap(), &Transform3D::from_translation(50.0, 0.0, 0.0)).unwrap();
        let cut = k.cut(&a, &b).unwrap();
        assert!((k.volume(&cut) - 500_000.0).abs() < 1e-3);
        let fused = k.fuse(&a, &b).unwrap();
        assert!((k.volume(&fused) - 1_500_000.0).abs() < 1e-3);
        let common = k.intersect(&a, &b).unwrap();
        assert!((k.volume(&common) - 500_000.0).abs() < 1e-3);
    }

    #[test]
    fn pocket_cut_keeps_face_ids() {
        let k = CsgKernel::new();
        let a = k.make_box(200.0, 200.0, 18.0).unwrap();
        let tool = k.tag_faces(
            &k.transform(&k.make_box(50.0, 50.0, 10.0).unwrap(), &Transform3D::from_translation(10.0, 10.0, 8.0 + 1e-3)).unwrap(),
            1000,
        );
        let cut = k.cut(&a, &tool).unwrap();
        let expected = 200.0 * 200.0 * 18.0 - 50.0 * 50.0 * (10.0 - 1e-3);
        assert!((k.volume(&cut) - expected).abs() < 1e-2, "{}", k.volume(&cut));
        assert!(k.faces(&cut).iter().any(|f| f.face_id == 1000));
    }
}
