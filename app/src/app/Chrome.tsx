// Title bar, workspace rail, status bar, toasts, jobs, context menu.
import { useEffect, useState } from 'react';
import { Commands } from '../core-api/commands';
import { Queries } from '../core-api/queries';
import { Icon } from '../shared/icons';
import { CONTACT_LABEL, fmt } from '../shared/i18n';
import { View } from '../viewport/viewportBus';
import { Actions } from './actions';
import type { ObjectId } from '../core-api/types';
import { findNode, useUi, type Workspace } from './uiStore';

export function TitleBar() {
  const { tree, canUndo, canRedo } = useUi();
  return (
    <div className="titlebar">
      <div className="brand">
        <Icon name="logo" size={22} style={{ color: 'var(--accent)' }} />
        <b>AIC CAD</b>
      </div>
      <div className="sep" />
      <div className="project-name">Dự án: {tree?.name ?? '…'}</div>
      <div className="spacer" />
      <button className="tb-btn" onClick={() => void Actions.save()} title="Lưu (Ctrl+S)">
        <Icon name="save" size={17} /> Lưu
      </button>
      <button className="tb-btn" disabled={!canUndo} onClick={() => void Actions.undo()} title="Hoàn tác (Ctrl+Z)">
        <Icon name="undo" size={17} />
      </button>
      <button className="tb-btn" disabled={!canRedo} onClick={() => void Actions.redo()} title="Làm lại (Ctrl+Y)">
        <Icon name="redo" size={17} />
      </button>
    </div>
  );
}

export function WorkspaceRail() {
  const { workspace, set, drawer } = useUi();
  const items: [Workspace, string, string][] = [
    ['design', 'box3d', 'Thiết kế'],
    ['manufacturing', 'drill', 'Gia công'],
    ['nesting', 'nesting', 'Xếp tấm'],
    ['cnc', 'cnc', 'CNC'],
  ];
  return (
    <nav className="rail">
      {items.map(([k, icon, label]) => (
        <button key={k} className={workspace === k ? 'active' : ''} onClick={() => set({ workspace: k })}>
          <Icon name={icon} size={22} />
          <span>{label}</span>
        </button>
      ))}
      <div className="rail-sep" />
      <button className={drawer === 'materials' ? 'active' : ''} onClick={() => set({ drawer: drawer === 'materials' ? null : 'materials' })}>
        <Icon name="material" size={22} />
        <span>Vật liệu</span>
      </button>
      <button className={drawer === 'report' ? 'active' : ''} onClick={() => set({ drawer: drawer === 'report' ? null : 'report' })}>
        <Icon name="report" size={22} />
        <span>Báo cáo</span>
      </button>
    </nav>
  );
}

export function StatusBar() {
  const { selection, tree, cursor, jobs, gridSize, set, snap, active, revision, showRelations, relationFilter, workspace } = useUi();
  const [status, setStatus] = useState<{ panels: number } | null>(null);
  const [bounds, setBounds] = useState<string | null>(null);
  useEffect(() => {
    Queries.status().then(setStatus).catch(() => undefined);
  }, [revision]);
  useEffect(() => {
    if (!selection.length) return setBounds(null);
    Queries.bounds(selection)
      .then((b) => setBounds(b ? `${fmt(b.max[0] - b.min[0], 0)} × ${fmt(b.max[1] - b.min[1], 0)} × ${fmt(b.max[2] - b.min[2], 0)} mm` : null))
      .catch(() => setBounds(null));
  }, [selection, revision]);
  const act = active !== null ? findNode(tree, active)?.node : null;
  return (
    <div className="statusbar">
      <span className={`dot ${jobs.length ? 'busy' : ''}`} />
      <span>{jobs.length ? jobs[0].label + '…' : 'Sẵn sàng'}</span>
      <span className="sep" />
      <span>Tổng: {status?.panels ?? 0} tấm</span>
      <span className="sep" />
      <span>
        {selection.length ? `Chọn: ${selection.length === 1 ? act?.name : selection.length + ' đối tượng'}` : 'Chưa chọn'}
        {bounds ? ` · ${bounds}` : ''}
      </span>
      {workspace === 'design' && showRelations && (
        <>
          <span className="sep" />
          <span className="filters">
            {(['TOUCH', 'GAP', 'PENETRATE'] as const).map((c) => (
              <label key={c} className={`rel-chip ${c.toLowerCase()} ${relationFilter[c] ? '' : 'off'}`}>
                <input type="checkbox" checked={relationFilter[c]} onChange={() => set({ relationFilter: { ...relationFilter, [c]: !relationFilter[c] } })} />
                {CONTACT_LABEL[c]}
              </label>
            ))}
          </span>
        </>
      )}
      <div className="spacer" />
      {cursor && (
        <span className="mono">
          X {fmt(cursor[0], 0)} · Y {fmt(cursor[1], 0)} · Z {fmt(cursor[2], 0)}
        </span>
      )}
      <span className="sep" />
      <button className={`sb-btn ${snap ? 'on' : ''}`} onClick={() => set({ snap: !snap })} title="Bắt dính">
        <Icon name="magnet" size={14} /> Bắt dính
      </button>
      <label className="sb-grid">
        Lưới
        <select value={gridSize} onChange={(e) => set({ gridSize: Number(e.target.value) })}>
          {[1, 5, 10, 50, 100].map((g) => (
            <option key={g} value={g}>
              {g} mm
            </option>
          ))}
        </select>
      </label>
      <button className="sb-btn" onClick={() => View.fitAll()} title="Phóng vừa">
        <Icon name="fit" size={14} />
      </button>
    </div>
  );
}

export function Toasts() {
  const { toasts, dismissToast } = useUi();
  return (
    <div className="toasts">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`} onClick={() => dismissToast(t.id)}>
          <Icon name={t.kind === 'error' ? 'warning' : t.kind === 'success' ? 'check' : 'info'} size={18} />
          <div>
            <b>{t.title}</b>
            {t.detail && <p>{t.detail}</p>}
          </div>
        </div>
      ))}
    </div>
  );
}

export function Jobs() {
  const { jobs } = useUi();
  const [show, setShow] = useState(false);
  // Only surface the overlay for jobs lasting > 250 ms.
  useEffect(() => {
    if (!jobs.length) return setShow(false);
    const t = setTimeout(() => setShow(true), 250);
    return () => clearTimeout(t);
  }, [jobs.length]);
  if (!show || !jobs.length) return null;
  return (
    <div className="jobs">
      {jobs.map((j) => (
        <div key={j.id} className="job">
          <div className="job-top">
            <b>{j.label}</b>
            <span>{j.status}</span>
            {j.cancel && (
              <button className="btn ghost" onClick={j.cancel}>
                Hủy
              </button>
            )}
          </div>
          <div className={`progress ${j.progress === null ? 'indet' : ''}`}>
            <i style={{ width: j.progress === null ? undefined : `${j.progress * 100}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}

/** Quick build menu for a zone: common parts in one click, with sensible options. */
export function ZoneMenu() {
  const { zoneMenu, set } = useUi();
  useEffect(() => {
    if (!zoneMenu) return;
    const close = () => set({ zoneMenu: null });
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  }, [zoneMenu, set]);
  if (!zoneMenu) return null;
  const { cabinet, zone } = zoneMenu;
  const zones = [zone];
  const run = (p: Promise<unknown>) => {
    set({ zoneMenu: null });
    void p.catch(() => undefined);
  };
  const Item = ({ label, fn }: { label: string; fn: () => Promise<unknown> }) => (
    <button onPointerDown={(e) => e.stopPropagation()} onClick={() => run(fn())}>
      <span>{label}</span>
    </button>
  );
  const shelves = (n: number, kind: 'SHELF_ADJUSTABLE' | 'SHELF_FIXED' = 'SHELF_ADJUSTABLE') =>
    Commands.zoneAddPanels({ cabinet, zones, kind, count: n, lock: n > 1 ? 'EVEN' : 'RATIO', value: 50 });
  const dividers = (n: number) => Commands.zoneAddPanels({ cabinet, zones, kind: 'DIVIDER', count: n, lock: n > 1 ? 'EVEN' : 'RATIO', value: 50 });
  const door = (kind: 'SINGLE' | 'DOUBLE' | 'SLIDING', hinge: 'LEFT' | 'RIGHT' | 'TOP' = 'LEFT', mount: 'OVERLAY' | 'INSET' = 'OVERLAY', cols = kind === 'SINGLE' ? 1 : 2) =>
    Commands.zoneAddDoors({ cabinet, zones, kind, cols, rows: 1, mount, hinge });
  const drawers = (n: number, mount: 'OVERLAY' | 'INSET' = 'OVERLAY') => Commands.zoneAddDrawers({ cabinet, zones, count: n, cols: 1, mount, with_box: true });
  return (
    <div className="ctx zone-menu" style={{ left: Math.min(zoneMenu.x, window.innerWidth - 440), top: Math.max(8, Math.min(zoneMenu.y, window.innerHeight - 600)) }} onPointerDown={(e) => e.stopPropagation()}>
      <div className="ctx-title">Dựng nhanh · vùng #{zone}</div>
      <div className="zm-grid">
        <div>
          <h5>Tấm ngang</h5>
          {[1, 2, 3, 4, 5].map((n) => (
            <Item key={n} label={`Kệ di động × ${n}`} fn={() => shelves(n)} />
          ))}
          <Item label="Kệ cố định giữa" fn={() => shelves(1, 'SHELF_FIXED')} />
          <h5>Tấm đứng / hậu</h5>
          <Item label="Hông giữa (2 khoang)" fn={() => dividers(1)} />
          <Item label="Chia 3 khoang" fn={() => dividers(2)} />
          <Item label="Hậu phụ" fn={() => Commands.zoneAddPanels({ cabinet, zones, kind: 'BACK_SUB', count: 1, lock: 'FROM_START', value: 0 })} />
          <h5>Chia khoang</h5>
          {[2, 3, 4].map((n) => (
            <Item key={`h${n}`} label={`Chia ngang ${n} khoang (vách)`} fn={() => dividers(n - 1)} />
          ))}
          {[2, 3, 4].map((n) => (
            <Item key={`v${n}`} label={`Chia dọc ${n} tầng (kệ cố định)`} fn={() => shelves(n - 1, 'SHELF_FIXED')} />
          ))}
          <Item
            label="Chia đều lại khoang cha"
            fn={async () => {
              const info = await Queries.zones(cabinet);
              const bay = info.bays.find((b) => b.child === zone);
              if (bay) await Commands.equalizeSplit(cabinet, bay.zone);
            }}
          />
        </div>
        <div>
          <h5>Cánh</h5>
          <Item label="Cánh đơn · lề trái" fn={() => door('SINGLE', 'LEFT')} />
          <Item label="Cánh đơn · lề phải" fn={() => door('SINGLE', 'RIGHT')} />
          <Item label="Cánh đôi (phủ bì)" fn={() => door('DOUBLE')} />
          <Item label="Cánh đôi (lọt lòng)" fn={() => door('DOUBLE', 'LEFT', 'INSET')} />
          <Item label="Cánh lật (lề trên)" fn={() => door('SINGLE', 'TOP')} />
          <Item label="Cửa lùa 2 cánh" fn={() => door('SLIDING')} />
          <h5>Ngăn kéo</h5>
          {[1, 2, 3, 4].map((n) => (
            <Item key={n} label={`Ngăn kéo × ${n}`} fn={() => drawers(n)} />
          ))}
          <Item label="Ngăn kéo × 2 (lọt lòng)" fn={() => drawers(2, 'INSET')} />
          <h5>Liên kết</h5>
          <Item label="Thanh treo oval" fn={() => Commands.zoneAddLink(cabinet, zones, 'OVAL_RAIL')} />
        </div>
      </div>
      <hr />
      <button onPointerDown={(e) => e.stopPropagation()} onClick={() => { set({ zoneMenu: null, designerTab: 'create' }); }}>
        <span>Tùy chọn chi tiết… (tab Tạo tấm)</span>
      </button>
    </div>
  );
}

/** Small input dialog (uiStore.prompt). */
export function PromptDialog() {
  const { prompt, set } = useUi();
  const [v, setV] = useState('');
  useEffect(() => setV(prompt?.value ?? ''), [prompt]);
  if (!prompt) return null;
  const ok = () => {
    set({ prompt: null });
    if (v.trim()) prompt.ok(v.trim());
  };
  return (
    <div className="modal-back" onPointerDown={() => set({ prompt: null })}>
      <div className="modal prompt" onPointerDown={(e) => e.stopPropagation()}>
        <h3>{prompt.title}</h3>
        <label>{prompt.label}</label>
        <input
          className="field"
          autoFocus
          value={v}
          onFocus={(e) => e.currentTarget.select()}
          onChange={(e) => setV(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') ok();
            if (e.key === 'Escape') set({ prompt: null });
          }}
        />
        <div className="row-btns">
          <button className="btn" onClick={() => set({ prompt: null })}>Hủy</button>
          <button className="btn primary" onClick={ok}>Đồng ý</button>
        </div>
      </div>
    </div>
  );
}

/** Chia đều lại the split that owns a shelf / divider. */
async function equalizeOf(panel: ObjectId, cabinet: ObjectId) {
  const info = await Queries.zones(cabinet);
  const uid = info.panels.find((p) => p.id === panel)?.uid;
  const zone = info.positions.find((p) => p.uid === uid)?.zone;
  if (zone !== undefined) await Commands.equalizeSplit(cabinet, zone);
}

export function ContextMenu() {
  const { contextMenu, set, tree } = useUi();
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => set({ contextMenu: null });
    window.addEventListener('pointerdown', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('pointerdown', close);
      window.removeEventListener('blur', close);
    };
  }, [contextMenu, set]);
  if (!contextMenu) return null;
  const n = findNode(tree, contextMenu.id)?.node;
  const item = (icon: string, label: string, fn: () => void, hint?: string, disabled = false) => (
    <button
      disabled={disabled}
      onPointerDown={(e) => e.stopPropagation()}
      onClick={() => {
        set({ contextMenu: null });
        fn();
      }}
    >
      <Icon name={icon} size={15} />
      <span>{label}</span>
      {hint && <kbd>{hint}</kbd>}
    </button>
  );
  const id = contextMenu.id;
  const selection = useUi.getState().selection;
  const sel2 = selection.length === 2 && selection.every((x) => findNode(tree, x)?.node.kind === 'PANEL') ? selection : [];
  return (
    <div className="ctx" style={{ left: contextMenu.x, top: contextMenu.y }} onPointerDown={(e) => e.stopPropagation()}>
      <div className="ctx-title">{n?.name}</div>
      {item('new', 'Đổi tên', () => set({ renaming: id }), 'F2')}
      {item('duplicate', 'Sao chép', () => void Actions.duplicate(), 'Ctrl+D')}
      {item(n?.visible ? 'eyeOff' : 'eye', n?.visible ? 'Ẩn' : 'Hiện', () => void Actions.toggleHidden(), 'H')}
      {item(n?.locked ? 'unlock' : 'lock', n?.locked ? 'Mở khóa' : 'Khóa', () => void Actions.toggleLocked())}
      {item('trash', 'Xóa', () => void Actions.delete(), 'Del')}
      <hr />
      {item('chevronRight', 'Chọn đối tượng cha', () => Actions.selectParent(id), undefined, findNode(tree, id)?.parent == null)}
      {item('chevronDown', 'Chọn các con', () => Actions.selectChildren(id), undefined, !n?.children.length)}
      <hr />
      {sel2.length === 2 && (
        <>
          <hr />
          <div className="ctx-title">Quan hệ 2 tấm (A = chọn trước)</div>
          {item('shelf', 'A phủ B', () => void Commands.setRelation(sel2[0], sel2[1], 'OVERLAY').catch(() => undefined))}
          {item('shelf', 'A lọt B', () => void Commands.setRelation(sel2[0], sel2[1], 'INSET').catch(() => undefined))}
          {item('align', 'Bằng mặt trước', () => void Commands.setRelation(sel2[0], sel2[1], 'FLUSH').catch(() => undefined))}
          {item('fit', 'Khe…', () =>
            set({ prompt: { title: 'Khe giữa 2 tấm', label: 'Khe (mm)', value: '3', ok: (v) => void Commands.setRelation(sel2[0], sel2[1], 'GAP', Number(v.replace(',', '.')) || 0).catch(() => undefined) } }),
          )}
          {item('x', 'Bỏ quan hệ', () => void Commands.setRelation(sel2[0], sel2[1], 'NONE').catch(() => undefined))}
        </>
      )}
      {n?.kind === 'PANEL' && n.generated && (
        <>
          <hr />
          {(n.role === 'Shelf' || n.role === 'ShelfFixed') && item('shelf', n.role === 'Shelf' ? 'Đổi thành kệ cố định' : 'Đổi thành kệ di động', () => void Commands.setParameter(id, 'split_kind', n.role === 'Shelf' ? 'SHELF_FIXED' : 'SHELF_ADJUSTABLE').catch(() => undefined))}
          {['Shelf', 'ShelfFixed', 'Divider', 'BackSub'].includes(n.role ?? '') && item('align', 'Căn giữa vùng (50%)', () => void Commands.setParameter(id, 'pos_ratio', '50').catch(() => undefined))}
          {['Shelf', 'ShelfFixed', 'Divider', 'BackSub'].includes(n.role ?? '') &&
            item('grid', 'Chia đều lại', () => {
              const cab = findNode(tree, id)?.parent;
              if (cab != null) void equalizeOf(id, cab).catch(() => undefined);
            })}
          {['Shelf', 'ShelfFixed', 'Divider', 'BackSub'].includes(n.role ?? '') &&
            item('duplicate', 'Nhân tấm (chia đều)…', () =>
              set({ prompt: { title: 'Nhân tấm', label: 'Thêm bao nhiêu tấm giống tấm này?', value: '2', ok: (v) => void Commands.arraySplitPanel(id, Math.max(1, Math.round(Number(v)) || 1)).catch(() => undefined) } }),
            )}
          {item('fit', 'Co giãn trên +20', () => void Commands.setPartMod(id, { extend_delta: [0, 0, 0, 20] }).catch(() => undefined))}
          {item('fit', 'Co giãn trên −20', () => void Commands.setPartMod(id, { extend_delta: [0, 0, 0, -20] }).catch(() => undefined))}
          {item('edit', 'Chỉnh tấm…', () => set({ designerTab: 'edit' }))}
        </>
      )}
      {n?.kind === 'CABINET' && (
        <>
          {item('shelf', 'Dựng chi tiết (Tạo tấm)…', () => set({ designerTab: 'create' }))}
          {item('edit', 'Sửa kích thước…', () => set({ designerTab: 'edit' }))}
          {item('mirror', 'Lật gương trái ↔ phải', () => void Commands.mirrorCabinet(id).catch(() => undefined))}
          {item('duplicate', 'Nhân dãy tủ sang phải…', () =>
            set({ prompt: { title: 'Nhân dãy tủ', label: 'Số tủ thêm (đặt liền bên phải)', value: '1', ok: (v) => void Commands.arrayCabinet(id, Math.max(1, Math.round(Number(v)) || 1), 0, 0).catch(() => undefined) } }),
          )}
          {item('save', 'Lưu làm template…', () =>
            set({
              prompt: {
                title: 'Lưu tủ làm template',
                label: 'Tên template (lưu cấu trúc khoang, cánh, ngăn kéo, luật, vật liệu)',
                value: n.name,
                ok: (v) => void Commands.saveTemplate(id, v).then(() => useUi.getState().toast({ kind: 'success', title: `Đã lưu template “${v}”` })).catch(() => undefined),
              },
            }),
          )}
          {item('report', 'Báo cáo', () => set({ drawer: 'report' }))}
        </>
      )}
      {item('relations', 'Xem quan hệ', () => Actions.inspectRelations(id))}
      {item('drill', 'Gia công', () => Actions.openManufacturing(id))}
      {item('fit', 'Phóng vừa', () => View.fit([id], tree), 'F')}
    </div>
  );
}
