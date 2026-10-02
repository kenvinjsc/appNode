//! Parametric engine: safe expression language + dependency graph with cycle
//! detection, dirty propagation and incremental recompute. Units: millimetres.
//!
//! Expressions never go through any kind of string `eval`; they are tokenised
//! and parsed into an AST that can only reference other parameters and a fixed
//! set of pure functions (`min`, `max`, `clamp`, `if`, `abs`, `round`, `floor`, `ceil`, `sqrt`).

mod expr;
mod graph;

pub use expr::{parse, BinOp, Expr, ParseError};
pub use graph::{Constraint, ParamError, ParamGraph, ParamKey, ParamState};
