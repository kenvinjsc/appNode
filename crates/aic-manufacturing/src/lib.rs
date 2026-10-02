//! Manufacturing: feature derivation (rules + assembly joints), 3D → 2D
//! projection of panels into their local manufacturing frame, 2D polygon
//! operations (Clipper2) and CNC toolpath / G-code generation.

pub mod cnc;
pub mod export;
pub mod features;
pub mod flatten;
pub mod polygon_ops;

pub use cnc::{default_tools, generate_program, CncProgram, Move, Operation, SheetPart, Tool, ToolKind};
pub use features::{count_brackets, derive_joint_features, rule_features, DerivedFeature, FeatureOrigin, JointSettings, PanelPlacement};
pub use flatten::{flatten, FlatPanel, FeatureSummary};
