//! Math primitives for AIC CAD: transforms, bounding volumes and 2D polygons.
//!
//! All lengths are millimetres, all angles in the public API are degrees.

mod bbox;
mod polygon;
mod transform;

pub use bbox::{Aabb, Obb};
pub use nalgebra::{Isometry3, Matrix4, Point3, Rotation3, Translation3, UnitQuaternion, Vector2, Vector3};
pub use polygon::{Point2, Polygon2D};
pub use transform::Transform3D;

/// Default linear tolerance (mm) used for contact classification and comparisons.
pub const LINEAR_TOL: f64 = 1e-3;
/// Default angular tolerance (degrees).
pub const ANGULAR_TOL_DEG: f64 = 0.01;

pub fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}
