// 2D technical view (elevation / side / plan). A projection of what the
// viewport already displays — presentation only, no CAD computation.
import { useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { useUi } from '../../app/uiStore';
import type { ObjectId } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { fmt, objectLabel } from '../../shared/i18n';
import { expandSubtrees, getEngine, useSceneRevision } from '../../viewport/viewportBus';
import { useCurrentCabinet, useZones } from '../cabinet/useZones';
import { EditLayer } from './EditLayer';

type Plane = 'front' | 'side' | 'top';

export interface Item {
  id: ObjectId;
  kind: string;
  role: string | null;
  name: string;
  color: string;
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  depth: number;
  front: boolean;
}

export function Drawing2D({ onClose }: { onClose?: () => void }) {
  const { selection, tree, select, active } = useUi();
  const rev = useSceneRevision((s) => s.rev);
  const [plane, setPlane] = useState<Plane>('front');
  const [hideFronts, setHideFronts] = useState(true);
  const [showDims, setShowDims] = useState(true);
  const svgRef = useRef<SVGSVGElement>(null);
  // 2D editor: the current cabinet (selection or pinned zone), front view.
  const current = useCurrentCabinet();
  const zinfo = useZones(plane === 'front' ? current : null);

  // Scope: the cabinet(s) of the selection, or everything.
  const scope = useMemo(() => {
    const e = getEngine();
    if (!e) return [];
    let roots: ObjectId[] = [];
    for (const id of selection) {
      const cab = e.entries.get(id)?.ro.cabinet;
      roots.push(cab ?? id);
    }
    roots = Array.from(new Set(roots));
    return roots.length ? expandSubtrees(tree, roots) : Array.from(e.entries.keys());
  }, [selection, tree, rev]);

  const items = useMemo(() => {
    const e = getEngine();
    if (!e) return [] as Item[];
    const out: Item[] = [];
    for (const id of scope) {
      const en = e.entries.get(id);
      if (!en || !en.ro.visible || en.ro.kind === 'ROOM') continue;
      const isFront = en.ro.role === 'Door' || en.ro.role === 'DrawerFront' || en.ro.kind === 'HARDWARE';
      if (hideFronts && isFront && plane === 'front') continue;
      const b = e.worldBox(id);
      if (!b) continue;
      const [ax, ay, dz] = plane === 'front' ? (['x', 'y', 'z'] as const) : plane === 'side' ? (['z', 'y', 'x'] as const) : (['x', 'z', 'y'] as const);
      out.push({
        id,
        kind: en.ro.kind,
        role: en.ro.role,
        name: en.ro.name,
        color: en.ro.color,
        x0: b.min[ax],
        x1: b.max[ax],
        y0: plane === 'top' ? b.min[ay] : -b.max[ay],
        y1: plane === 'top' ? b.max[ay] : -b.min[ay],
        depth: plane === 'side' ? -b.max[dz] : b.max[dz],
        front: isFront,
      });
    }
    out.sort((a, b) => a.depth - b.depth);
    return out;
  }, [scope, plane, hideFronts, rev]);

  const bounds = useMemo(() => {
    const b = new THREE.Box2();
    for (const it of items) {
      b.expandByPoint(new THREE.Vector2(it.x0, it.y0));
      b.expandByPoint(new THREE.Vector2(it.x1, it.y1));
    }
    return b;
  }, [items]);

  const direct = new Set(selection);
  const selected = new Set(expandSubtrees(tree, selection));
  const pad = Math.max(bounds.max.x - bounds.min.x, bounds.max.y - bounds.min.y) * 0.14 + 60;
  const vb = bounds.isEmpty() ? '0 0 1000 1000' : `${bounds.min.x - pad} ${bounds.min.y - pad} ${bounds.max.x - bounds.min.x + 2 * pad} ${bounds.max.y - bounds.min.y + 2 * pad}`;
  const W = bounds.max.x - bounds.min.x;
  const H = bounds.max.y - bounds.min.y;
  const unit = Math.max(W, H) / 60 || 10;
  const editing = !!zinfo && plane === 'front' && scope.includes(zinfo.cabinet);

  return (
    <div className="panel drawing">
      <div className="panel-header">
        <Icon name="drawing" size={16} />
        <span>2D · Bản vẽ</span>
        <div className="spacer" />
        <select value={plane} onChange={(e) => setPlane(e.target.value as Plane)}>
          <option value="front">Mặt đứng (Trước)</option>
          <option value="side">Mặt cắt cạnh (Phải)</option>
          <option value="top">Mặt bằng (Trên)</option>
        </select>
        <button className={`icon-btn ${hideFronts ? 'on' : ''}`} title="Ẩn cánh / mặt ngăn kéo" onClick={() => setHideFronts(!hideFronts)}>
          <Icon name="door" size={16} />
        </button>
        <button className={`icon-btn ${showDims ? 'on' : ''}`} title="Hiển thị kích thước" onClick={() => setShowDims(!showDims)}>
          <Icon name="dimension" size={16} />
        </button>
        {onClose && (
          <button className="icon-btn" title="Đóng" onClick={onClose}>
            <Icon name="x" size={16} />
          </button>
        )}
      </div>
      <div className="drawing-body">
        {items.length === 0 ? (
          <div className="empty">Không có gì để hiển thị.</div>
        ) : (
          <svg ref={svgRef} viewBox={vb} preserveAspectRatio="xMidYMid meet">
            {items.map((it) => {
              const sel = direct.has(it.id);
              const inSel = selected.has(it.id);
              const w = it.x1 - it.x0;
              const h = it.y1 - it.y0;
              return (
                <g key={it.id} onClick={(e) => select([it.id], e.ctrlKey ? 'toggle' : 'replace')} className="d2-item">
                  <rect
                    x={it.x0}
                    y={it.y0}
                    width={Math.max(w, 0.5)}
                    height={Math.max(h, 0.5)}
                    fill={sel ? '#ffd8bf' : it.kind === 'HARDWARE' ? '#adb5bd' : it.color}
                    fillOpacity={it.front ? 0.55 : 0.9}
                    stroke={sel || inSel ? '#e8590c' : '#495057'}
                    strokeWidth={sel ? unit * 0.28 : unit * 0.1}
                    vectorEffect="non-scaling-stroke"
                  >
                    <title>{`${it.name} (${objectLabel(it.kind, it.role)}) ${fmt(w, 1)} × ${fmt(h, 1)}`}</title>
                  </rect>
                </g>
              );
            })}
            {editing && <EditLayer info={zinfo!} items={items} unit={unit} svg={svgRef.current} />}
            {showDims && !editing && <Openings items={items} unit={unit} active={active} />}
            {showDims && !editing && !bounds.isEmpty() && (
              <g className="d2-dims" fontSize={unit * 1.3}>
                <line x1={bounds.min.x} y1={bounds.min.y - unit * 2.5} x2={bounds.max.x} y2={bounds.min.y - unit * 2.5} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
                <line x1={bounds.min.x} y1={bounds.min.y - unit * 3.5} x2={bounds.min.x} y2={bounds.min.y} className="ext" />
                <line x1={bounds.max.x} y1={bounds.min.y - unit * 3.5} x2={bounds.max.x} y2={bounds.min.y} className="ext" />
                <text x={(bounds.min.x + bounds.max.x) / 2} y={bounds.min.y - unit * 3.1} textAnchor="middle">
                  {fmt(W, 1)}
                </text>
                <line x1={bounds.min.x - unit * 2.5} y1={bounds.min.y} x2={bounds.min.x - unit * 2.5} y2={bounds.max.y} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
                <line x1={bounds.min.x - unit * 3.5} y1={bounds.min.y} x2={bounds.min.x} y2={bounds.min.y} className="ext" />
                <line x1={bounds.min.x - unit * 3.5} y1={bounds.max.y} x2={bounds.min.x} y2={bounds.max.y} className="ext" />
                <text transform={`translate(${bounds.min.x - unit * 3.2} ${(bounds.min.y + bounds.max.y) / 2}) rotate(-90)`} textAnchor="middle">
                  {fmt(H, 1)}
                </text>
              </g>
            )}
            <defs>
              <marker id="d2a" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
                <path d="M0 1 L9 5 L0 9 z" fill="#343a40" />
              </marker>
            </defs>
          </svg>
        )}
      </div>
    </div>
  );
}

/** Label clear openings between parallel parts (visual measurement aid). */
function Openings({ items, unit }: { items: Item[]; unit: number; active: ObjectId | null }) {
  // Vertical members (taller than wide) and horizontal members.
  // Only carcass / division members bound openings (drawer boxes, fronts and rails do not).
  const STRUCT = new Set(['LeftSide', 'RightSide', 'Divider', 'Top', 'Bottom', 'Shelf', 'ShelfFixed', 'Generic', 'Rail']);
  const members = items.filter((i) => i.kind === 'PANEL' && (i.role === null || STRUCT.has(i.role)));
  const verts = members.filter((i) => i.y1 - i.y0 > (i.x1 - i.x0) * 3);
  const hors = members.filter((i) => i.x1 - i.x0 > (i.y1 - i.y0) * 3);
  const labels: JSX.Element[] = [];
  const xs = Array.from(new Set(verts.flatMap((v) => [v.x0, v.x1]).map((v) => Math.round(v * 10) / 10))).sort((a, b) => a - b);
  // Column openings: gaps between successive vertical members.
  const cols: [number, number][] = [];
  const sortedV = [...verts].sort((a, b) => a.x0 - b.x0);
  for (let i = 0; i + 1 < sortedV.length; i++) {
    const a = sortedV[i].x1;
    const b = sortedV[i + 1].x0;
    if (b - a > unit * 3) cols.push([a, b]);
  }
  void xs;
  for (const [a, b] of cols) {
    const inCol = hors.filter((h) => h.x0 <= a + 1 && h.x1 >= b - 1).sort((p, q) => p.y0 - q.y0);
    for (let i = 0; i + 1 < inCol.length; i++) {
      const top = inCol[i].y1;
      const bot = inCol[i + 1].y0;
      if (bot - top < unit * 3) continue;
      labels.push(
        <text key={`${a}-${top}`} x={(a + b) / 2} y={(top + bot) / 2} textAnchor="middle" dominantBaseline="middle" fontSize={unit * 1.25} className="d2-open">
          {fmt(b - a, 1)} × {fmt(bot - top, 1)}
        </text>,
      );
    }
  }
  return <g>{labels}</g>;
}
