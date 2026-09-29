// Commands: every mutation goes UI action → command → core → events.
import { send } from './transport';
import { emitCoreEvents, emitStatus } from './events';
import { describeError, type UserMessage } from './errors';
import type { CabinetKind, DoorKind, EdgeSide, HingeSide, Lock, MachiningFeature, Mount, ObjectId, PartModPatch, SplitKind, StopRailSpec, Transform3D, Vec3, ShapeOp, BayMode } from './types';

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
  zoneAddPanels: (p: { cabinet: ObjectId; zones: number[]; kind: SplitKind; count: number; thickness?: number; lock: Lock; value: number; tilt_deg?: [number, number] }) =>
    command<{ uids: number[] }>({ cmd: 'zone_add_panels', ...p }),
  zoneAddDoors: (p: { cabinet: ObjectId; zones: number[]; kind: DoorKind; cols: number; rows: number; mount: Mount; hinge: HingeSide; thickness?: number; stop?: StopRailSpec }) =>
    command({ cmd: 'zone_add_doors', ...p }),
  zoneAddDrawers: (p: { cabinet: ObjectId; zones: number[]; count: number; cols: number; mount: Mount; thickness?: number; with_box: boolean }) =>
    command({ cmd: 'zone_add_drawers', ...p }),
  zoneAddLink: (cabinet: ObjectId, zones: number[], kind: 'OVAL_RAIL', offset = 60) => command({ cmd: 'zone_add_link', cabinet, zones, kind, offset }),
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
  arrayCabinet: (id: ObjectId, count: number, axis: 0 | 1 | 2, gap: number) => command({ cmd: 'array_cabinet', id, count, axis, gap }),
  mirrorCabinet: (id: ObjectId) => command({ cmd: 'mirror_cabinet', id }),
  /** Multi-edit: one undo step, all or nothing. */
  setParameterMulti: (ids: ObjectId[], name: string, value: string) => command({ cmd: 'set_parameter_multi', ids, name, value }),
  setBay: (cabinet: ObjectId, zone: number, index: number, mode?: BayMode, value?: number) => command({ cmd: 'set_bay', cabinet, zone, index, mode, value }),
  equalizeSplit: (cabinet: ObjectId, zone: number) => command({ cmd: 'equalize_split', cabinet, zone }),
  /** Kéo vách / kệ: `before` = clear size of the bay before the panel. */
  moveSplitPanel: (id: ObjectId, before: number) => command<{ zone: number }>({ cmd: 'move_split_panel', id, before }),
  shapeTool: (ids: ObjectId[], op: ShapeOp) => command<{ changed: number; skipped: { id: ObjectId; reason: string }[] }>({ cmd: 'shape_tool', ids, op }),
  mergePanels: (ids: ObjectId[]) => command<{ id: ObjectId; size: [number, number] }>({ cmd: 'merge_panels', ids }),
  setPrice: (key: string, value: number | null) => command({ cmd: 'set_price', key, value }),
  createPanel: (p: { name?: string; width: number; height: number; thickness: number; material?: string; transform?: Transform3D; parent?: ObjectId }) =>
    command<{ id: ObjectId }>({ cmd: 'create_panel', ...p }),
  deleteObjects: (ids: ObjectId[]) => command({ cmd: 'delete_objects', ids }),
  duplicateObjects: (ids: ObjectId[]) => command<{ created: ObjectId[] }>({ cmd: 'duplicate_objects', ids }),
  setParameter: (id: ObjectId, name: string, value: string) => command({ cmd: 'set_parameter', id, name, value }),
  setTransform: (id: ObjectId, transform: Transform3D) => command({ cmd: 'set_transform', id, transform }),
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
