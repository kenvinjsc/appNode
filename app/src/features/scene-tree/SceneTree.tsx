// Scene tree: a view of the core's tree. Every change is a command to the core;
// validity (e.g. of a reparent) is decided there, not here.
import { useEffect, useMemo, useRef, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { ObjectId, TreeNode } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { objectLabel } from '../../shared/i18n';

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
  const { tree, selection, select, set, renaming, hovered } = useUi();
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
    if (!q) return flatten(tree.roots, open);
    const every = new Set<ObjectId>();
    const walk = (n: TreeNode) => {
      every.add(n.id);
      n.children.forEach(walk);
    };
    tree.roots.forEach(walk);
    return flatten(tree.roots.filter((r) => matches(r, q)), every).filter((r) => matches(r.n, q));
  }, [tree, open, q]);

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
        {tree && tree.roots.length === 0 && <div className="empty">Chưa có đối tượng. Dùng “Tủ” trên thanh công cụ để tạo tủ.</div>}
      </div>
    </div>
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
