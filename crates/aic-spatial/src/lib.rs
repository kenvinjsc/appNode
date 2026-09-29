//! Spatial queries: broad phase (sort-and-sweep over AABBs), exact box-box
//! separation (SAT) and snapping.

mod broad;
mod sat;
mod snap;

pub use broad::{SpatialIndex, SpatialItem};
pub use sat::{obb_separation, Separation};
pub use snap::{snap_translation, SnapHint, SnapKind, SnapResult, SnapSettings};
