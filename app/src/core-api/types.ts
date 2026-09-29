// TypeScript mirror of the Rust wire protocol (crates/aic-api/src/protocol.rs).
// The UI never owns project data: these are read models returned by the core.

export type ObjectId = number;
export type Vec3 = [number, number, number];

export interface Transform3D {
  translation: Vec3;
  rotation_deg: Vec3;
}

export type CabinetKind = 'BASE' | 'WALL' | 'WARDROBE' | 'OPEN_SHELF' | 'DRAWER' | 'DOOR';
export type EdgeSide = 'LEFT' | 'RIGHT' | 'BOTTOM' | 'TOP';
export type FaceSide = 'A' | 'B';

export type CoreEvent =
  | { type: 'ObjectCreated'; ids: ObjectId[] }
  | { type: 'ObjectDeleted'; ids: ObjectId[] }
  | { type: 'ObjectChanged'; ids: ObjectId[] }
  | { type: 'GeometryChanged'; ids: ObjectId[] }
  | { type: 'TransformChanged'; ids: ObjectId[] }
  | { type: 'SceneTreeChanged' }
  | { type: 'SelectionInvalidated'; ids: ObjectId[] }
  | { type: 'ProjectLoaded' };

export interface ApiError {
  code: string;
  message: string;
  details: Record<string, unknown> | null;
}

export interface Response<T = unknown> {
  ok: boolean;
  result: T;
  events: CoreEvent[];
  error: ApiError | null;
  revision: number;
  can_undo: boolean;
  can_redo: boolean;
}

export interface TreeNode {
  id: ObjectId;
  name: string;
  kind: 'ROOM' | 'CABINET' | 'PANEL' | 'HARDWARE';
  role: string | null;
  visible: boolean;
  locked: boolean;
  generated: boolean;
  /** Cabinet room (phòng); '' = chưa gán. */
  room?: string;
  /** Cabinet floor (tầng); '' = chưa gán. */
  floor?: string;
  children: TreeNode[];
}

export interface SceneTree {
  name: string;
  roots: TreeNode[];
}

export interface PropertyField {
  key: string;
  label: string;
  value: number | string | boolean | null | unknown[];
  source: string | null;
  expression: boolean;
  unit: 'mm' | 'deg' | null;
  kind: 'number' | 'text' | 'select' | 'bool' | 'readonly' | 'list' | 'anchors';
  options: { value: string; label: string; color?: string }[];
  editable: boolean;
  error: string | null;
  locked?: boolean;
  /** Multi-selection: values differ between the selected objects. */
  mixed?: boolean;
}

export interface PropertyGroup {
  key: string;
  title: string;
  fields: PropertyField[];
}

export interface Bounds {
  min: Vec3;
  max: Vec3;
}

export interface PropertySheet {
  id: ObjectId;
  /** Multi-selection sheet: edits go to every id. */
  ids?: ObjectId[];
  kind: TreeNode['kind'];
  name: string;
  locked: boolean;
  groups: PropertyGroup[];
  bounds: Bounds | null;
}

export interface MeshData {
  positions: number[];
  normals: number[];
  indices: number[];
  face_ids: number[];
  edges: number[];
  edge_ids: number[];
}

export interface RenderObject {
  id: ObjectId;
  kind: 'PANEL' | 'HARDWARE' | 'ROOM';
  role: string | null;
  name: string;
  geometry_key: string;
  matrix: number[];
  color: string;
  material_id: string | null;
  visible: boolean;
  locked: boolean;
  parent: ObjectId | null;
  cabinet: ObjectId | null;
  size: Vec3 | null;
}

export interface RenderBatch {
  objects: RenderObject[];
  meshes: Record<string, MeshData>;
}

export interface Material {
  id: string;
  name: string;
  kind: 'MDF' | 'PLYWOOD' | 'PARTICLEBOARD' | 'HDF' | 'SOLID_WOOD';
  thickness_mm: number;
  has_grain: boolean;
  sheet_width_mm: number;
  sheet_height_mm: number;
  color: string;
  texture: string | null;
}

export type ContactType = 'TOUCH' | 'GAP' | 'PENETRATE';

export interface AssemblyRelation {
  source: ObjectId;
  target: ObjectId;
  contact: ContactType;
  orientation: 'PARALLEL' | 'PERPENDICULAR' | 'OBLIQUE';
  source_region: 'FACE' | 'EDGE' | 'END';
  target_region: 'FACE' | 'EDGE' | 'END';
  gap_mm: number;
  penetration_mm: number;
  contact_area_mm2: number;
  angle_deg: number;
  normal: Vec3;
  contact_region: Vec3[];
}

export interface Point2 {
  x: number;
  y: number;
}
export interface Polygon2D {
  points: Point2[];
}

export type DrillPurpose = 'GENERIC' | 'SHELF_PIN' | 'DOWEL' | 'CAM_LOCK' | 'CONNECTOR' | 'HINGE_CUP' | 'HINGE_SCREW' | 'HANDLE';

export type MachiningFeature =
  | { type: 'DRILL'; x: number; y: number; diameter: number; depth: number; side: FaceSide; purpose: DrillPurpose }
  | { type: 'EDGE_DRILL'; edge: EdgeSide; offset: number; z: number; diameter: number; depth: number; purpose: DrillPurpose }
  | { type: 'POCKET'; x: number; y: number; width: number; height: number; depth: number; side: FaceSide; corner_radius: number }
  | { type: 'GROOVE'; x: number; y: number; length: number; width: number; depth: number; direction: 'X' | 'Y'; side: FaceSide }
  | { type: 'CONTOUR'; polygon: Polygon2D; inner: boolean; depth: number };

export type FeatureOrigin = { kind: 'USER'; index: number } | { kind: 'RULE' } | { kind: 'JOINT'; with: ObjectId };

export interface FlatPanel {
  id: ObjectId;
  name: string;
  role: string;
  material_id: string;
  width: number;
  height: number;
  thickness: number;
  grain: 'ALONG_HEIGHT' | 'ALONG_WIDTH' | 'NONE';
  outer: Polygon2D;
  inner: Polygon2D[];
  features: { feature: MachiningFeature; origin: FeatureOrigin }[];
  edge_bands: { edge: EdgeSide; material_code: string; thickness_mm: number }[];
  summary: { drills: number; edge_drills: number; pockets: number; grooves: number; contours: number; edge_bands: string[] };
}

export interface PartRow {
  id: ObjectId;
  name: string;
  cabinet: string | null;
  role: string;
  material_id: string;
  length: number;
  width: number;
  thickness: number;
  grain: string;
  edge_bands: string[];
  drills: number;
  grooves: number;
  pockets: number;
}

export interface PartsReport {
  parts: PartRow[];
  materials: { material_id: string; name: string; count: number; area_m2: number }[];
}

export interface NestingSettings {
  spacing_mm: number;
  margin_mm: number;
  allow_rotation: boolean;
}

export interface NestingPlacement {
  part_id: ObjectId;
  instance: number;
  sheet_id: number;
  x_mm: number;
  y_mm: number;
  rotation_deg: number;
  width_mm: number;
  height_mm: number;
}

export interface NestingJobResult {
  material_id: string;
  sheet: { material_id: string; width_mm: number; height_mm: number; thickness_mm: number; has_grain: boolean };
  result: {
    material_id: string;
    sheets: { id: number; width_mm: number; height_mm: number; used_area_mm2: number; utilization: number }[];
    placements: NestingPlacement[];
    unplaced: ObjectId[];
    waste_ratio: number;
  };
  names: Record<string, string>;
}

export interface CncMove {
  x: number;
  y: number;
  z: number;
  rapid: boolean;
}

export interface CncTool {
  id: number;
  name: string;
  kind: 'DRILL' | 'END_MILL';
  diameter: number;
  rpm: number;
  feed: number;
  plunge: number;
  step_down: number;
}

export interface CncProgram {
  sheet_id: number;
  sheet_width: number;
  sheet_height: number;
  thickness: number;
  tools: CncTool[];
  operations: { index: number; tool_id: number; kind: string; part_id: ObjectId; moves: CncMove[] }[];
  gcode: string;
  warnings: string[];
  stats: { tool_changes: number; cut_length_mm: number; rapid_length_mm: number; estimated_time_s: number };
}

export interface SnapHint {
  kind: 'FACE' | 'ALIGN' | 'CENTER' | 'GRID';
  axis: 0 | 1 | 2;
  value: number;
  target: ObjectId | null;
}

export interface SnapResult {
  delta: Vec3;
  hints: SnapHint[];
}

// ---- Zone model (Tạo tấm / Chỉnh tấm) ----
export type SplitKind = 'SHELF_ADJUSTABLE' | 'SHELF_FIXED' | 'DIVIDER' | 'BACK_SUB';
export type Lock = 'EVEN' | 'RATIO' | 'FROM_START' | 'FROM_END';
export type Mount = 'OVERLAY' | 'INSET';
export type DoorKind = 'SINGLE' | 'DOUBLE' | 'SLIDING';
export type HingeSide = 'LEFT' | 'RIGHT' | 'TOP' | 'BOTTOM';
export type StopRail = 'NONE' | 'L_SHAPE' | 'STRAIGHT';

export interface StopRailSpec {
  kind: StopRail;
  height: number;
  cover_up: number;
  leg_depth: number;
  setback: number;
}

export interface ZoneBox {
  id: number;
  min: Vec3;
  size: Vec3;
  leaf: boolean;
  depth: number;
  has_front: boolean;
}

export interface PanelPosition {
  uid: number;
  zone: number;
  axis: 0 | 1 | 2;
  lock: Lock;
  ratio: number;
  from_start: number;
  from_end: number;
  cell_before: number;
  cell_after: number;
  zone_length: number;
}

export type FrontSpec =
  | { type: 'DOORS'; uid: number; kind: DoorKind; cols: number; rows: number; mount: Mount; hinge: HingeSide; stop: StopRailSpec }
  | { type: 'DRAWERS'; uid: number; cols: number; count: number; mount: Mount; with_box: boolean };

export interface ZoneAttachment {
  zone: number;
  uid: number;
  front?: FrontSpec;
  link?: { uid: number; kind: 'OVAL_RAIL'; offset: number };
}

export interface ZonesInfo {
  cabinet: ObjectId;
  name: string;
  room: string;
  matrix: number[];
  zones: ZoneBox[];
  positions: PanelPosition[];
  bays: BayInfo[];
  /** Drawer fronts (0 = bottom), editable heights. */
  front_bays: FrontBay[];
  /** Cabinet W, H, D. */
  size: [number, number, number];
  problems: number[];
  anchors: { width: Anchor; height: Anchor; depth: Anchor };
  /** Split panels: layout uid ↔ scene object id. */
  panels: { uid: number; id: ObjectId }[];
  attachments: ZoneAttachment[];
  fittings: Record<string, unknown>;
}

export type BayMode = 'AUTO' | 'LOCK' | 'PERCENT';

export interface FrontBay {
  uid: number;
  index: number;
  start: number;
  size: number;
  x0: number;
  x1: number;
  z: number;
  mode: BayMode | null;
  value: number;
  usable: number;
}
export type Anchor = 'START' | 'CENTER' | 'END';

/** One bay (khoang) of a split zone, cabinet frame. */
export interface BayInfo {
  zone: number;
  index: number;
  child: number;
  axis: 0 | 1 | 2;
  start: number;
  size: number;
  /** null = legacy per-panel positioning (converted on first edit). */
  mode: BayMode | null;
  value: number;
  usable: number;
}

export interface CostLine {
  key: string;
  name: string;
  qty: number;
  unit: string;
  price: number;
  amount: number;
  factor?: number;
}

export interface CutRow {
  id: ObjectId;
  cabinet: string;
  room: string;
  name: string;
  full_name: string;
  role: string;
  material_id: string;
  material: string;
  length: number;
  width: number;
  thickness: number;
  cut_length: number;
  cut_width: number;
  qty: number;
  edges: [EdgeSide, string][];
  edge_m: number;
  machining: string[];
  note: string;
}

export interface Costing {
  panels: CostLine[];
  edges: CostLine[];
  fittings: CostLine[];
  totals: { panels: number; edges: number; fittings: number; total: number };
  cut_list: CutRow[];
  cabinets: { id: ObjectId; room: string; name: string; frame: string; size: Vec3; panels: number; amount: number }[];
}

export type Corner = 'BOTTOM_LEFT' | 'BOTTOM_RIGHT' | 'TOP_RIGHT' | 'TOP_LEFT';
export type ShapeOp =
  | { kind: 'CORNERS'; corners: Corner[]; size: number; chamfer: boolean }
  | { kind: 'CUT_LINE'; a: [number, number]; b: [number, number]; keep: 'AUTO' | 'LEFT' | 'RIGHT' }
  | { kind: 'CUT_BY_PANEL'; cutter: ObjectId; clearance: number };

export interface PartModPatch {
  name?: string;
  extend?: [number, number, number, number];
  extend_delta?: [number, number, number, number];
  thickness?: number | null;
  clear_tools?: boolean;
  /** Bỏ hình dạng (bo góc / cắt) — về lại hình chữ nhật. */
  clear_shape?: boolean;
  add_anchor?: { edge: EdgeSide; target: ObjectId; face?: 'INNER' | 'OUTER'; offset?: number };
  remove_anchor?: number;
  /** Chia tấm: null = bỏ chia. */
  split?: { axis: 'X' | 'Y'; count: number; gap: number } | null;
  add_features?: MachiningFeature[];
  tool?: string;
}

export interface TemplatesInfo {
  templates: { name: string; kind: CabinetKind; frame: string; size: [number, number, number]; zones: number }[];
  presets: { name: string; builtin: boolean; values: Record<string, number>; top_style: string | null; bottom_style: string | null }[];
}
