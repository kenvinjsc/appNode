//! Geometry layer. The domain never sees kernel types: everything goes through
//! the [`GeometryKernel`] trait. The default backend is [`CsgKernel`], a pure-Rust
//! BSP boundary-representation kernel for planar solids (exact for boxes/prisms,
//! faceted cylinders). An OpenCASCADE backend is planned behind the `occt` feature.

mod csg;
mod kernel;
mod mesh;
pub mod panel;

pub use csg::{CsgKernel, CsgFace, CsgShape};
pub use kernel::{GeometryError, GeometryKernel, Result};
pub use mesh::MeshData;
pub use panel::{build_hardware_mesh, build_panel_mesh, build_room_mesh, face_ids, PanelGeometryInput};
