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
    pub plinth_height: Option<f64>,
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

/// Chia khoang theo công thức (hộp thoại Chia ngang / Chia dọc).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SplitZone {
    pub cabinet: ObjectId,
    pub zone: Uid,
    /// Tấm chia (SHELF_* / DIVIDER) hoặc VIRTUAL_H / VIRTUAL_V (không tạo tấm).
    pub kind: SplitKind,
    /// `500`, `500,300`, `30%,*`, `3*400`, `/3` … (xem `aic_domain::parse_split_formula`).
    pub formula: String,
    /// Công thức tính từ trên xuống (ngang) / phải sang trái (dọc).
    #[serde(default)]
    pub from_end: bool,
    #[serde(default)]
    pub thickness: Option<f64>,
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
    /// Some(None) (JSON null) = back to the generated thickness.
    #[serde(default, deserialize_with = "some_or_null")]
    pub thickness: Option<Option<f64>>,
    #[serde(default)]
    pub clear_tools: bool,
    /// Ràng buộc động: add `edge → target face (+ offset)`; the target is a part of the same cabinet.
    #[serde(default)]
    pub add_anchor: Option<AnchorPatch>,
    /// Remove the anchor at this index.
    #[serde(default)]
    pub remove_anchor: Option<usize>,
    /// Remove the outer shape override (bo góc / cắt) and restore the rectangle.
    #[serde(default)]
    pub clear_shape: bool,
    /// Chia tấm: Some(split) sets it, Some(None) (JSON null) removes it.
    #[serde(default, deserialize_with = "some_or_null")]
    pub split: Option<Option<aic_domain::PartSplit>>,
    #[serde(default)]
    pub add_features: Option<Vec<MachiningFeature>>,
    #[serde(default)]
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnchorPatch {
    pub edge: aic_domain::EdgeSide,
    pub target: ObjectId,
    #[serde(default)]
    pub face: aic_domain::AnchorFace,
    #[serde(default)]
    pub offset: f64,
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
    /// Tủ góc bếp L mù: phần mù (tấm cố định) phía góc, phần cánh phía ngoài.
    CreateCorner {
        /// LEFT = góc bên trái (phần mù bên trái), RIGHT = góc bên phải.
        /// Tủ góc chéo: bản lề trái (LEFT) / phải (RIGHT).
        hand: String,
        /// `BLIND` (mặc định, L mù) hoặc `DIAGONAL` (góc chéo).
        #[serde(default)]
        kind: Option<String>,
        /// Tủ góc treo (bếp trên).
        #[serde(default)]
        wall: bool,
        #[serde(default)]
        width: Option<f64>,
        #[serde(default)]
        height: Option<f64>,
        #[serde(default)]
        depth: Option<f64>,
        /// Rộng cánh (mm), mặc định 450.
        #[serde(default)]
        door_width: Option<f64>,
        #[serde(default)]
        position: Option<[f64; 3]>,
        #[serde(default)]
        room: Option<String>,
        #[serde(default)]
        floor: Option<String>,
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
    /// Di chuyển khối: move objects by a world-space vector (one undo step).
    MoveObjects { ids: Vec<ObjectId>, delta: [f64; 3] },
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
    SplitZone(SplitZone),
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
        /// Ngăn kéo trong (sau cánh).
        #[serde(default)]
        inner: bool,
        /// Mặt giả (tủ chậu).
        #[serde(default)]
        false_front: bool,
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
    // template / preset / array / mirror
    /// `to_library` (default true): save in the shared library (all projects), else in the project.
    SaveTemplate { cabinet: ObjectId, name: String, #[serde(default = "yes")] to_library: bool },
    InsertTemplate {
        name: String,
        #[serde(default)]
        width: Option<f64>,
        #[serde(default)]
        height: Option<f64>,
        #[serde(default)]
        depth: Option<f64>,
        #[serde(default)]
        position: Option<[f64; 3]>,
        #[serde(default)]
        room: Option<String>,
        #[serde(default)]
        floor: Option<String>,
        #[serde(default)]
        after: Option<ObjectId>,
    },
    DeleteTemplate { name: String },
    /// Bảng Thuộc tính kết cấu (tabs + fields) of a cabinet; mẫu từng tab (thư viện).
    GetStructure { cabinet: ObjectId },
    /// Mẫu sản phẩm dựng sẵn (tủ bếp dưới / trên / góc L / tủ lò…).
    GetProducts,
    InsertProduct {
        key: String,
        #[serde(default)]
        position: Option<[f64; 3]>,
        #[serde(default)]
        room: Option<String>,
        #[serde(default)]
        floor: Option<String>,
        #[serde(default)]
        after: Option<ObjectId>,
    },
    /// Tool gia công theo tham số cho nhiều tấm (khấu góc, khấu bề mặt, rãnh LED / V-bit).
    ToolFeature { ids: Vec<ObjectId>, #[serde(default)] tool: String, feature: aic_domain::ParamFeature },
    /// Bộ vật liệu (thùng / cánh / hậu + chỉ dán), dựng sẵn + thư viện.
    GetMaterialSets,
    SaveMaterialSet { cabinet: ObjectId, name: String },
    ApplyMaterialSet {
        #[serde(default)]
        ids: Vec<ObjectId>,
        /// Áp cho mọi tủ của phòng này.
        #[serde(default)]
        room: Option<String>,
        name: String,
    },
    DeleteMaterialSet { name: String },
    /// Dãy tủ: mặt đá, len chân liền, tấm lấp, che trần sinh theo các tủ đã chọn.
    CreateRun {
        ids: Vec<ObjectId>,
        #[serde(default)]
        rules: Option<aic_domain::run::RunRules>,
    },
    UpdateRun { name: String, rules: aic_domain::run::RunRules },
    DeleteRun { name: String },
    GetRuns,
    SaveGroupPreset { cabinet: ObjectId, group: String, name: String },
    ApplyGroupPreset { ids: Vec<ObjectId>, group: String, name: String },
    DeleteGroupPreset { group: String, name: String },
    /// Mẫu vùng (thư viện): save the content of a zone / apply it to pinned zones.
    SaveZonePreset { cabinet: ObjectId, zone: aic_domain::zone::Uid, name: String },
    ApplyZonePreset { cabinet: ObjectId, zones: Vec<aic_domain::zone::Uid>, name: String },
    DeleteZonePreset { name: String },
    GetTemplates,
    SaveRulePreset { cabinet: ObjectId, name: String, #[serde(default = "yes")] to_library: bool },
    DeleteRulePreset { name: String },
    ApplyRulePreset { ids: Vec<ObjectId>, name: String },
    /// Nhân tấm: `count` more shelves / dividers like this one, bays equal.
    ArraySplitPanel { id: ObjectId, count: u32 },
    /// Nhân dãy tủ: `count` copies along axis 0/1/2 with a gap.
    ArrayCabinet { id: ObjectId, count: u32, #[serde(default)] axis: usize, #[serde(default)] gap: f64 },
    MirrorCabinet { id: ObjectId },
    /// Multi-edit: the same parameter on many objects, one undo step, all or nothing.
    SetParameterMulti { ids: Vec<ObjectId>, name: String, value: String },
    /// Property sheet of several objects: common fields, `mixed` where values differ.
    GetPropertiesMulti { ids: Vec<ObjectId> },
    /// Kích thước khoang: LOCK (mm) / AUTO / PERCENT (%) of one bay of a split zone.
    SetBay {
        cabinet: ObjectId,
        zone: aic_domain::zone::Uid,
        index: usize,
        #[serde(default)]
        mode: Option<aic_domain::zone::BayMode>,
        #[serde(default)]
        value: Option<f64>,
    },
    /// Quan hệ 2 tấm: INSET (lọt) / OVERLAY (phủ) / FLUSH (bằng mặt trước) / GAP (khe) / NONE.
    SetRelation { a: ObjectId, b: ObjectId, kind: crate::relations_edit::RelationKind, #[serde(default)] gap: f64 },
    /// Kéo cạnh tấm (handle 2D): side in the cabinet frame, `delta` > 0 grows.
    ResizePanelSide { id: ObjectId, side: crate::relations_edit::Side, delta: f64, #[serde(default)] constrained: bool },
    /// Kéo kích thước tủ với chế độ dãn khoang: KEEP (giữ chế độ khoang), PROPORTIONAL
    /// (mọi khoang theo tỷ lệ cũ), EDGE (chỉ khoang sát cạnh kéo). `edge` = cạnh đang kéo
    /// (START = trái/dưới/sau, END = phải/trên/trước); None = theo neo của tủ.
    ResizeCabinet {
        id: ObjectId,
        name: String,
        value: f64,
        #[serde(default)]
        stretch: Stretch,
        #[serde(default)]
        edge: Option<aic_domain::Anchor>,
    },
    /// Cao từng ngăn kéo (0 = dưới cùng): LOCK mm / PERCENT / AUTO.
    SetDrawerHeight {
        cabinet: ObjectId,
        uid: aic_domain::zone::Uid,
        index: usize,
        #[serde(default)]
        mode: Option<aic_domain::zone::BayMode>,
        #[serde(default)]
        value: Option<f64>,
    },
    /// Kéo đường chia ngăn kéo: front `index` becomes `before` mm.
    MoveDrawerDivider { cabinet: ObjectId, uid: aic_domain::zone::Uid, index: usize, before: f64 },
    /// Chia đều lại một zone đã chia.
    EqualizeSplit { cabinet: ObjectId, zone: aic_domain::zone::Uid },
    /// Kéo vách / kệ: `before` = clear size (mm) of the bay before the panel.
    MoveSplitPanel { id: ObjectId, before: f64 },
    /// Change the outline of panels: rounded/chamfered corners, straight cut, cut by
    /// another panel (tools 04, 09, 10). Local panel frame.
    ShapeTool { ids: Vec<ObjectId>, op: crate::shape::ShapeOp },
    /// Merge coplanar panels of one cabinet into the first one (tool 06).
    MergePanels { ids: Vec<ObjectId> },
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
    /// Khoang của các tủ này đổi (2D / Tạo tấm chỉ tải lại khoang của chúng).
    ZonesChanged { cabinets: Vec<ObjectId> },
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

/// Distinguish a missing field (None) from an explicit null (Some(None)).
fn some_or_null<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

fn yes() -> bool {
    true
}

/// Chế độ dãn khoang khi đổi kích thước tủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Stretch {
    #[default]
    Keep,
    Proportional,
    Edge,
}
