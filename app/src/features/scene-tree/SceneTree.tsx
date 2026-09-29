// Scene tree: a view of the core's tree. Every change is a command to the core;
// validity (e.g. of a reparent) is decided there, not here.
import { useEffect, useMemo, useRef, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { ObjectId, TreeNode } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { objectLabel } from '../../shared/i18n';
import { extraKey, projectFloors, projectRooms, roomLabel, roomRoots } from './rooms';

function iconFor(n: TreeNode): string {
  if (n.kind === 'ROOM') return 'room';
  if (n.kind === 'CABINET') return 'cabinet';
  if (n.kind === 'HARDWARE') return 'hardware';
  switch (n.role) {
    case 'Shelf':
      return 'shelf';
    case 'Door':
      return 'door';
    case 'DrawerFront':
      return 'drawer';
    default:
      return 'panel';
  }
}

function flatten(nodes: TreeNode[], open: Set<ObjectId>, depth = 0, out: { n: TreeNode; depth: number }[] = []) {
  for (const n of nodes) {
    out.push({ n, depth });
    if (n.children.length && open.has(n.id)) flatten(n.children, open, depth + 1, out);
  }
  return out;
}

function matches(n: TreeNode, q: string): boolean {
  return n.name.toLowerCase().includes(q) || n.children.some((c) => matches(c, q));
}

export function SceneTree() {
  const { tree, selection, select, set, renaming, hovered, activeRoom, activeFloor } = useUi();
  const roots = useMemo(() => roomRoots(tree, activeFloor, activeRoom), [tree, activeFloor, activeRoom]);
  const [open, setOpen] = useState<Set<ObjectId>>(new Set());
  const [query, setQuery] = useState('');
  const [dropTarget, setDropTarget] = useState<ObjectId | 'root' | null>(null);
  const anchor = useRef<ObjectId | null>(null);

  // Auto-expand ancestors of the selection.
  useEffect(() => {
    if (!tree || selection.length === 0) return;
    const next = new Set(open);
    let changed = false;
    const walk = (n: TreeNode, path: ObjectId[]): boolean => {
      if (selection.includes(n.id)) {
        for (const p of path) if (!next.has(p)) ((next.add(p)), (changed = true));
        return true;
      }
      return n.children.some((c) => walk(c, [...path, n.id]));
    };
    tree.roots.forEach((r) => walk(r, []));
    if (changed) setOpen(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selection, tree]);

  // Open roots by default.
  useEffect(() => {
    if (tree && open.size === 0) setOpen(new Set(tree.roots.map((r) => r.id)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tree]);

  const q = query.trim().toLowerCase();
  const rows = useMemo(() => {
    if (!tree) return [];
    if (!q) return flatten(roots, open);
    const every = new Set<ObjectId>();
    const walk = (n: TreeNode) => {
      every.add(n.id);
      n.children.forEach(walk);
    };
    tree.roots.forEach(walk);
    return flatten(roots.filter((r) => matches(r, q)), every).filter((r) => matches(r.n, q));
  }, [tree, roots, open, q]);

  const onClick = (e: React.MouseEvent, n: TreeNode) => {
    if (e.shiftKey && anchor.current !== null) {
      const ids = rows.map((r) => r.n.id);
      const a = ids.indexOf(anchor.current);
      const b = ids.indexOf(n.id);
      if (a >= 0 && b >= 0) {
        select(ids.slice(Math.min(a, b), Math.max(a, b) + 1));
        return;
      }
    }
    select([n.id], e.ctrlKey || e.metaKey ? 'toggle' : 'replace');
    anchor.current = n.id;
  };

  const toggle = (id: ObjectId) => {
    const next = new Set(open);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setOpen(next);
  };

  const drop = async (target: TreeNode | null, dragged: ObjectId) => {
    setDropTarget(null);
    if (target && target.id === dragged) return;
    try {
      await Commands.reparent(dragged, target ? target.id : null);
    } catch {
      /* core rejected; toast shown */
    }
  };

  return (
    <div className="panel scene-tree">
      <div className="panel-header">
        <Icon name="layers" size={16} />
        <span>Cây đối tượng</span>
      </div>
      <ProjectTabs />
      <div className="tree-search">
        <Icon name="search" size={14} />
        <input placeholder="Tìm chi tiết…" value={query} onChange={(e) => setQuery(e.target.value)} />
      </div>
      <div
        className={`tree-body ${dropTarget === 'root' ? 'drop' : ''}`}
        onDragOver={(e) => {
          e.preventDefault();
          if (e.target === e.currentTarget) setDropTarget('root');
        }}
        onDrop={(e) => {
          const id = Number(e.dataTransfer.getData('text/aic-id'));
          if (e.target === e.currentTarget && id) void drop(null, id);
        }}
      >
        <div className="tree-row project">
          <Icon name="logo" size={14} style={{ color: 'var(--accent)' }} />
          <b>{tree?.name ?? 'Dự án'}</b>
        </div>
        {rows.map(({ n, depth }) => {
          const sel = selection.includes(n.id);
          return (
            <div
              key={n.id}
              className={`tree-row ${sel ? 'selected' : ''} ${hovered === n.id ? 'hover' : ''} ${!n.visible ? 'hidden' : ''} ${dropTarget === n.id ? 'drop' : ''}`}
              style={{ paddingLeft: 10 + depth * 16 }}
              onClick={(e) => onClick(e, n)}
              onDoubleClick={() => set({ renaming: n.id })}
              onMouseEnter={() => set({ hovered: n.id })}
              onMouseLeave={() => set({ hovered: null })}
              onContextMenu={(e) => {
                e.preventDefault();
                if (!selection.includes(n.id)) select([n.id]);
                set({ contextMenu: { x: e.clientX, y: e.clientY, id: n.id } });
              }}
              draggable={!n.locked}
              onDragStart={(e) => e.dataTransfer.setData('text/aic-id', String(n.id))}
              onDragOver={(e) => {
                if (n.kind === 'ROOM' || n.kind === 'CABINET') {
                  e.preventDefault();
                  e.stopPropagation();
                  setDropTarget(n.id);
                }
              }}
              onDragLeave={() => setDropTarget(null)}
              onDrop={(e) => {
                e.stopPropagation();
                const id = Number(e.dataTransfer.getData('text/aic-id'));
                if (id) void drop(n, id);
              }}
            >
              <span
                className={`twisty ${n.children.length ? '' : 'none'}`}
                onClick={(e) => {
                  e.stopPropagation();
                  toggle(n.id);
                }}
              >
                {n.children.length > 0 && <Icon name={open.has(n.id) || q ? 'chevronDown' : 'chevronRight'} size={12} />}
              </span>
              <Icon name={iconFor(n)} size={15} className="kind-icon" />
              {renaming === n.id ? (
                <RenameBox node={n} />
              ) : (
                <span className="tree-name" title={objectLabel(n.kind, n.role)}>
                  {n.name}
                </span>
              )}
              <span className="tree-actions">
                <button
                  title={n.visible ? 'Ẩn' : 'Hiện'}
                  className={!n.visible ? 'on' : ''}
                  onClick={(e) => {
                    e.stopPropagation();
                    void Commands.setVisible([n.id], !n.visible).catch(() => undefined);
                  }}
                >
                  <Icon name={n.visible ? 'eye' : 'eyeOff'} size={14} />
                </button>
                <button
                  title={n.locked ? 'Mở khóa' : 'Khóa'}
                  className={n.locked ? 'on' : ''}
                  onClick={(e) => {
                    e.stopPropagation();
                    void Commands.setLocked([n.id], !n.locked).catch(() => undefined);
                  }}
                >
                  <Icon name={n.locked ? 'lock' : 'unlock'} size={14} />
                </button>
              </span>
            </div>
          );
        })}
        {tree && (activeRoom !== null || activeFloor !== null) && roots.length === 0 && (
          <div className="empty">
            {activeRoom !== null ? `Phòng “${roomLabel(activeRoom)}”` : `“${activeFloor}”`} chưa có tủ. Mở tab Khung → [TAB] Tạo tủ để thêm tủ vào đây.
          </div>
        )}
        {tree && tree.roots.length === 0 && <div className="empty">Chưa có đối tượng. Dùng “Tủ” trên thanh công cụ để tạo tủ.</div>}
      </div>
    </div>
  );
}

/** Editable tab row used for floors (tầng) and rooms (phòng). */
function TabRow(props: {
  label: string;
  addLabel: string;
  allLabel: string;
  allCount: number;
  items: { name: string; count: number }[];
  active: string | null;
  name: (v: string) => string;
  onPick: (v: string | null) => void;
  onAdd: (v: string) => void;
  onRename: (from: string, to: string) => Promise<void>;
  placeholder: string;
  className?: string;
}) {
  const [edit, setEdit] = useState<{ from: string | null; value: string } | null>(null);
  const commit = async () => {
    if (!edit) return;
    const v = edit.value.trim();
    setEdit(null);
    if (!v) return;
    if (edit.from === null) props.onAdd(v);
    else if (v !== edit.from) await props.onRename(edit.from, v);
  };
  const box = (
    <input
      autoFocus
      className="room-edit"
      value={edit?.value ?? ''}
      placeholder={props.placeholder}
      onChange={(e) => setEdit(edit && { ...edit, value: e.target.value })}
      onBlur={() => void commit()}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === 'Enter') void commit();
        if (e.key === 'Escape') setEdit(null);
      }}
    />
  );
  return (
    <div className={`room-tabs ${props.className ?? ''}`} role="tablist" aria-label={props.label}>
      <button role="tab" className={props.active === null ? 'on' : ''} onClick={() => props.onPick(null)}>
        {props.allLabel} <span>{props.allCount}</span>
      </button>
      {props.items.map((r) =>
        edit && edit.from === r.name ? (
          <span key={r.name}>{box}</span>
        ) : (
          <button
            key={r.name}
            role="tab"
            className={props.active === r.name ? 'on' : ''}
            title="Bấm: xem & tạo tủ tại đây · Bấm đúp: đổi tên"
            onClick={() => props.onPick(r.name)}
            onDoubleClick={() => r.name !== '' && setEdit({ from: r.name, value: r.name })}
          >
            {props.name(r.name)} <span>{r.count}</span>
          </button>
        ),
      )}
      {edit && edit.from === null ? (
        box
      ) : (
        <button className="add" title={props.addLabel} onClick={() => setEdit({ from: null, value: '' })}>
          <Icon name="plus" size={12} /> {props.addLabel}
        </button>
      )}
    </div>
  );
}

/** One project, many floors; each floor has its own rooms. Tabs filter the tree and the
 *  3D view, and decide where "[TAB] Tạo tủ" puts the new cabinet. */
function ProjectTabs() {
  const { tree, activeRoom, activeFloor, extraRooms, extraFloors, set } = useUi();
  const floors = projectFloors(tree, extraFloors);
  const rooms = projectRooms(tree, activeFloor, extraRooms);
  const total = tree?.roots.filter((r) => r.kind === 'CABINET').length ?? 0;
  const showFloors = floors.length > 1 || (floors.length === 1 && floors[0].name !== '') || extraFloors.length > 0;
  const moveAll = async (list: TreeNode[], key: 'floor' | 'room', v: string) => {
    for (const c of list) await Commands.setParameter(c.id, key, v).catch(() => undefined);
  };
  return (
    <>
      {showFloors ? (
        <TabRow
          className="floors"
          label="Tầng"
          addLabel="Tầng"
          allLabel="Mọi tầng"
          allCount={total}
          items={floors}
          active={activeFloor}
          name={(v) => (v === '' ? 'Chưa gán tầng' : v)}
          placeholder="Tầng 1"
          onPick={(v) => set({ activeFloor: v, activeRoom: null, pinned: { cabinet: null, zones: [] } })}
          onAdd={(v) => set({ extraFloors: [...extraFloors.filter((f) => f !== v), v], activeFloor: v, activeRoom: null })}
          onRename={async (from, to) => {
            await moveAll(roomRoots(tree, from, null), 'floor', to);
            set({
              extraFloors: extraFloors.map((f) => (f === from ? to : f)),
              extraRooms: extraRooms.map((k) => (k.startsWith(from + '/') ? to + k.slice(from.length) : k)),
              activeFloor: to,
            });
          }}
        />
      ) : (
        <div className="room-tabs floors">
          <button className="add" title="Chia dự án theo tầng" onClick={() => set({ extraFloors: ['Tầng 1'], activeFloor: 'Tầng 1', activeRoom: null })}>
            <Icon name="plus" size={12} /> Chia tầng
          </button>
        </div>
      )}
      <TabRow
        label="Phòng"
        addLabel="Phòng"
        allLabel={activeFloor === null ? 'Tất cả' : 'Cả tầng'}
        allCount={roomRoots(tree, activeFloor, null).filter((r) => r.kind === 'CABINET').length}
        items={rooms}
        active={activeRoom}
        name={roomLabel}
        placeholder="Tên phòng"
        onPick={(v) => set({ activeRoom: v, pinned: { cabinet: null, zones: [] } })}
        onAdd={(v) => set({ extraRooms: [...extraRooms.filter((k) => k !== extraKey(activeFloor, v)), extraKey(activeFloor, v)], activeRoom: v })}
        onRename={async (from, to) => {
          await moveAll(roomRoots(tree, activeFloor, from), 'room', to);
          set({ extraRooms: extraRooms.map((k) => (k === extraKey(activeFloor, from) ? extraKey(activeFloor, to) : k)), activeRoom: to });
        }}
      />
    </>
  );
}

function RenameBox({ node }: { node: TreeNode }) {
  const [v, setV] = useState(node.name);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => ref.current?.select(), []);
  const done = (commit: boolean) => {
    useUi.getState().set({ renaming: null });
    if (commit && v.trim() && v !== node.name) void Commands.setName(node.id, v).catch(() => undefined);
  };
  return (
    <input
      ref={ref}
      className="rename"
      value={v}
      onClick={(e) => e.stopPropagation()}
      onChange={(e) => setV(e.target.value)}
      onBlur={() => done(true)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') done(true);
        if (e.key === 'Escape') done(false);
        e.stopPropagation();
      }}
    />
  );
}
