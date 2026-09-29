use nalgebra::{Isometry3, Matrix4, Point3, Translation3, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};

/// Rigid transform (no scale). Panel sizes live in the domain, never in the transform,
/// so dimensions can never be polluted by non-uniform scale.
///
/// Rotation is stored as intrinsic XYZ Euler angles in degrees for stable
/// serialisation and editing; it is converted to a quaternion for composition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    pub translation: [f64; 3],
    pub rotation_deg: [f64; 3],
}

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform3D {
    pub const IDENTITY: Self = Self { translation: [0.0; 3], rotation_deg: [0.0; 3] };

    pub fn from_translation(x: f64, y: f64, z: f64) -> Self {
        Self { translation: [x, y, z], rotation_deg: [0.0; 3] }
    }

    pub fn new(translation: [f64; 3], rotation_deg: [f64; 3]) -> Self {
        Self { translation, rotation_deg }
    }

    pub fn rotation(&self) -> UnitQuaternion<f64> {
        let [rx, ry, rz] = self.rotation_deg.map(f64::to_radians);
        // Intrinsic X then Y then Z == extrinsic Z*Y*X applied to column vectors.
        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), rx)
            * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), ry)
            * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), rz)
    }

    pub fn to_isometry(&self) -> Isometry3<f64> {
        let [x, y, z] = self.translation;
        Isometry3::from_parts(Translation3::new(x, y, z), self.rotation())
    }

    pub fn from_isometry(iso: &Isometry3<f64>) -> Self {
        let t = iso.translation.vector;
        // Decompose R = Rx * Ry * Rz.
        let m = iso.rotation.to_rotation_matrix();
        let m = m.matrix();
        let sy = m[(0, 2)].clamp(-1.0, 1.0);
        let ry = sy.asin();
        let (rx, rz) = if sy.abs() < 1.0 - 1e-12 {
            ((-m[(1, 2)]).atan2(m[(2, 2)]), (-m[(0, 1)]).atan2(m[(0, 0)]))
        } else {
            // Gimbal lock: fold everything into X.
            (m[(2, 1)].atan2(m[(1, 1)]), 0.0)
        };
        let clean = |v: f64| {
            let d = v.to_degrees();
            let r = (d * 1e9).round() / 1e9;
            if r == -0.0 { 0.0 } else { r }
        };
        let cl = |v: f64| {
            let r = (v * 1e9).round() / 1e9;
            if r == -0.0 { 0.0 } else { r }
        };
        Self { translation: [cl(t.x), cl(t.y), cl(t.z)], rotation_deg: [clean(rx), clean(ry), clean(rz)] }
    }

    /// `self * other`: apply `other` first, then `self` (parent * local).
    pub fn compose(&self, other: &Transform3D) -> Transform3D {
        Transform3D::from_isometry(&(self.to_isometry() * other.to_isometry()))
    }

    pub fn inverse(&self) -> Transform3D {
        Transform3D::from_isometry(&self.to_isometry().inverse())
    }

    pub fn transform_point(&self, p: [f64; 3]) -> [f64; 3] {
        let q = self.to_isometry() * Point3::new(p[0], p[1], p[2]);
        [q.x, q.y, q.z]
    }

    pub fn transform_vector(&self, v: [f64; 3]) -> [f64; 3] {
        let q = self.rotation() * Vector3::new(v[0], v[1], v[2]);
        [q.x, q.y, q.z]
    }

    /// Column-major 4x4 matrix (the layout Three.js `Matrix4.fromArray` expects).
    pub fn to_matrix_col_major(&self) -> [f64; 16] {
        let m: Matrix4<f64> = self.to_isometry().to_homogeneous();
        let mut out = [0.0; 16];
        out.copy_from_slice(m.as_slice());
        out
    }

    pub fn is_finite(&self) -> bool {
        self.translation.iter().chain(self.rotation_deg.iter()).all(|v| v.is_finite())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_isometry() {
        let t = Transform3D::new([10.0, 20.0, -5.0], [10.0, 25.0, -40.0]);
        let back = Transform3D::from_isometry(&t.to_isometry());
        for i in 0..3 {
            assert!((t.translation[i] - back.translation[i]).abs() < 1e-9);
            assert!((t.rotation_deg[i] - back.rotation_deg[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn compose_translation_and_rotation() {
        let parent = Transform3D::new([100.0, 0.0, 0.0], [0.0, 0.0, 90.0]);
        let child = Transform3D::from_translation(10.0, 0.0, 0.0);
        let world = parent.compose(&child);
        assert!((world.translation[0] - 100.0).abs() < 1e-9);
        assert!((world.translation[1] - 10.0).abs() < 1e-9);
        assert!((world.rotation_deg[2] - 90.0).abs() < 1e-9);
    }

    #[test]
    fn inverse_cancels() {
        let t = Transform3D::new([1.0, 2.0, 3.0], [30.0, 0.0, 15.0]);
        let id = t.compose(&t.inverse());
        for v in id.translation.iter().chain(id.rotation_deg.iter()) {
            assert!(v.abs() < 1e-6);
        }
    }
}
