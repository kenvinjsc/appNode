use crate::{CabinetKind, JoinStyle, MaterialId, ObjectId, Panel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub id: ObjectId,
    pub name: String,
    pub width_mm: f64,
    pub depth_mm: f64,
    pub height_mm: f64,
    pub wall_thickness_mm: f64,
}

/// Structural (non-numeric) description of a cabinet. Numeric dimensions
/// (width, height, ...) live in the parametric graph, not here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cabinet {
    pub id: ObjectId,
    pub name: String,
    pub kind: CabinetKind,
    pub shelves: u32,
    pub doors: u32,
    pub drawers: u32,
    pub back_panel: bool,
    pub top_style: JoinStyle,
    pub bottom_style: JoinStyle,
    pub carcass_material: MaterialId,
    pub front_material: MaterialId,
    pub back_material: MaterialId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardwareKind {
    Handle,
    Hinge,
    Rail,
    Leg,
}

impl HardwareKind {
    pub fn label(&self) -> &'static str {
        match self {
            HardwareKind::Handle => "Handle",
            HardwareKind::Hinge => "Hinge",
            HardwareKind::Rail => "Rail",
            HardwareKind::Leg => "Leg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hardware {
    pub id: ObjectId,
    pub name: String,
    pub kind: HardwareKind,
    #[serde(default)]
    pub role_index: u32,
    /// Bounding size in local frame (render + clash only; hardware is not machined).
    pub size_mm: [f64; 3],
    pub catalog_code: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DomainObject {
    Room(Room),
    Cabinet(Cabinet),
    Panel(Panel),
    Hardware(Hardware),
}

/// Borrowed classification helper.
pub type ObjectKind = &'static str;

impl DomainObject {
    pub fn id(&self) -> ObjectId {
        match self {
            DomainObject::Room(o) => o.id,
            DomainObject::Cabinet(o) => o.id,
            DomainObject::Panel(o) => o.id,
            DomainObject::Hardware(o) => o.id,
        }
    }

    pub fn set_id(&mut self, id: ObjectId) {
        match self {
            DomainObject::Room(o) => o.id = id,
            DomainObject::Cabinet(o) => o.id = id,
            DomainObject::Panel(o) => o.id = id,
            DomainObject::Hardware(o) => o.id = id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DomainObject::Room(o) => &o.name,
            DomainObject::Cabinet(o) => &o.name,
            DomainObject::Panel(o) => &o.name,
            DomainObject::Hardware(o) => &o.name,
        }
    }

    pub fn set_name(&mut self, name: String) {
        match self {
            DomainObject::Room(o) => o.name = name,
            DomainObject::Cabinet(o) => o.name = name,
            DomainObject::Panel(o) => o.name = name,
            DomainObject::Hardware(o) => o.name = name,
        }
    }

    pub fn kind(&self) -> ObjectKind {
        match self {
            DomainObject::Room(_) => "ROOM",
            DomainObject::Cabinet(_) => "CABINET",
            DomainObject::Panel(_) => "PANEL",
            DomainObject::Hardware(_) => "HARDWARE",
        }
    }

    pub fn as_panel(&self) -> Option<&Panel> {
        match self {
            DomainObject::Panel(p) => Some(p),
            _ => None,
        }
    }

    pub fn as_panel_mut(&mut self) -> Option<&mut Panel> {
        match self {
            DomainObject::Panel(p) => Some(p),
            _ => None,
        }
    }

    pub fn as_cabinet(&self) -> Option<&Cabinet> {
        match self {
            DomainObject::Cabinet(c) => Some(c),
            _ => None,
        }
    }

    /// Whether the object may contain children in the scene tree.
    pub fn is_container(&self) -> bool {
        matches!(self, DomainObject::Room(_) | DomainObject::Cabinet(_))
    }
}
