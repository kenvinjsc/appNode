// UI actions → commands. Shared by ribbon, shortcuts and context menus.
import { Commands } from '../core-api/commands';
import { Queries } from '../core-api/queries';
import type { ObjectId } from '../core-api/types';
import { View } from '../viewport/viewportBus';
import { findNode, useUi } from './uiStore';

const ui = () => useUi.getState();
const safe = (p: Promise<unknown>) => p.catch(() => undefined);

export const Actions = {
  async newProject() {
    await safe(Commands.createProject('Dự án mới'));
  },
  async sampleProject() {
    await safe(Commands.createProject('Dự án mẫu · Phòng ngủ & bếp'));
    await safe(Commands.createRoom(4800, 3600, 2700));
    const a = await Commands.createCabinet('WARDROBE', [100, 0, 20], { width: 1200 }).catch(() => null);
    await safe(Commands.createCabinet('WARDROBE', [1300, 0, 20], { width: 800, doors: 1 }));
    await safe(Commands.createCabinet('DRAWER', [2150, 0, 20], { width: 600 }));
    await safe(Commands.createCabinet('BASE', [2800, 0, 20], { width: 900 }));
    await safe(Commands.createCabinet('WALL', [2800, 1450, 20], { width: 900 }));
    await safe(Commands.createCabinet('OPEN_SHELF', [3750, 0, 20], { width: 800 }));
    if (a) ui().select([a.id]);
    setTimeout(() => View.set('iso'), 50);
  },
  async save() {
    try {
      const project = await Queries.saveProject();
      const name = (project as { metadata?: { name?: string } }).metadata?.name ?? 'du-an';
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([JSON.stringify(project, null, 2)], { type: 'application/json' }));
      a.download = `${name.replace(/[^\p{L}\p{N}]+/gu, '-')}.aic.json`;
      a.click();
      ui().toast({ kind: 'success', title: 'Đã lưu dự án' });
    } catch {
      ui().toast({ kind: 'error', title: 'Không thể lưu dự án.' });
    }
  },
  open() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,.aic.json';
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return;
      try {
        const json = JSON.parse(await f.text());
        await Commands.loadProject(json);
        setTimeout(() => View.fitAll(), 100);
      } catch {
        ui().toast({ kind: 'error', title: 'Không mở được dự án.', detail: 'Tệp không đúng định dạng AIC.' });
      }
    };
    input.click();
  },
  undo: () => safe(Commands.undo()),
  redo: () => safe(Commands.redo()),
  async delete(ids = ui().selection) {
    if (!ids.length) return;
    await safe(Commands.deleteObjects(ids));
  },
  async duplicate(ids = ui().selection) {
    if (!ids.length) return;
    const r = await Commands.duplicateObjects(ids).catch(() => null);
    if (r?.created.length) ui().select(r.created);
  },
  async toggleHidden(ids = ui().selection) {
    const t = ui().tree;
    if (!ids.length) return;
    const anyVisible = ids.some((id) => findNode(t, id)?.node.visible);
    await safe(Commands.setVisible(ids, !anyVisible));
  },
  async toggleLocked(ids = ui().selection) {
    const t = ui().tree;
    if (!ids.length) return;
    const anyUnlocked = ids.some((id) => !findNode(t, id)?.node.locked);
    await safe(Commands.setLocked(ids, anyUnlocked));
  },
  selectParent(id: ObjectId) {
    const p = findNode(ui().tree, id)?.parent;
    if (p != null) ui().select([p]);
  },
  selectChildren(id: ObjectId) {
    const n = findNode(ui().tree, id)?.node;
    if (n?.children.length) ui().select(n.children.map((c) => c.id));
  },
  selectAll() {
    const t = ui().tree;
    if (t) ui().select(t.roots.map((r) => r.id));
  },
  /** Cabinet of the selection (or the selection itself when it is a cabinet). */
  selectedCabinet(): ObjectId | null {
    const s = ui();
    for (const id of s.selection) {
      const f = findNode(s.tree, id);
      if (f?.node.kind === 'CABINET') return id;
      if (f?.parent != null && findNode(s.tree, f.parent)?.node.kind === 'CABINET') return f.parent;
    }
    return null;
  },
  async bumpCabinet(param: 'shelves' | 'doors' | 'drawers', delta: number) {
    const cab = Actions.selectedCabinet();
    if (cab === null) {
      ui().toast({ kind: 'info', title: 'Chọn một tủ trước.', detail: 'Thao tác này thêm thành phần vào tủ đang chọn.' });
      return;
    }
    const sheet = await Queries.properties(cab).catch(() => null);
    const cur = Number(sheet?.groups.flatMap((g) => g.fields).find((f) => f.key === param)?.value ?? 0);
    await safe(Commands.setParameter(cab, param, String(Math.max(0, cur + delta))));
  },
  async addDivider() {
    const cab = Actions.selectedCabinet();
    if (cab === null) {
      ui().toast({ kind: 'info', title: 'Chọn một tủ trước.' });
      return;
    }
    const sheet = await Queries.properties(cab).catch(() => null);
    const v = (k: string) => Number(sheet?.groups.flatMap((g) => g.fields).find((f) => f.key === k)?.value ?? 0);
    const t = v('thickness') || 18;
    const w = v('width');
    const h = v('height');
    const d = v('depth');
    const plinth = v('plinth_height');
    const r = await Commands.createPanel({
      name: 'Vách ngăn',
      width: d - 25,
      height: h - plinth - 2 * t,
      thickness: t,
      transform: { translation: [w / 2 - t / 2, plinth + t, d], rotation_deg: [0, 90, 0] },
      parent: cab,
    }).catch(() => null);
    if (r) {
      // Keep the divider parametric: follow the cabinet's inner height/depth.
      await safe(Commands.setParameter(r.id, 'height', '= cabinet.inner_height'));
      await safe(Commands.setParameter(r.id, 'width', '= cabinet.inner_depth'));
      await safe(Commands.setParameter(r.id, 'x', '= cabinet.width / 2 - cabinet.thickness / 2'));
      ui().select([r.id]);
    }
  },
  async addFreePanel() {
    const c = ui().cursor ?? [0, 0, 0];
    const r = await Commands.createPanel({ name: 'Tấm', width: 600, height: 800, thickness: 18, transform: { translation: [Math.round(c[0]), 0, Math.round(c[2])], rotation_deg: [0, 0, 0] } }).catch(() => null);
    if (r) ui().select([r.id]);
  },
  async addRoom() {
    await safe(Commands.createRoom(4000, 3000, 2700));
  },
  inspectRelations(id?: ObjectId) {
    if (id !== undefined) ui().select([id]);
    ui().set({ showRelations: true });
  },
  openManufacturing(id: ObjectId) {
    const n = findNode(ui().tree, id)?.node;
    if (n?.kind === 'PANEL') ui().set({ workspace: 'manufacturing', mfgPanel: id });
    else ui().set({ workspace: 'manufacturing' });
  },
};
