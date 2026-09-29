//! Wire protocol between UI and core (JSON over Tauri IPC or HTTP in dev).

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
