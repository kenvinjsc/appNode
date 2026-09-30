// UI state only. Project data (geometry, parameters, relations) stays in the
// core; the tree/materials below are read-model caches refreshed from events.
import { create } from 'zustand';
import type { CabinetKind, ContactType, Material, ObjectId, SceneTree } from '../core-api/types';

/** Hộp thoại Chia khoang (giống Chia ngang / Chia dọc của plugin). */
export interface SplitToolState {
  /** H = chia ngang (tầng trên / dưới), V = chia dọc (khoang trái / phải). */
  dir: 'H' | 'V';
  formula: string;
  /** Tính từ trên xuống (ngang) / từ phải sang trái (dọc). */
  fromEnd: boolean;
  /** Tạo tấm: kệ / vách; No = chỉ chia khoang (gắn cánh, ngăn kéo từng phần). */
  panel: boolean;
  /** Đợt di động (chia ngang). */
  adjustable: boolean;
}

export type Workspace = 'design' | 'manufacturing' | 'nesting' | 'cnc';
export type Tool =
  | { type: 'select' }
  | { type: 'move' }
  | { type: 'rotate' }
  | { type: 'place-cabinet'; kind: CabinetKind }
  | { type: 'place-panel'; preset: 'panel' | 'shelf' | 'divider' | 'door' };
export type SelectionMode = 'object' | 'face' | 'edge';
export type RibbonTab = 'file' | 'home' | 'design' | 'edit' | 'material' | 'manufacturing' | 'view';
export type Drawer = null | 'materials' | 'report' | 'shortcuts';
export type DesignerTab = 'frame' | 'create' | 'edit' | 'manage' | 'library' | 'settings';
export type CreateType = 'horizontal' | 'vertical' | 'back' | 'door' | 'drawer' | 'link';

export interface Toast {
  id: number;
  kind: 'error' | 'info' | 'success';
  title: string;
  detail?: string;
}

export interface Job {
  id: number;
  label: string;
  status: string;
  progress: number | null; // null = indeterminate
  cancel?: () => void;
}

export interface FaceSelection {
  id: ObjectId;
  faceId: number;
}

interface UiState {
  workspace: Workspace;
  ribbonTab: RibbonTab;
  tool: Tool;
  selectionMode: SelectionMode;
  selection: ObjectId[];
  active: ObjectId | null;
  face: FaceSelection | null;
  edge: { id: ObjectId; edgeId: number } | null;
  hovered: ObjectId | null;
  panels: { tree: boolean; properties: boolean; drawing: boolean };
  drawer: Drawer;
  projection: 'perspective' | 'orthographic';
  showRelations: boolean;
  relationFilter: Record<ContactType, boolean>;
  showDimensions: boolean;
  snap: boolean;
  gridSize: number;
  section: boolean;
  canUndo: boolean;
  canRedo: boolean;
  revision: number;
  tree: SceneTree | null;
  materials: Material[];
  toasts: Toast[];
  jobs: Job[];
  contextMenu: { x: number; y: number; id: ObjectId } | null;
  /** Quick build menu on a zone (right-click in "Tạo tấm"). */
  zoneMenu: { x: number; y: number; cabinet: ObjectId; zone: number } | null;
  /** Công cụ Chia khoang: đang mở thì bấm vào khoang (3D / 2D) để chia theo công thức. */
  splitTool: SplitToolState | null;
  /** Small input dialog (tên template, số lượng…). */
  prompt: { title: string; label: string; value: string; ok: (v: string) => void } | null;
  /** 2D edge handles: keep constraints (anchored edges change their gap) or free. */
  resizeMode: 'free' | 'constrained';
  /** Chế độ dãn khoang khi kéo kích thước tủ. */
  stretchMode: 'KEEP' | 'PROPORTIONAL' | 'EDGE';
  /** Bảng Thuộc tính kết cấu đang mở cho tủ này. */
  structureOf: ObjectId | null;
  renaming: ObjectId | null;
  cursor: [number, number, number] | null;
  mfgPanel: ObjectId | null;
  propertiesTab: 'params' | 'material' | 'machining';
  designerTab: DesignerTab;
  createType: CreateType;
  /** Zones pinned for "Tạo tấm" (ghim vùng). */
  pinned: { cabinet: ObjectId | null; zones: number[] };
  leftTab: 'tree' | 'tools';
  toolId: string | null;
  isolate: boolean;
  /** Room tab of the project (null = Tất cả). New cabinets go into it. */
  activeRoom: string | null;
  /** Floor tab (tầng) of the project (null = Tất cả tầng). Rooms are listed per floor. */
  activeFloor: string | null;
  extraFloors: string[];
  /** Rooms added by the user that have no cabinet yet. */
  extraRooms: string[];
  showZones: boolean;
  /** Action executed by the TAB key for the open panel. */
  tabAction: (() => void) | null;
  /** Contextual red hint in the viewport corner. */
  hint: string | null;

  set: (p: Partial<UiState>) => void;
  select: (ids: ObjectId[], mode?: 'replace' | 'toggle' | 'add') => void;
  clearSelection: () => void;
  setTool: (t: Tool) => void;
  toast: (t: Omit<Toast, 'id'>) => void;
  dismissToast: (id: number) => void;
  startJob: (label: string, cancel?: () => void) => number;
  updateJob: (id: number, p: Partial<Job>) => void;
  endJob: (id: number) => void;
}

let seq = 1;

export const useUi = create<UiState>((set, get) => ({
  workspace: 'design',
  ribbonTab: 'home',
  tool: { type: 'select' },
  selectionMode: 'object',
  selection: [],
  active: null,
  face: null,
  edge: null,
  hovered: null,
  panels: { tree: true, properties: true, drawing: true },
  drawer: null,
  projection: 'perspective',
  showRelations: false,
  relationFilter: { TOUCH: true, GAP: true, PENETRATE: true },
  showDimensions: true,
  snap: true,
  gridSize: 10,
  section: false,
  canUndo: false,
  canRedo: false,
  revision: 0,
  tree: null,
  materials: [],
  toasts: [],
  jobs: [],
  contextMenu: null,
  zoneMenu: null,
  prompt: null,
  splitTool: null,
  resizeMode: 'constrained',
  stretchMode: 'PROPORTIONAL',
  structureOf: null,
  renaming: null,
  cursor: null,
  mfgPanel: null,
  propertiesTab: 'params',
  designerTab: 'frame',
  createType: 'horizontal',
  pinned: { cabinet: null, zones: [] },
  leftTab: 'tree',
  toolId: null,
  isolate: false,
  activeRoom: null,
  activeFloor: null,
  extraFloors: [],
  extraRooms: [],
  showZones: true,
  tabAction: null,
  hint: null,

  set: (p) => set(p),
  select: (ids, mode = 'replace') => {
    const cur = get().selection;
    let next: ObjectId[];
    if (mode === 'replace') next = ids;
    else if (mode === 'add') next = Array.from(new Set([...cur, ...ids]));
    else {
      const s = new Set(cur);
      for (const id of ids) {
        if (s.has(id)) s.delete(id);
        else s.add(id);
      }
      next = Array.from(s);
    }
    const active = next.length ? (ids.find((i) => next.includes(i)) ?? next[next.length - 1]) : null;
    set({ selection: next, active, face: null, edge: null });
  },
  clearSelection: () => set({ selection: [], active: null, face: null, edge: null }),
  setTool: (t) => set({ tool: t }),
  toast: (t) => {
    const id = seq++;
    set({ toasts: [...get().toasts, { ...t, id }] });
    setTimeout(() => get().dismissToast(id), t.kind === 'error' ? 6000 : 3000);
  },
  dismissToast: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
  startJob: (label, cancel) => {
    const id = seq++;
    set({ jobs: [...get().jobs, { id, label, status: 'Đang xử lý…', progress: null, cancel }] });
    return id;
  },
  updateJob: (id, p) => set({ jobs: get().jobs.map((j) => (j.id === id ? { ...j, ...p } : j)) }),
  endJob: (id) => set({ jobs: get().jobs.filter((j) => j.id !== id) }),
}));

/** Find a tree node by id (read-model helper). */
export function findNode(tree: SceneTree | null, id: ObjectId) {
  if (!tree) return null;
  const stack = [...tree.roots];
  const parents = new Map<number, number | null>();
  for (const r of tree.roots) parents.set(r.id, null);
  while (stack.length) {
    const n = stack.pop()!;
    if (n.id === id) return { node: n, parent: parents.get(n.id) ?? null };
    for (const c of n.children) {
      parents.set(c.id, n.id);
      stack.push(c);
    }
  }
  return null;
}
