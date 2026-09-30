use crate::{EdgeBand, EdgeSide, MachiningFeature, MaterialId, ObjectId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PanelRole {
    Generic,
    LeftSide,
    RightSide,
    Top,
    Bottom,
    Back,
    Shelf,
    Divider,
    Door,
    DrawerFront,
    Plinth,
    ShelfFixed,
    BackSub,
    DrawerSide,
    DrawerBack,
    DrawerBottom,
    Rail,
    /// Phào, ốp hông, nẹp.
    Trim,
}

impl PanelRole {
    pub fn label(&self) -> &'static str {
        match self {
            PanelRole::Generic => "Panel",
            PanelRole::LeftSide => "Left Panel",
            PanelRole::RightSide => "Right Panel",
            PanelRole::Top => "Top",
            PanelRole::Bottom => "Bottom",
            PanelRole::Back => "Back",
            PanelRole::Shelf => "Shelf",
            PanelRole::Divider => "Divider",
            PanelRole::Door => "Door",
            PanelRole::DrawerFront => "Drawer Front",
            PanelRole::Plinth => "Plinth",
            PanelRole::ShelfFixed => "Fixed Shelf",
            PanelRole::BackSub => "Sub Back",
            PanelRole::DrawerSide => "Drawer Side",
            PanelRole::DrawerBack => "Drawer Back",
            PanelRole::DrawerBottom => "Drawer Bottom",
            PanelRole::Rail => "Rail",
            PanelRole::Trim => "Trim",
        }
    }

    pub fn is_side(&self) -> bool {
        matches!(self, PanelRole::LeftSide | PanelRole::RightSide)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrainDirection {
    /// Grain runs along the panel height (local Y).
    #[default]
    AlongHeight,
    /// Grain runs along the panel width (local X).
    AlongWidth,
    None,
}

/// A wooden board. Its size lives here, in definition space, and is never
/// inferred from a (possibly scaled) world transform. The placement is the
/// scene node's local transform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Panel {
    pub id: ObjectId,
    pub name: String,
    pub role: PanelRole,
    /// Index among siblings of the same role (shelf 0, shelf 1, ...).
    #[serde(default)]
    pub role_index: u32,

    pub width_mm: f64,
    pub height_mm: f64,
    pub thickness_mm: f64,

    pub material_id: MaterialId,
    pub grain_direction: GrainDirection,
    pub edge_bands: Vec<EdgeBand>,
    pub features: Vec<MachiningFeature>,
    /// Stable key of a generated part inside its cabinet (`c:left`, `p:12`, …).
    #[serde(default)]
    pub gen_key: Option<String>,
    /// Machining produced by the cabinet generator (hinge cups, back grooves, tools).
    #[serde(default)]
    pub gen_features: Vec<MachiningFeature>,
    /// Hinge edge of a door.
    #[serde(default)]
    pub hinge: Option<EdgeSide>,
}

impl Panel {
    pub fn new(id: ObjectId, name: impl Into<String>, role: PanelRole, size: [f64; 3], material_id: MaterialId) -> Self {
        Self {
            id,
            name: name.into(),
            role,
            role_index: 0,
            width_mm: size[0],
            height_mm: size[1],
            thickness_mm: size[2],
            material_id,
            grain_direction: GrainDirection::AlongHeight,
            edge_bands: Vec::new(),
            features: Vec::new(),
            gen_key: None,
            gen_features: Vec::new(),
            hinge: None,
        }
    }

    pub fn size(&self) -> [f64; 3] {
        [self.width_mm, self.height_mm, self.thickness_mm]
    }

    pub fn volume_mm3(&self) -> f64 {
        self.width_mm * self.height_mm * self.thickness_mm
    }

    pub fn edge_band(&self, edge: EdgeSide) -> Option<&EdgeBand> {
        self.edge_bands.iter().find(|b| b.edge == edge)
    }

    pub fn set_edge_band(&mut self, edge: EdgeSide, band: Option<EdgeBand>) {
        self.edge_bands.retain(|b| b.edge != edge);
        if let Some(b) = band {
            self.edge_bands.push(b);
            self.edge_bands.sort_by_key(|b| b.edge);
        }
    }
}
