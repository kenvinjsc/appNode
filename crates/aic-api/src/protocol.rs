//! Wire protocol between UI and core (JSON over Tauri IPC or HTTP in dev).

use aic_domain::zone::{DoorKind, HingeSide, LinkKind, Lock, Mount, SplitKind, StopRailSpec, Uid};
use aic_domain::{CabinetKind, EdgeSide, MachiningFeature, ObjectId};
use aic_math::Transform3D;
use aic_nesting::NestingSettings;
use aic_project::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CabinetOverrides {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub depth: Option<f64>,
    pub thickness: Option<f64>,
    pub shelves: Option<u32>,
    pub doors: Option<u32>,
    pub drawers: Option<u32>,
    pub back_panel: Option<bool>,
    pub carcass_material: Option<String>,
    pub front_material: Option<String>,
    /// INSET | OVERLAY | RAILS (luật liên kết).
    pub top_style: Option<String>,
    pub bottom_style: Option<String>,
    pub edge_rule: Option<aic_domain::EdgeRule>,
    pub back_groove: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZoneAddPanels {
    pub cabinet: ObjectId,
    /// Pinned zones (ghim vùng); panels are added to each.
    pub zones: Vec<Uid>,
    pub kind: SplitKind,
    #[serde(default = "one")]
    pub count: u32,
    #[serde(default)]
    pub thickness: Option<f64>,
    #[serde(default)]
    pub lock: Lock,
    /// Ratio in % for `RATIO`, mm otherwise.
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub tilt_deg: Option<[f64; 2]>,
}

fn one() -> u32 {
    1
}

fn yes() -> bool {
    true
}

fn sixty() -> f64 {
    60.0
}

/// Partial update of a generated part's modifications.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PartModPatch {
    #[serde(default)]
    pub name: Option<String>,
    /// Absolute extension of left, right, bottom, top edges (co giãn).
    #[serde(default)]
    pub extend: Option<[f64; 4]>,
    /// Added to the current extension (cộng dồn).
    #[serde(default)]
    pub extend_delta: Option<[f64; 4]>,
    #[serde(default)]
    pub thickness: Option<Option<f64>>,
    #[serde(default)]
    pub clear_tools: bool,
    #[serde(default)]
    pub add_features: Option<Vec<MachiningFeature>>,
    #[serde(default)]
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    // project
    CreateProject { name: String },
    LoadProject { project: Value },
    SaveProject,
    // create
    CreateRoom { width: f64, depth: f64, height: f64 },
    CreateCabinet {
        kind: CabinetKind,
        #[serde(default)]
        position: Option<[f64; 3]>,
        #[serde(default)]
        parent: Option<ObjectId>,
        #[serde(default)]
        overrides: CabinetOverrides,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        room: Option<String>,
        #[serde(default)]
        floor: Option<String>,
        /// Place the new cabinet to the right of this one (dãy tủ liền nhau).
        #[serde(default)]
        after: Option<ObjectId>,
    },
    CreatePanel {
        #[serde(default)]
        name: Option<String>,
        width: f64,
        height: f64,
        thickness: f64,
        #[serde(default)]
        material: Option<String>,
        #[serde(default)]
        transform: Option<Transform3D>,
        #[serde(default)]
        parent: Option<ObjectId>,
    },
    // edit
    DeleteObjects { ids: Vec<ObjectId> },
    DuplicateObjects { ids: Vec<ObjectId> },
    SetParameter { id: ObjectId, name: String, value: String },
    SetTransform { id: ObjectId, transform: Transform3D },
    SetName { id: ObjectId, name: String },
    SetVisible { ids: Vec<ObjectId>, visible: bool },
    SetLocked { ids: Vec<ObjectId>, locked: bool },
    Reparent { id: ObjectId, parent: Option<ObjectId>, #[serde(default)] index: Option<usize> },
    SetMaterial { id: ObjectId, material: String, #[serde(default)] slot: Option<String> },
    SetEdgeBand { id: ObjectId, edge: EdgeSide, enabled: bool },
    AddFeature { id: ObjectId, feature: MachiningFeature },
    RemoveFeature { id: ObjectId, index: usize },
    Undo,
    Redo,
    // zones (Tạo tấm / Chỉnh tấm)
    GetZones { cabinet: ObjectId },
    ZoneAddPanels(ZoneAddPanels),
    ZoneAddDoors {
        cabinet: ObjectId,
        zones: Vec<Uid>,
        #[serde(default)]
        kind: DoorKind,
        #[serde(default = "one")]
        cols: u32,
        #[serde(default = "one")]
        rows: u32,
        #[serde(default)]
        mount: Mount,
        #[serde(default)]
        hinge: HingeSide,
        #[serde(default)]
        thickness: Option<f64>,
        #[serde(default)]
        stop: Option<StopRailSpec>,
    },
    ZoneAddDrawers {
        cabinet: ObjectId,
        zones: Vec<Uid>,
        #[serde(default = "one")]
        count: u32,
        #[serde(default = "one")]
        cols: u32,
        #[serde(default)]
        mount: Mount,
        #[serde(default)]
        thickness: Option<f64>,
        #[serde(default = "yes")]
        with_box: bool,
    },
    ZoneAddLink {
        cabinet: ObjectId,
        zones: Vec<Uid>,
        kind: LinkKind,
        #[serde(default = "sixty")]
        offset: f64,
    },
    ZoneRemove { cabinet: ObjectId, uid: Uid },
    SetPartMod { id: ObjectId, patch: PartModPatch },
    // costing (báo giá)
    GetCosting,
    SetPrice {
        key: String,
        #[serde(default)]
        value: Option<f64>,
    },
    // queries
    GetSceneTree,
    GetProperties { id: ObjectId },
    GetRenderObjects {
        #[serde(default)]
        ids: Option<Vec<ObjectId>>,
        /// Geometry keys the client already has; their meshes are not resent.
        #[serde(default)]
        known_keys: Vec<String>,
    },
    GetMaterials,
    GetRelations {
        #[serde(default)]
        id: Option<ObjectId>,
    },
    GetManufacturing { id: ObjectId },
    GetParts,
    RunNesting {
        #[serde(default)]
        material: Option<String>,
        #[serde(default)]
        settings: Option<NestingSettings>,
    },
    GenerateCnc { material: String, sheet_id: u32 },
    Snap { id: ObjectId, delta: [f64; 3], #[serde(default)] grid: Option<f64> },
    GetBounds { ids: Vec<ObjectId> },
    GetTransform { id: ObjectId },
    GetStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum CoreEvent {
    ObjectCreated { ids: Vec<ObjectId> },
    ObjectDeleted { ids: Vec<ObjectId> },
    ObjectChanged { ids: Vec<ObjectId> },
    GeometryChanged { ids: Vec<ObjectId> },
    TransformChanged { ids: Vec<ObjectId> },
    SceneTreeChanged,
    SelectionInvalidated { ids: Vec<ObjectId> },
    ProjectLoaded,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApiError {
    /// Stable error code the UI localises (e.g. `CONSTRAINT_VIOLATED`).
    pub code: String,
    /// Technical message for logs; not meant for end users.
    pub message: String,
    /// Structured details (e.g. `{"constraint": "WIDTH_LESS_THAN_SIDES"}`).
    pub details: Value,
}

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        let mut details = serde_json::to_value(&e).unwrap_or(Value::Null);
        if let Some(o) = details.as_object_mut() {
            o.remove("code");
        }
        ApiError { code: e.code().into(), message: e.to_string(), details }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub ok: bool,
    pub result: Value,
    pub events: Vec<CoreEvent>,
    pub error: Option<ApiError>,
    pub revision: u64,
    pub can_undo: bool,
    pub can_redo: bool,
}
