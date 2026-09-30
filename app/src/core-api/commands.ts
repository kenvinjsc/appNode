// Commands: every mutation goes UI action → command → core → events.
import { send } from './transport';
import { emitCoreEvents, emitStatus } from './events';
import { describeError, type UserMessage } from './errors';
import type { CabinetKind, DoorKind, EdgeSide, HingeSide, Lock, MachiningFeature, Mount, ObjectId, PartModPatch, SplitKind, StopRailSpec, Transform3D, Vec3, ShapeOp, BayMode, RelationKind, PanelSide, StructureInfo, RunRules, RunDef, MaterialSet, ParamFeature, BayInfo, PanelPosition, BackCutout, AccessoryInfo } from './types';

export class CommandError extends Error {
  constructor(public readonly user: UserMessage, public readonly code: string) {
    super(user.title);
  }
}

type Notifier = (m: UserMessage) => void;
let notify: Notifier = () => undefined;
export function setErrorNotifier(n: Notifier) {
  notify = n;
}

export async function command<T = Record<string, unknown>>(req: Record<string, unknown>, opts: { silent?: boolean } = {}): Promise<T> {
  const r = await send<T>(req);
  emitStatus(r);
  emitCoreEvents(r.events);
  if (!r.ok || r.error) {
    const msg = describeError(r.error!);
    if (!opts.silent && r.error!.code !== 'NOTHING_TO') notify(msg);
    throw new CommandError(msg, r.error!.code);
  }
  return r.result;
}

export const Commands = {
  createProject: (name: string) => command({ cmd: 'create_project', name }),
  loadProject: (project: unknown) => command({ cmd: 'load_project', project }),
  createRoom: (width: number, depth: number, height: number) => command<{ id: ObjectId }>({ cmd: 'create_room', width, depth, height }),
  createCabinet: (kind: CabinetKind, position: Vec3 | null, overrides: Record<string, unknown> = {}, extra: { parent?: ObjectId; name?: string; room?: string; floor?: string; after?: ObjectId } = {}) =>
    command<{ id: ObjectId }>({ cmd: 'create_cabinet', kind, position: position ?? undefined, overrides, ...extra }),
  /** Chia khoang theo công thức (`500`, `500,300`, `30%,*`, `3*400`, `/3`). */
  /** Tủ góc bếp L mù (phần mù phía góc + 1 cánh). */
  createCorner: (hand: 'LEFT' | 'RIGHT', after?: ObjectId, width?: number, door_width?: number) => command<{ id: ObjectId }>({ cmd: 'create_corner', hand, after, width, door_width }),
  /** Tủ góc chéo (mặt cánh xiên 45°), bếp dưới hoặc bếp trên. */
  createDiagonalCorner: (wall: boolean, after?: ObjectId) => command<{ id: ObjectId }>({ cmd: 'create_corner', kind: 'DIAGONAL', hand: 'LEFT', wall, after }),
  /** Sản phẩm ngoài tủ hộp (giường …): W × D × H theo sản phẩm, tuỳ chọn như set_parameter; một undo. */
  createFurniture: (kind: string, size: { width?: number; height?: number; depth?: number }, options: Record<string, string> = {}, room?: string) =>
    command<{ id: ObjectId }>({ cmd: 'create_furniture', kind, ...size, options, room }),
  /** Bản vẽ in: trang A3 / A4 đã chiếu (mm giấy). */
  getDrawingSheet: (p: { ids?: ObjectId[]; room?: string; floor?: string; paper?: string; portrait?: boolean; views?: string[]; hide_fronts?: boolean; drawer?: string; date?: string }) =>
    command<DrawingSheet>({ cmd: 'get_drawing_sheet', ...p }),
  /** Xuất file máy theo tấm (DXF / MPR / CIX). */
  exportMachine: (ids: ObjectId[], format: 'DXF' | 'MPR' | 'CIX', flip_for_b = true) =>
    command<{ files: { id: ObjectId; name: string; content: string }[] }>({ cmd: 'export_machine', ids, format, flip_for_b }),
  getMaterialSets: () => command<{ sets: { set: MaterialSet; builtin: boolean }[] }>({ cmd: 'get_material_sets' }),
  saveMaterialSet: (cabinet: ObjectId, name: string) => command({ cmd: 'save_material_set', cabinet, name }),
  /** Áp bộ vật liệu cho các tủ hoặc mọi tủ của một phòng (một bước undo). */
  applyMaterialSet: (name: string, ids: ObjectId[], room?: string) => command<{ cabinets: number }>({ cmd: 'apply_material_set', name, ids, room }),
  deleteMaterialSet: (name: string) => command({ cmd: 'delete_material_set', name }),
  /** Tool gia công theo tham số cho nhiều tấm (một undo). */
  toolFeature: (ids: ObjectId[], tool: string, feature: ParamFeature) => command<{ panels: number }>({ cmd: 'tool_feature', ids, tool, feature }),
  getProducts: () => command<{ products: ProductInfo[] }>({ cmd: 'get_products' }),
  /** Chèn mẫu sản phẩm dựng sẵn (một undo), cạnh tủ `after` nếu có. */
  insertProduct: (key: string, after?: ObjectId, size: { width?: number; height?: number; depth?: number } = {}, params: Record<string, string> = {}, room?: string, floor?: string) =>
    command<{ id: ObjectId }>({ cmd: 'insert_product', key, after, ...size, params, room, floor }),
  /** Chạy thử một thao tác (không undo, không đổi dự án): khoang / vị trí tấm chia kết quả. */
  preview: (cabinet: ObjectId, request: Record<string, unknown>) =>
    command<{ ok: boolean; error?: string; bays?: BayInfo[]; positions?: PanelPosition[]; problems?: number[] }>({ cmd: 'preview', cabinet, request }),
  alignObjects: (ids: ObjectId[], mode: 'LEFT' | 'RIGHT' | 'TOP' | 'BOTTOM' | 'BACK' | 'FRONT' | 'CENTER_X' | 'CENTER_Y' | 'CENTER_Z') => command<{ moved: number }>({ cmd: 'align_objects', ids, mode }),
  distributeObjects: (ids: ObjectId[], axis: 'X' | 'Y' | 'Z') => command<{ moved: number }>({ cmd: 'distribute_objects', ids, axis }),
  rotateObjects: (ids: ObjectId[], deg: number, pivot: 'CENTER' | 'LEFT_BACK' | 'RIGHT_BACK' = 'CENTER') => command<{ moved: number }>({ cmd: 'rotate_objects', ids, deg, pivot }),
  snapToWall: (ids: ObjectId[], gap = 0) => command<{ moved: number }>({ cmd: 'snap_to_wall', ids, gap }),
  createRun: (ids: ObjectId[], rules?: Partial<RunRules>) => command<{ name: string }>({ cmd: 'create_run', ids, rules }),
  updateRun: (name: string, rules: RunRules) => command({ cmd: 'update_run', name, rules }),
  deleteRun: (name: string) => command({ cmd: 'delete_run', name }),
  getRuns: () => command<{ runs: RunDef[] }>({ cmd: 'get_runs' }),
  splitZone: (p: { cabinet: ObjectId; zone: number; kind: SplitKind; formula: string; from_end: boolean }) => command<{ uids: number[] }>({ cmd: 'split_zone', ...p }),
  zoneAddPanels: (p: { cabinet: ObjectId; zones: number[]; kind: SplitKind; count: number; thickness?: number; lock: Lock; value: number; tilt_deg?: [number, number] }) =>
    command<{ uids: number[] }>({ cmd: 'zone_add_panels', ...p }),
  zoneAddDoors: (p: { cabinet: ObjectId; zones: number[]; kind: DoorKind; cols: number; rows: number; mount: Mount; hinge: HingeSide; thickness?: number; stop?: StopRailSpec }) =>
    command({ cmd: 'zone_add_doors', ...p }),
  zoneAddDrawers: (p: { cabinet: ObjectId; zones: number[]; count: number; cols: number; mount: Mount; thickness?: number; with_box: boolean; inner?: boolean; false_front?: boolean }) =>
    command({ cmd: 'zone_add_drawers', ...p }),
  zoneAddLink: (cabinet: ObjectId, zones: number[], kind: 'OVAL_RAIL' | 'ACCESSORY' | 'APPLIANCE_BAY', offset = 60, code?: string) =>
    command<{ misfit_zones: number[] }>({ cmd: 'zone_add_link', cabinet, zones, kind, offset, code }),
  /** Catalog phụ kiện khoang; `fits` theo các khoang đang ghim. */
  getAccessories: (cabinet?: ObjectId, zones: number[] = []) => command<{ accessories: AccessoryInfo[] }>({ cmd: 'get_accessories', cabinet, zones }),
  zoneRemove: (cabinet: ObjectId, uid: number) => command({ cmd: 'zone_remove', cabinet, uid }),
  setPartMod: (id: ObjectId, patch: PartModPatch) => command({ cmd: 'set_part_mod', id, patch }),
  saveTemplate: (cabinet: ObjectId, name: string) => command({ cmd: 'save_template', cabinet, name }),
  insertTemplate: (p: { name: string; width?: number; height?: number; depth?: number; room?: string; floor?: string; after?: ObjectId }) =>
    command<{ id: ObjectId }>({ cmd: 'insert_template', ...p }),
  deleteTemplate: (name: string) => command({ cmd: 'delete_template', name }),
  saveRulePreset: (cabinet: ObjectId, name: string) => command({ cmd: 'save_rule_preset', cabinet, name }),
  deleteRulePreset: (name: string) => command({ cmd: 'delete_rule_preset', name }),
  applyRulePreset: (ids: ObjectId[], name: string) => command({ cmd: 'apply_rule_preset', ids, name }),
  arraySplitPanel: (id: ObjectId, count: number) => command({ cmd: 'array_split_panel', id, count }),
  arrayCabinet: (id: ObjectId, count: number, axis: 0 | 1 | 2, gap: number, sizes?: string) => command<{ created?: ObjectId[] }>({ cmd: 'array_cabinet', id, count, axis, gap, sizes }),
  mirrorCabinet: (id: ObjectId) => command({ cmd: 'mirror_cabinet', id }),
  /** Multi-edit: one undo step, all or nothing. */
  setParameterMulti: (ids: ObjectId[], name: string, value: string) => command({ cmd: 'set_parameter_multi', ids, name, value }),
  setBay: (cabinet: ObjectId, zone: number, index: number, mode?: BayMode, value?: number) => command({ cmd: 'set_bay', cabinet, zone, index, mode, value }),
  /** Kéo kích thước tủ với chế độ dãn khoang (KEEP / PROPORTIONAL / EDGE). */
  resizeCabinet: (id: ObjectId, name: 'width' | 'height' | 'depth', value: number, stretch: 'KEEP' | 'PROPORTIONAL' | 'EDGE', edge?: 'START' | 'END') =>
    command({ cmd: 'resize_cabinet', id, name, value, stretch, edge }),
  getStructure: (cabinet: ObjectId) => command<StructureInfo>({ cmd: 'get_structure', cabinet }),
  /** Khoét hậu: thay cả danh sách lỗ (một undo). */
  setBackCutouts: (cabinet: ObjectId, cutouts: BackCutout[]) => command({ cmd: 'set_back_cutouts', cabinet, cutouts }),
  saveGroupPreset: (cabinet: ObjectId, group: string, name: string) => command({ cmd: 'save_group_preset', cabinet, group, name }),
  applyGroupPreset: (ids: ObjectId[], group: string, name: string) => command({ cmd: 'apply_group_preset', ids, group, name }),
  deleteGroupPreset: (group: string, name: string) => command({ cmd: 'delete_group_preset', group, name }),
  saveZonePreset: (cabinet: ObjectId, zone: number, name: string) => command({ cmd: 'save_zone_preset', cabinet, zone, name }),
  applyZonePreset: (cabinet: ObjectId, zones: number[], name: string) => command({ cmd: 'apply_zone_preset', cabinet, zones, name }),
  deleteZonePreset: (name: string) => command({ cmd: 'delete_zone_preset', name }),
  setDrawerHeight: (cabinet: ObjectId, uid: number, index: number, mode?: BayMode, value?: number) => command({ cmd: 'set_drawer_height', cabinet, uid, index, mode, value }),
  moveDrawerDivider: (cabinet: ObjectId, uid: number, index: number, before: number) => command({ cmd: 'move_drawer_divider', cabinet, uid, index, before }),
  equalizeSplit: (cabinet: ObjectId, zone: number) => command({ cmd: 'equalize_split', cabinet, zone }),
  /** Kéo vách / kệ: `before` = clear size of the bay before the panel. */
  moveSplitPanel: (id: ObjectId, before: number) => command<{ zone: number }>({ cmd: 'move_split_panel', id, before }),
  shapeTool: (ids: ObjectId[], op: ShapeOp) => command<{ changed: number; skipped: { id: ObjectId; reason: string }[] }>({ cmd: 'shape_tool', ids, op }),
  setRelation: (a: ObjectId, b: ObjectId, kind: RelationKind, gap = 0) => command({ cmd: 'set_relation', a, b, kind, gap }),
  /** Kéo cạnh tấm: `delta` > 0 grows that side (cabinet frame). */
  resizePanelSide: (id: ObjectId, side: PanelSide, delta: number, constrained: boolean) => command({ cmd: 'resize_panel_side', id, side, delta, constrained }),
  mergePanels: (ids: ObjectId[]) => command<{ id: ObjectId; size: [number, number] }>({ cmd: 'merge_panels', ids }),
  setPrice: (key: string, value: number | null) => command({ cmd: 'set_price', key, value }),
  createPanel: (p: { name?: string; width: number; height: number; thickness: number; material?: string; transform?: Transform3D; parent?: ObjectId }) =>
    command<{ id: ObjectId }>({ cmd: 'create_panel', ...p }),
  deleteObjects: (ids: ObjectId[]) => command({ cmd: 'delete_objects', ids }),
  duplicateObjects: (ids: ObjectId[]) => command<{ created: ObjectId[] }>({ cmd: 'duplicate_objects', ids }),
  setParameter: (id: ObjectId, name: string, value: string) => command({ cmd: 'set_parameter', id, name, value }),
  setTransform: (id: ObjectId, transform: Transform3D) => command({ cmd: 'set_transform', id, transform }),
  /** Di chuyển khối theo vector (mm, thế giới); một bước undo. */
  moveObjects: (ids: ObjectId[], delta: Vec3) => command({ cmd: 'move_objects', ids, delta }),
  setName: (id: ObjectId, name: string) => command({ cmd: 'set_name', id, name }),
  setVisible: (ids: ObjectId[], visible: boolean) => command({ cmd: 'set_visible', ids, visible }),
  setLocked: (ids: ObjectId[], locked: boolean) => command({ cmd: 'set_locked', ids, locked }),
  reparent: (id: ObjectId, parent: ObjectId | null, index?: number) => command({ cmd: 'reparent', id, parent, index }),
  setMaterial: (id: ObjectId, material: string, slot?: 'carcass' | 'front' | 'back') => command({ cmd: 'set_material', id, material, slot }),
  setEdgeBand: (id: ObjectId, edge: EdgeSide, enabled: boolean) => command({ cmd: 'set_edge_band', id, edge, enabled }),
  addFeature: (id: ObjectId, feature: MachiningFeature) => command({ cmd: 'add_feature', id, feature }),
  removeFeature: (id: ObjectId, index: number) => command({ cmd: 'remove_feature', id, index }),
  undo: () => command<{ label: string }>({ cmd: 'undo' }),
  redo: () => command<{ label: string }>({ cmd: 'redo' }),
};

/** Primitive bản vẽ in (mm giấy, gốc trên-trái). */
export type SheetItem =
  | { t: 'rect'; x: number; y: number; w: number; h: number; cls: string }
  | { t: 'line'; x1: number; y1: number; x2: number; y2: number; cls: string }
  | { t: 'text'; x: number; y: number; s: string; size: number; anchor: 'start' | 'middle' | 'end'; cls: string; rotate?: number; value?: number };

export interface DrawingSheet {
  scale: string;
  sheets: { paper: [number, number]; scale: string; views: { title: string; x: number; y: number; w: number; h: number }[]; items: SheetItem[] }[];
}

/** Mẫu dựng sẵn (D30): kích thước mặc định + tham số hiển thị. */
export interface ProductInfo {
  key: string;
  name: string;
  room: string;
  summary: string;
  size: [number, number, number];
  params: { key: string; label: string; kind: 'number' | 'bool' | 'select'; default: string; options: { value: string; label: string }[] }[];
}
