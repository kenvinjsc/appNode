//! Machining features. All coordinates are in the panel's *local* manufacturing
//! frame (X along width, Y along height, Z through thickness). Never world space.

use aic_math::Polygon2D;
use serde::{Deserialize, Serialize};

/// Which big face of the panel a feature is machined from.
/// `A` is the face at local Z = thickness, `B` the face at Z = 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaceSide {
    A,
    B,
}

/// Narrow edges of a panel. Left: x=0, Right: x=width, Bottom: y=0, Top: y=height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeSide {
    Left,
    Right,
    Bottom,
    Top,
}

impl EdgeSide {
    pub const ALL: [EdgeSide; 4] = [EdgeSide::Left, EdgeSide::Right, EdgeSide::Bottom, EdgeSide::Top];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DrillPurpose {
    #[default]
    Generic,
    ShelfPin,
    Dowel,
    CamLock,
    Connector,
    HingeCup,
    HingeScrew,
    Handle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrillFeature {
    pub x: f64,
    pub y: f64,
    pub diameter: f64,
    /// Depth into the panel from `side`. `>= thickness` means through-hole.
    pub depth: f64,
    pub side: FaceSide,
    #[serde(default)]
    pub purpose: DrillPurpose,
}

/// Horizontal drill into a narrow edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeDrillFeature {
    pub edge: EdgeSide,
    /// Position along the edge (X for Top/Bottom, Y for Left/Right).
    pub offset: f64,
    /// Position in thickness direction (usually thickness / 2).
    pub z: f64,
    pub diameter: f64,
    pub depth: f64,
    #[serde(default)]
    pub purpose: DrillPurpose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PocketFeature {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub side: FaceSide,
    #[serde(default)]
    pub corner_radius: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Axis2 {
    X,
    Y,
}

/// Straight groove (e.g. for a back panel). Starts at (x, y) and runs `length`
/// along `direction`; `width` is measured across the groove.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrooveFeature {
    pub x: f64,
    pub y: f64,
    pub length: f64,
    pub width: f64,
    pub depth: f64,
    pub direction: Axis2,
    pub side: FaceSide,
}

/// Custom outer/inner contour cut, local XY.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContourFeature {
    pub polygon: Polygon2D,
    /// true = through cut of an inner opening, false = outer shape override.
    pub inner: bool,
    pub depth: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MachiningFeature {
    Drill(DrillFeature),
    EdgeDrill(EdgeDrillFeature),
    Pocket(PocketFeature),
    Groove(GrooveFeature),
    Contour(ContourFeature),
}

impl MachiningFeature {
    pub fn kind_name(&self) -> &'static str {
        match self {
            MachiningFeature::Drill(_) => "DRILL",
            MachiningFeature::EdgeDrill(_) => "EDGE_DRILL",
            MachiningFeature::Pocket(_) => "POCKET",
            MachiningFeature::Groove(_) => "GROOVE",
            MachiningFeature::Contour(_) => "CONTOUR",
        }
    }

    /// Validate that the feature lies inside a panel of the given local size.
    pub fn validate(&self, w: f64, h: f64, t: f64) -> Result<(), String> {
        let pos = |v: f64, name: &str| if v > 0.0 && v.is_finite() { Ok(()) } else { Err(format!("{name} must be > 0")) };
        match self {
            MachiningFeature::Drill(d) => {
                pos(d.diameter, "diameter")?;
                pos(d.depth, "depth")?;
                if d.x < 0.0 || d.y < 0.0 || d.x > w || d.y > h {
                    return Err("drill outside panel".into());
                }
            }
            MachiningFeature::EdgeDrill(d) => {
                pos(d.diameter, "diameter")?;
                pos(d.depth, "depth")?;
                if d.z < 0.0 || d.z > t {
                    return Err("edge drill outside thickness".into());
                }
            }
            MachiningFeature::Pocket(p) => {
                pos(p.width, "width")?;
                pos(p.height, "height")?;
                pos(p.depth, "depth")?;
                if p.x < 0.0 || p.y < 0.0 || p.x + p.width > w + 1e-9 || p.y + p.height > h + 1e-9 {
                    return Err("pocket outside panel".into());
                }
            }
            MachiningFeature::Groove(g) => {
                pos(g.length, "length")?;
                pos(g.width, "width")?;
                pos(g.depth, "depth")?;
                if g.depth >= t {
                    return Err("groove deeper than thickness".into());
                }
            }
            MachiningFeature::Contour(c) => {
                if c.polygon.points.len() < 3 {
                    return Err("contour needs 3+ points".into());
                }
            }
        }
        Ok(())
    }
}

/// Edge banding applied to a narrow edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeBand {
    pub edge: EdgeSide,
    pub material_code: String,
    pub thickness_mm: f64,
}
