use nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};

use crate::Transform3D;

/// Axis-aligned bounding box in world space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    pub fn empty() -> Self {
        Self { min: [f64::INFINITY; 3], max: [f64::NEG_INFINITY; 3] }
    }

    pub fn is_empty(&self) -> bool {
        (0..3).any(|i| self.min[i] > self.max[i])
    }

    pub fn from_points<I: IntoIterator<Item = [f64; 3]>>(pts: I) -> Self {
        let mut b = Self::empty();
        for p in pts {
            b.expand_point(p);
        }
        b
    }

    pub fn expand_point(&mut self, p: [f64; 3]) {
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
    }

    pub fn union(&self, o: &Aabb) -> Aabb {
        let mut b = *self;
        if !o.is_empty() {
            b.expand_point(o.min);
            b.expand_point(o.max);
        }
        b
    }

    pub fn inflate(&self, d: f64) -> Aabb {
        Aabb { min: self.min.map(|v| v - d), max: self.max.map(|v| v + d) }
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        (0..3).all(|i| self.min[i] <= o.max[i] && o.min[i] <= self.max[i])
    }

    pub fn size(&self) -> [f64; 3] {
        [self.max[0] - self.min[0], self.max[1] - self.min[1], self.max[2] - self.min[2]]
    }

    pub fn center(&self) -> [f64; 3] {
        [
            0.5 * (self.min[0] + self.max[0]),
            0.5 * (self.min[1] + self.max[1]),
            0.5 * (self.min[2] + self.max[2]),
        ]
    }
}

/// Oriented box: a local box `[0,size]` placed by a rigid transform.
/// This is exactly how a panel is defined (definition space + placement).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    pub size: [f64; 3],
    pub transform: Transform3D,
}

impl Obb {
    pub fn new(size: [f64; 3], transform: Transform3D) -> Self {
        Self { size, transform }
    }

    pub fn corners(&self) -> [[f64; 3]; 8] {
        let [w, h, t] = self.size;
        let iso = self.transform.to_isometry();
        let mut out = [[0.0; 3]; 8];
        let mut k = 0;
        for &x in &[0.0, w] {
            for &y in &[0.0, h] {
                for &z in &[0.0, t] {
                    let p = iso * Point3::new(x, y, z);
                    out[k] = [p.x, p.y, p.z];
                    k += 1;
                }
            }
        }
        out
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::from_points(self.corners())
    }

    /// World-space unit axes (local X, Y, Z).
    pub fn axes(&self) -> [Vector3<f64>; 3] {
        let r = self.transform.rotation();
        [r * Vector3::x(), r * Vector3::y(), r * Vector3::z()]
    }

    pub fn center(&self) -> Point3<f64> {
        let [w, h, t] = self.size;
        self.transform.to_isometry() * Point3::new(w / 2.0, h / 2.0, t / 2.0)
    }

    pub fn half_extents(&self) -> [f64; 3] {
        self.size.map(|v| v / 2.0)
    }

    /// Projection interval of the box onto a unit axis.
    pub fn project(&self, axis: &Vector3<f64>) -> (f64, f64) {
        let c = self.center().coords.dot(axis);
        let ax = self.axes();
        let he = self.half_extents();
        let r: f64 = (0..3).map(|i| he[i] * ax[i].dot(axis).abs()).sum();
        (c - r, c + r)
    }

    pub fn volume(&self) -> f64 {
        self.size.iter().product()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_box_aabb() {
        let b = Obb::new([100.0, 10.0, 10.0], Transform3D::new([0.0; 3], [0.0, 0.0, 90.0]));
        let a = b.aabb();
        assert!((a.size()[0] - 10.0).abs() < 1e-9);
        assert!((a.size()[1] - 100.0).abs() < 1e-9);
    }
}
