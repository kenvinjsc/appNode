// Title bar, workspace rail, status bar, toasts, jobs, context menu.
import { useEffect, useState } from 'react';
import { Queries } from '../core-api/queries';
import { Icon } from '../shared/icons';
import { CONTACT_LABEL, fmt } from '../shared/i18n';
import { View } from '../viewport/viewportBus';
import { Actions } from './actions';
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
      {item('relations', 'Xem quan hệ', () => Actions.inspectRelations(id))}
      {item('drill', 'Gia công', () => Actions.openManufacturing(id))}
      {item('fit', 'Phóng vừa', () => View.fit([id], tree), 'F')}
    </div>
  );
}
