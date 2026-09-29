// Queries: read models from the core. Never mutate.
import { send } from './transport';
import { describeError } from './errors';
import { CommandError } from './commands';
import type {
  AssemblyRelation, Bounds, CncProgram, Costing, ZonesInfo, FlatPanel, Material, NestingJobResult, NestingPlacement, NestingSettings,
  ObjectId, PartsReport, PropertySheet, RenderBatch, SceneTree, SnapResult, Vec3, TemplatesInfo } from './types';

async function query<T>(req: Record<string, unknown>, signal?: AbortSignal): Promise<T> {
  const r = await send<T>(req, signal);
  if (!r.ok || r.error) {
    throw new CommandError(describeError(r.error!), r.error!.code);
  }
  return r.result;
}

export const Queries = {
  sceneTree: () => query<SceneTree>({ cmd: 'get_scene_tree' }),
  properties: (id: ObjectId) => query<PropertySheet>({ cmd: 'get_properties', id }),
  templates: () => query<TemplatesInfo>({ cmd: 'get_templates' }),
  propertiesMulti: (ids: ObjectId[]) => query<PropertySheet>({ cmd: 'get_properties_multi', ids }),
  renderObjects: (ids: ObjectId[] | null, knownKeys: string[]) =>
    query<RenderBatch>({ cmd: 'get_render_objects', ids, known_keys: knownKeys }),
  materials: () => query<Material[]>({ cmd: 'get_materials' }),
  relations: (id?: ObjectId) => query<{ relations: AssemblyRelation[]; names: Record<string, string> }>({ cmd: 'get_relations', id }),
  manufacturing: (id: ObjectId) => query<FlatPanel>({ cmd: 'get_manufacturing', id }),
  parts: () => query<PartsReport>({ cmd: 'get_parts' }),
  nesting: (material: string | null, settings: NestingSettings, signal?: AbortSignal) =>
    query<{ jobs: NestingJobResult[] }>({ cmd: 'run_nesting', material, settings }, signal),
  cnc: (material: string, sheetId: number, signal?: AbortSignal) =>
    query<{ program: CncProgram; placements: NestingPlacement[]; sheet_count: number }>({ cmd: 'generate_cnc', material, sheet_id: sheetId }, signal),
  snap: (id: ObjectId, delta: Vec3, grid?: number) => query<SnapResult>({ cmd: 'snap', id, delta, grid }),
  bounds: (ids: ObjectId[]) => query<Bounds | null>({ cmd: 'get_bounds', ids }),
  saveProject: () => query<unknown>({ cmd: 'save_project' }),
  zones: (cabinet: ObjectId) => query<ZonesInfo>({ cmd: 'get_zones', cabinet }),
  costing: () => query<Costing>({ cmd: 'get_costing' }),
  status: () => query<{ name: string; revision: number; objects: number; panels: number; undo: string | null; redo: string | null }>({ cmd: 'get_status' }),
};

export interface TransformInfo {
  local: import('./types').Transform3D;
  world: import('./types').Transform3D;
  parent_world: import('./types').Transform3D;
  world_matrix: number[];
  parent_matrix: number[];
}

export const transformOf = (id: ObjectId) => query<TransformInfo>({ cmd: 'get_transform', id });
