//! Cabinet generators. A generator turns a [`CabinetSpec`] into *domain templates*
//! whose dimensions and positions are parametric expressions over the cabinet's
//! parameters (e.g. `cabinet.inner_width`). No geometry is created here.
//!
//! Cabinet frame: X = width (left → right), Y = height (up), Z = depth (back → front).
//! Panel local frame: X = width, Y = height, Z = thickness.

use crate::{GrainDirection, HardwareKind, MaterialId, PanelRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CabinetKind {
    Base,
    Wall,
    Wardrobe,
    OpenShelf,
    Drawer,
    Door,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JoinStyle {
    /// Top/bottom sit between the sides.
    #[default]
    Inset,
    /// Top/bottom cover the sides' end grain.
    Overlay,
    /// Two stretcher rails (giằng) instead of a full top — kitchen base units.
    Rails,
}

/// Creation input for a cabinet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CabinetSpec {
    pub kind: CabinetKind,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    pub thickness: f64,
    pub back_thickness: f64,
    pub plinth_height: f64,
    pub shelves: u32,
    pub doors: u32,
    pub drawers: u32,
    pub back_panel: bool,
    pub top_style: JoinStyle,
    pub bottom_style: JoinStyle,
    pub carcass_material: MaterialId,
    pub front_material: MaterialId,
    pub back_material: MaterialId,
    /// Tên phòng.
    #[serde(default)]
    pub room: String,
    /// Tầng (Tầng 1, Tầng 2, …); empty = one-storey project.
    #[serde(default)]
    pub floor: String,
    /// Luật dán cạnh (None = default rule).
    #[serde(default)]
    pub edge_rule: Option<crate::EdgeRule>,
    /// Rãnh hậu (0 = back inset without groove).
    #[serde(default)]
    pub back_groove: f64,
}

impl CabinetSpec {
    pub fn preset(kind: CabinetKind) -> Self {
        let base = CabinetSpec {
            kind,
            width: 800.0,
            height: 720.0,
            depth: 560.0,
            thickness: 17.2,
            back_thickness: 8.6,
            plinth_height: 0.0,
            shelves: 1,
            doors: 2,
            drawers: 0,
            back_panel: true,
            top_style: JoinStyle::Inset,
            bottom_style: JoinStyle::Inset,
            carcass_material: MaterialId::new("MDF17-WHITE"),
            front_material: MaterialId::new("MDF17-OAK"),
            back_material: MaterialId::new("MDF8-WHITE"),
            room: String::new(),
            floor: String::new(),
            edge_rule: None,
            back_groove: 0.0,
        };
        match kind {
            CabinetKind::Base => CabinetSpec { width: 800.0, height: 850.0, depth: 600.0, top_style: JoinStyle::Rails, ..base },
            CabinetKind::Wall => CabinetSpec { depth: 320.0, ..base },
            CabinetKind::Wardrobe => CabinetSpec {
                width: 1600.0,
                height: 2400.0,
                depth: 600.0,
                plinth_height: 80.0,
                shelves: 4,
                doors: 4,
                top_style: JoinStyle::Overlay,
                ..base
            },
            CabinetKind::OpenShelf => CabinetSpec { height: 1600.0, depth: 350.0, shelves: 3, doors: 0, ..base },
            CabinetKind::Drawer => CabinetSpec { plinth_height: 100.0, shelves: 0, doors: 0, drawers: 3, ..base },
            CabinetKind::Door => CabinetSpec { width: 450.0, doors: 1, ..base },
        }
    }
}

impl CabinetKind {
    pub fn label(&self) -> &'static str {
        match self {
            CabinetKind::Base => "Base Cabinet",
            CabinetKind::Wall => "Wall Cabinet",
            CabinetKind::Wardrobe => "Wardrobe",
            CabinetKind::OpenShelf => "Open Shelf",
            CabinetKind::Drawer => "Drawer Cabinet",
            CabinetKind::Door => "Door Cabinet",
        }
    }

    /// Kiểu khung name prefix used for automatic cabinet names (BếpDưới01, TủQA01…).
    pub fn frame_name(&self) -> &'static str {
        match self {
            CabinetKind::Base => "BếpDưới",
            CabinetKind::Wall => "BếpTrên",
            CabinetKind::Wardrobe => "TủQA",
            CabinetKind::OpenShelf => "Kệ",
            CabinetKind::Drawer => "TủNgănKéo",
            CabinetKind::Door => "Tủ1Cánh",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaterialSlot {
    Carcass,
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PanelTemplate {
    pub role: PanelRole,
    pub index: u32,
    pub name: String,
    pub width: String,
    pub height: String,
    pub thickness: String,
    pub position: [String; 3],
    pub rotation_deg: [f64; 3],
    pub material: MaterialSlot,
    pub grain: GrainDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareTemplate {
    pub kind: HardwareKind,
    pub index: u32,
    pub name: String,
    pub size: [f64; 3],
    pub position: [String; 3],
    /// Optional parametric override of size[0] (e.g. rail length = inner width).
    pub length_expr: Option<String>,
    pub catalog_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintTemplate {
    /// Boolean expression that must evaluate non-zero.
    pub expr: String,
    pub code: String,
    pub message: String,
}

/// Output of a generator: cabinet parameters (base + derived) and part templates.
#[derive(Debug, Clone, PartialEq)]
pub struct CabinetLayout {
    /// Base (user editable) parameters, as (name, initial expression).
    pub params: Vec<(String, String)>,
    /// Derived parameters computed from base ones.
    pub derived: Vec<(String, String)>,
    pub constraints: Vec<ConstraintTemplate>,
    pub panels: Vec<PanelTemplate>,
    pub hardware: Vec<HardwareTemplate>,
}

/// Rotation that maps panel local Z (thickness) onto cabinet X (used for sides).
pub const ROT_SIDE: [f64; 3] = [0.0, 90.0, 0.0];
/// Rotation that maps panel local Z onto cabinet Y and local Y onto -Z (horizontal parts).
pub const ROT_HORIZONTAL: [f64; 3] = [-90.0, 0.0, 0.0];

fn num(v: f64) -> String {
    format!("{v}")
}

fn s(v: &str) -> String {
    v.to_string()
}

/// Structural fields that are part of the cabinet object, not the param graph.
pub struct CabinetStructure {
    pub kind: CabinetKind,
    pub shelves: u32,
    pub doors: u32,
    pub drawers: u32,
    pub back_panel: bool,
    pub top_style: JoinStyle,
    pub bottom_style: JoinStyle,
}

impl From<&CabinetSpec> for CabinetStructure {
    fn from(s: &CabinetSpec) -> Self {
        Self {
            kind: s.kind,
            shelves: s.shelves,
            doors: s.doors,
            drawers: s.drawers,
            back_panel: s.back_panel,
            top_style: s.top_style,
            bottom_style: s.bottom_style,
        }
    }
}

/// Base numeric parameters of a cabinet with their initial values.
pub fn base_params(spec: &CabinetSpec) -> Vec<(String, String)> {
    vec![
        (s("width"), num(spec.width)),
        (s("height"), num(spec.height)),
        (s("depth"), num(spec.depth)),
        (s("thickness"), num(spec.thickness)),
        (s("back_thickness"), num(spec.back_thickness)),
        (s("plinth_height"), num(spec.plinth_height)),
        (s("door_thickness"), s("thickness")),
        (s("door_gap"), num(2.0)),
        (s("shelf_setback"), num(20.0)),
        (s("back_groove"), num(spec.back_groove)),
        (s("back_offset"), num(if spec.back_groove > 0.0 { 10.0 } else { 0.0 })),
        (s("rail_width"), num(100.0)),
    ]
}

/// Build the parametric layout for a cabinet structure.
pub fn generate(st: &CabinetStructure) -> CabinetLayout {
    let top_overlay = st.top_style == JoinStyle::Overlay;
    let bottom_overlay = st.bottom_style == JoinStyle::Overlay;
    let back = st.back_panel;

    let mut derived: Vec<(String, String)> = vec![
        (s("inner_width"), s("width - 2 * thickness")),
        (s("side_y"), if bottom_overlay { s("plinth_height + thickness") } else { s("0") }),
        (
            s("side_height"),
            format!("height - side_y{}", if top_overlay { " - thickness" } else { "" }),
        ),
        (s("inner_bottom"), s("plinth_height + thickness")),
        (s("inner_top"), s("height - thickness")),
        (s("inner_height"), s("inner_top - inner_bottom")),
        (s("inner_depth"), if back { s("depth - back_thickness") } else { s("depth") }),
        (s("shelf_depth"), s("inner_depth - shelf_setback")),
        (s("front_height"), s("height - plinth_height")),
    ];

    let mut panels = Vec::new();
    let mut hardware = Vec::new();
    let h = |role, index, name: String, w: &str, hh: &str, t: &str, pos: [&str; 3], rot, mat| PanelTemplate {
        role,
        index,
        name,
        width: format!("cabinet.{w}"),
        height: format!("cabinet.{hh}"),
        thickness: format!("cabinet.{t}"),
        position: pos.map(|p| p.to_string()),
        rotation_deg: rot,
        material: mat,
        grain: GrainDirection::AlongHeight,
    };

    // Sides: local X (width) = cabinet depth, placed with ROT_SIDE so local X → -Z.
    panels.push(h(
        PanelRole::LeftSide,
        0,
        "Left Panel".into(),
        "depth",
        "side_height",
        "thickness",
        ["0", "cabinet.side_y", "cabinet.depth"],
        ROT_SIDE,
        MaterialSlot::Carcass,
    ));
    panels.push(h(
        PanelRole::RightSide,
        0,
        "Right Panel".into(),
        "depth",
        "side_height",
        "thickness",
        ["cabinet.width - cabinet.thickness", "cabinet.side_y", "cabinet.depth"],
        ROT_SIDE,
        MaterialSlot::Carcass,
    ));

    // Top / bottom: local Y (height) = depth, laid flat with ROT_HORIZONTAL (local Y → -Z).
    derived.push((s("top_width"), if top_overlay { s("width") } else { s("inner_width") }));
    derived.push((s("bottom_width"), if bottom_overlay { s("width") } else { s("inner_width") }));
    derived.push((s("horizontal_depth"), s("inner_depth")));
    let top_x = if top_overlay { "0" } else { "cabinet.thickness" };
    let bottom_x = if bottom_overlay { "0" } else { "cabinet.thickness" };
    let mut top = h(
        PanelRole::Top,
        0,
        "Top".into(),
        "top_width",
        "horizontal_depth",
        "thickness",
        [top_x, "cabinet.height - cabinet.thickness", "cabinet.depth"],
        ROT_HORIZONTAL,
        MaterialSlot::Carcass,
    );
    top.grain = GrainDirection::AlongWidth;
    panels.push(top);
    let mut bottom = h(
        PanelRole::Bottom,
        0,
        "Bottom".into(),
        "bottom_width",
        "horizontal_depth",
        "thickness",
        [bottom_x, "cabinet.plinth_height", "cabinet.depth"],
        ROT_HORIZONTAL,
        MaterialSlot::Carcass,
    );
    bottom.grain = GrainDirection::AlongWidth;
    panels.push(bottom);

    if back {
        derived.push((s("back_width"), s("inner_width")));
        derived.push((s("back_height"), s("inner_top - plinth_height")));
        panels.push(h(
            PanelRole::Back,
            0,
            "Back".into(),
            "back_width",
            "back_height",
            "back_thickness",
            ["cabinet.thickness", "cabinet.plinth_height", "0"],
            [0.0; 3],
            MaterialSlot::Back,
        ));
    }

    // Shelves evenly spaced in the inner height.
    let n = st.shelves;
    if n > 0 {
        derived.push((s("shelf_pitch"), format!("(inner_height - {n} * thickness) / {}", n + 1)));
    }
    for i in 0..n {
        let y = format!("cabinet.inner_bottom + {} * cabinet.shelf_pitch + {i} * cabinet.thickness", i + 1);
        let mut p = h(
            PanelRole::Shelf,
            i,
            format!("Shelf {:02}", i + 1),
            "inner_width",
            "shelf_depth",
            "thickness",
            ["cabinet.thickness", &y, "cabinet.depth - cabinet.shelf_setback"],
            ROT_HORIZONTAL,
            MaterialSlot::Carcass,
        );
        p.grain = GrainDirection::AlongWidth;
        panels.push(p);
    }

    // Plinth (kick board) for cabinets standing on the floor.
    if matches!(st.kind, CabinetKind::Base | CabinetKind::Wardrobe | CabinetKind::Drawer) {
        let mut p = h(
            PanelRole::Plinth,
            0,
            "Plinth".into(),
            "inner_width",
            "plinth_height",
            "thickness",
            ["cabinet.thickness", "0", "cabinet.depth - 50 - cabinet.thickness"],
            [0.0; 3],
            MaterialSlot::Carcass,
        );
        p.grain = GrainDirection::AlongWidth;
        panels.push(p);
    }

    // Doors (overlay, in front of the carcass).
    let d = st.doors;
    if d > 0 {
        derived.push((s("door_width"), format!("(width - {} * door_gap) / {d}", d + 1)));
        derived.push((s("door_height"), s("front_height - 2 * door_gap")));
    }
    for i in 0..d {
        let x = format!("cabinet.door_gap + {i} * (cabinet.door_width + cabinet.door_gap)");
        panels.push(h(
            PanelRole::Door,
            i,
            if d == 1 { "Door".into() } else { format!("Door {:02}", i + 1) },
            "door_width",
            "door_height",
            "door_thickness",
            [&x, "cabinet.plinth_height + cabinet.door_gap", "cabinet.depth"],
            [0.0; 3],
            MaterialSlot::Front,
        ));
        // Handle on the opening side: door 0 of a pair opens right, door 1 opens left.
        let opening_right = d == 1 || i % 2 == 0;
        let hx = if opening_right {
            format!("{x} + cabinet.door_width - 40 - 6")
        } else {
            format!("{x} + 40 - 6")
        };
        let hy = if st.kind == CabinetKind::Wall {
            s("cabinet.plinth_height + cabinet.door_gap + 60")
        } else {
            s("cabinet.plinth_height + cabinet.door_gap + cabinet.door_height - 60 - 128")
        };
        hardware.push(HardwareTemplate {
            kind: HardwareKind::Handle,
            index: i,
            name: format!("Handle {:02}", i + 1),
            size: [12.0, 160.0, 30.0],
            position: [hx, hy, s("cabinet.depth + cabinet.door_thickness")],
            length_expr: None,
            catalog_code: "HDL-BAR-160".into(),
        });
    }

    // Drawer fronts stacked from the plinth up.
    let k = st.drawers;
    if k > 0 {
        derived.push((s("drawer_front_width"), s("width - 2 * door_gap")));
        derived.push((s("drawer_front_height"), format!("(front_height - {} * door_gap) / {k}", k + 1)));
    }
    for i in 0..k {
        let y = format!("cabinet.plinth_height + cabinet.door_gap + {i} * (cabinet.drawer_front_height + cabinet.door_gap)");
        let mut p = h(
            PanelRole::DrawerFront,
            i,
            format!("Drawer {:02}", i + 1),
            "drawer_front_width",
            "drawer_front_height",
            "door_thickness",
            ["cabinet.door_gap", &y, "cabinet.depth"],
            [0.0; 3],
            MaterialSlot::Front,
        );
        p.grain = GrainDirection::AlongWidth;
        panels.push(p);
        hardware.push(HardwareTemplate {
            kind: HardwareKind::Handle,
            index: d + i,
            name: format!("Drawer Handle {:02}", i + 1),
            size: [160.0, 12.0, 30.0],
            position: [
                s("cabinet.width / 2 - 80"),
                format!("{y} + cabinet.drawer_front_height / 2 - 6"),
                s("cabinet.depth + cabinet.door_thickness"),
            ],
            length_expr: None,
            catalog_code: "HDL-BAR-160".into(),
        });
    }

    // Hanging rail for wardrobes, under the top.
    if st.kind == CabinetKind::Wardrobe {
        hardware.push(HardwareTemplate {
            kind: HardwareKind::Rail,
            index: 0,
            name: "Hanging Rail".into(),
            size: [0.0, 25.0, 25.0],
            position: [s("cabinet.thickness"), s("cabinet.inner_top - 80"), s("cabinet.depth / 2 - 12.5")],
            length_expr: Some(s("cabinet.inner_width")),
            catalog_code: "RAIL-OVAL-25".into(),
        });
    }

    let mut constraints = vec![
        ConstraintTemplate {
            expr: s("width > 2 * thickness + 10"),
            code: "WIDTH_LESS_THAN_SIDES".into(),
            message: "Width cannot be smaller than the thickness of both side panels".into(),
        },
        ConstraintTemplate {
            expr: s("inner_height > 10"),
            code: "HEIGHT_TOO_SMALL".into(),
            message: "Height leaves no inner space".into(),
        },
        ConstraintTemplate {
            expr: s("inner_depth > shelf_setback + 10"),
            code: "DEPTH_TOO_SMALL".into(),
            message: "Depth leaves no inner space".into(),
        },
        ConstraintTemplate {
            expr: s("thickness >= 3 && thickness <= 60"),
            code: "THICKNESS_OUT_OF_RANGE".into(),
            message: "Board thickness must be between 3 and 60 mm".into(),
        },
    ];
    if n > 0 {
        constraints.push(ConstraintTemplate {
            expr: s("shelf_pitch > 20"),
            code: "TOO_MANY_SHELVES".into(),
            message: "Not enough height for this number of shelves".into(),
        });
    }
    if d > 0 {
        constraints.push(ConstraintTemplate {
            expr: s("door_width > 50"),
            code: "DOOR_TOO_NARROW".into(),
            message: "Doors would be narrower than 50 mm".into(),
        });
    }

    CabinetLayout { params: Vec::new(), derived, constraints, panels, hardware }
}

pub fn material_for(slot: MaterialSlot, carcass: &MaterialId, front: &MaterialId, back: &MaterialId) -> MaterialId {
    match slot {
        MaterialSlot::Carcass => carcass.clone(),
        MaterialSlot::Front => front.clone(),
        MaterialSlot::Back => back.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wardrobe_layout_has_expected_parts() {
        let spec = CabinetSpec::preset(CabinetKind::Wardrobe);
        let l = generate(&CabinetStructure::from(&spec));
        let count = |r| l.panels.iter().filter(|p| p.role == r).count();
        assert_eq!(count(PanelRole::LeftSide), 1);
        assert_eq!(count(PanelRole::Shelf), 4);
        assert_eq!(count(PanelRole::Door), 4);
        assert_eq!(count(PanelRole::Back), 1);
        assert!(l.hardware.iter().any(|h| h.kind == HardwareKind::Rail));
    }
}
