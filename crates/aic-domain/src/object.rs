use crate::layout::PartMod;
use crate::zone::ZoneTree;
use crate::{CabinetKind, CabinetSpec, JoinStyle, MaterialId, ObjectId, Panel};
use std::collections::BTreeMap;
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
    /// Tên phòng (Bếp, PN1, …).
    #[serde(default)]
    pub room: String,
    /// Tầng (Tầng 1, …). A project is floors → rooms → cabinets.
    #[serde(default)]
    pub floor: String,
    /// Interior layout (zones, splits, fronts, accessories).
    #[serde(default)]
    pub zones: ZoneTree,
    /// Per-part user modifications keyed by part key.
    #[serde(default)]
    pub mods: BTreeMap<String, PartMod>,
    /// Generate handles on doors / drawer fronts.
    #[serde(default = "yes")]
    pub handles: bool,
    /// Edge banding rule (luật dán cạnh).
    #[serde(default)]
    pub edge_rule: EdgeRule,
    /// Zones were built (false for files written before the zone model).
    #[serde(default)]
    pub zones_ready: bool,
    /// Resize anchors (giữ trái/giữa/phải, dưới/giữa/trên, sau/giữa/trước).
    #[serde(default)]
    pub anchors: Anchors,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeMode {
    /// Dán hở bỏ khuất: band only exposed edges.
    #[default]
    ExposedOnly,
    /// Dán toàn bộ.
    All,
    /// Không dán.
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRule {
    pub mode: EdgeMode,
    /// Loại chỉ dán code, e.g. `DON-1` (Đơn 1mm).
    pub band_code: String,
    pub band_thickness: f64,
    /// Board thicknesses that are never banded.
    pub skip_thicknesses: Vec<f64>,
    /// Ngưỡng: an edge within this distance of another part counts as hidden.
    pub threshold: f64,
    /// Bỏ cạnh ngắn ≤.
    pub min_length: f64,
}

impl Default for EdgeRule {
    fn default() -> Self {
        Self {
            mode: EdgeMode::ExposedOnly,
            band_code: "DON-1".into(),
            band_thickness: 1.0,
            skip_thicknesses: vec![8.6],
            threshold: 0.5,
            min_length: 20.0,
        }
    }
}

/// What stays in place when a cabinet dimension changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Anchor {
    /// Keep left / bottom / back (the cabinet origin).
    #[default]
    Start,
    Center,
    /// Keep right / top / front.
    End,
}

impl Anchor {
    /// Fraction of the size change the origin moves back by.
    pub fn factor(&self) -> f64 {
        match self {
            Anchor::Start => 0.0,
            Anchor::Center => 0.5,
            Anchor::End => 1.0,
        }
    }
}

/// Resize anchors for W / H / D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Anchors {
    pub width: Anchor,
    pub height: Anchor,
    pub depth: Anchor,
}

impl Cabinet {
    pub fn from_spec(id: ObjectId, name: String, s: &CabinetSpec) -> Self {
        Cabinet {
            id,
            name,
            kind: s.kind,
            shelves: s.shelves,
            doors: s.doors,
            drawers: s.drawers,
            back_panel: s.back_panel,
            top_style: s.top_style,
            bottom_style: s.bottom_style,
            carcass_material: s.carcass_material.clone(),
            front_material: s.front_material.clone(),
            back_material: s.back_material.clone(),
            room: s.room.clone(),
            floor: s.floor.clone(),
            zones: crate::layout::default_zones(s.kind, s.shelves, s.doors, s.drawers, s.thickness),
            mods: BTreeMap::new(),
            handles: true,
            edge_rule: s.edge_rule.clone().unwrap_or_default(),
            zones_ready: true,
            anchors: Anchors::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardwareKind {
    Handle,
    Hinge,
    Rail,
    Leg,
    Slide,
}

impl HardwareKind {
    pub fn label(&self) -> &'static str {
        match self {
            HardwareKind::Handle => "Handle",
            HardwareKind::Hinge => "Hinge",
            HardwareKind::Rail => "Rail",
            HardwareKind::Leg => "Leg",
            HardwareKind::Slide => "Slide",
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
    #[serde(default)]
    pub gen_key: Option<String>,
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
