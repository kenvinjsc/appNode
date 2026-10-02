//! Pure domain model of AIC CAD. No geometry kernel, no renderer, no UI.
//!
//! Source of truth = domain objects + parameters + relations + features.
//! BREP / meshes are derived caches built elsewhere.

pub mod cabinet;
pub mod error;
pub mod feature;
pub mod ids;
pub mod layout;
pub mod run;
pub mod structure;
pub mod material;
pub mod object;
pub mod panel;
pub mod product;
pub mod scene;
pub mod zone;

pub use cabinet::{CabinetKind, CabinetSpec, JoinStyle, MaterialSlot, PanelTemplate};
pub use layout::{build as build_cabinet, AnchorFace, PanelAnchor, ParamFeature, BayInfo, EdgeAnchor, CabinetValues, Fittings, Layout, Part, PartKind, PartMod, PartSplit, PanelPosition, ZoneBox};
pub use error::DomainError;
pub use feature::*;
pub use ids::{IdAllocator, MaterialId, NodeId, ObjectId};
pub use material::{default_materials, Material, MaterialKind};
pub use object::{band_thickness_of, edge_group, Anchor, Anchors, Cabinet, DomainObject, EdgeMode, EdgeRule, GroupEdge, Hardware, HardwareKind, ObjectKind, Room};
pub use panel::{GrainDirection, Panel, PanelRole};
pub use scene::{Scene, SceneNode};
