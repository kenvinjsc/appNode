// Commands: every mutation goes UI action → command → core → events.
import { send } from './transport';
import { emitCoreEvents, emitStatus } from './events';
import { describeError, type UserMessage } from './errors';
import type { CabinetKind, EdgeSide, MachiningFeature, ObjectId, Transform3D, Vec3 } from './types';

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
  createCabinet: (kind: CabinetKind, position: Vec3, overrides: Record<string, unknown> = {}, parent?: ObjectId) =>
    command<{ id: ObjectId }>({ cmd: 'create_cabinet', kind, position, overrides, parent }),
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
