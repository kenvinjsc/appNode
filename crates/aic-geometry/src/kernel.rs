use crate::MeshData;
use aic_math::{Polygon2D, Transform3D};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GeometryError {
    #[error("boolean operation failed: {0}")]
    BooleanFailed(String),
    #[error("invalid geometry input: {0}")]
    InvalidInput(String),
    #[error("tessellation failed: {0}")]
    Tessellation(String),
}

pub type Result<T> = std::result::Result<T, GeometryError>;

/// Abstraction over a solid-modelling kernel (OpenCASCADE, pure Rust CSG, ...).
pub trait GeometryKernel {
    type Shape: Clone;
    type Face;
    type Edge;

    /// Axis-aligned box `[0,width] x [0,depth] x [0,height]` (X, Y, Z).
    fn make_box(&self, width: f64, depth: f64, height: f64) -> Result<Self::Shape>;
    /// Cylinder along +Z with its base centred on the origin.
    fn make_cylinder(&self, radius: f64, height: f64) -> Result<Self::Shape>;
    /// Extrude a planar polygon (XY) along +Z.
    fn extrude(&self, profile: &Polygon2D, height: f64) -> Result<Self::Shape>;
    fn transform(&self, shape: &Self::Shape, t: &Transform3D) -> Result<Self::Shape>;
    /// Tag every face of the shape with a stable face id (used for selection).
    fn tag_faces(&self, shape: &Self::Shape, face_id: u32) -> Self::Shape;

    fn cut(&self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape>;
    fn fuse(&self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape>;
    fn intersect(&self, a: &Self::Shape, b: &Self::Shape) -> Result<Self::Shape>;

    fn volume(&self, shape: &Self::Shape) -> f64;
    fn faces(&self, shape: &Self::Shape) -> Vec<Self::Face>;
    fn tessellate(&self, shape: &Self::Shape) -> Result<MeshData>;
}
