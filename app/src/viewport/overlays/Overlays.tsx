// Screen-space overlays (SVG) drawn over the canvas: dimensions, snap hints,
// relation markers and direct-manipulation handles. Nothing is baked into
// geometry; everything is re-projected every rendered frame.
import { useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { findNode, useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries, transformOf } from '../../core-api/queries';
import type { AssemblyRelation, ObjectId, SnapHint, Vec3 } from '../../core-api/types';
import { CONTACT_LABEL, ORIENT_LABEL, REGION_LABEL, fmt } from '../../shared/i18n';
import type { ViewportEngine } from '../renderer/ViewportEngine';
import { expandSubtrees, useSceneRevision } from '../viewportBus';

/** Re-render on every drawn frame of the viewport. */
export function useFrame(engine: ViewportEngine | null) {
  const [, setN] = useState(0);
  useEffect(() => {
    if (!engine) return;
    let pending = false;
    const prev = engine.onFrame;
    engine.onFrame = () => {
      prev?.();
      if (!pending) {
        pending = true;
        requestAnimationFrame(() => {
          pending = false;
          setN((n) => n + 1);
        });
      }
    };
    return () => {
      engine.onFrame = prev;
    };
  }, [engine]);
}

type P = { x: number; y: number; visible: boolean };

function DimLine({ a, b, label, offset = 0, onPick }: { a: P; b: P; label: string; offset?: number; onPick?: (x: number, y: number) => void }) {
  if (!a.visible || !b.visible) return null;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy);
  if (len < 24) return null;
  const nx = (-dy / len) * offset;
  const ny = (dx / len) * offset;
  const A = { x: a.x + nx, y: a.y + ny };
  const B = { x: b.x + nx, y: b.y + ny };
  const mx = (A.x + B.x) / 2;
  const my = (A.y + B.y) / 2;
  let ang = (Math.atan2(dy, dx) * 180) / Math.PI;
  if (ang > 90) ang -= 180;
  if (ang < -90) ang += 180;
  return (
    <g className="dim">
      <line x1={a.x} y1={a.y} x2={A.x} y2={A.y} className="dim-ext" />
      <line x1={b.x} y1={b.y} x2={B.x} y2={B.y} className="dim-ext" />
      <line x1={A.x} y1={A.y} x2={B.x} y2={B.y} className="dim-line" markerStart="url(#dim-arrow)" markerEnd="url(#dim-arrow)" />
      <g
        transform={`translate(${mx} ${my}) rotate(${ang})`}
        className={onPick ? 'dim-edit' : undefined}
        style={onPick ? { pointerEvents: 'all', cursor: 'text' } : undefined}
        onPointerDown={onPick ? (e) => e.stopPropagation() : undefined}
        onClick={onPick ? () => onPick(mx, my) : undefined}
      >
        {onPick && <title>Bấm để nhập kích thước mới</title>}
        <rect x={-label.length * 3.7 - 5} y={-9} width={label.length * 7.4 + 10} height={18} rx={3} className="dim-bg" />
        <text textAnchor="middle" dy={4} className="dim-text">
          {label}
        </text>
      </g>
    </g>
  );
}

export function DimensionOverlay({ engine }: { engine: ViewportEngine }) {
  useFrame(engine);
  const { selection, tree, showDimensions, stretchMode } = useUi();
  useSceneRevision((s) => s.rev);
  const [edit, setEdit] = useState<{ key: 'width' | 'height' | 'depth'; x: number; y: number; value: number } | null>(null);
  // One cabinet selected: its W / H / D labels are editable (a parameter change,
  // spread over the bays by the chosen stretch mode).
  const cab = selection.length === 1 && findNode(tree, selection[0])?.node.kind === 'CABINET' ? selection[0] : null;
  const pick = (key: 'width' | 'height' | 'depth') =>
    cab === null
      ? undefined
      : (x: number, y: number) =>
          void Queries.properties(cab)
            .then((p) => setEdit({ key, x, y, value: Number(p.groups.flatMap((g) => g.fields).find((f) => f.key === key)?.value ?? 0) }))
            .catch(() => undefined);
  if (!showDimensions || selection.length === 0) return null;
  const ids = expandSubtrees(tree, selection);
  const box = engine.boxOf(ids);
  if (box.isEmpty()) return null;
  const s = (x: number, y: number, z: number) => engine.toScreen(new THREE.Vector3(x, y, z));
  const { min, max } = box;
  const size = box.getSize(new THREE.Vector3());
  const out: JSX.Element[] = [
    <DimLine key="w" a={s(min.x, min.y, max.z)} b={s(max.x, min.y, max.z)} label={`${fmt(size.x)}`} offset={28} onPick={pick('width')} />,
    <DimLine key="h" a={s(max.x, min.y, max.z)} b={s(max.x, max.y, max.z)} label={`${fmt(size.y)}`} offset={-28} onPick={pick('height')} />,
    <DimLine key="d" a={s(max.x, min.y, max.z)} b={s(max.x, min.y, min.z)} label={`${fmt(size.z)}`} offset={-28} onPick={pick('depth')} />,
  ];
  if (edit && cab !== null) {
    let closed = false;
    const done = (txt: string | null) => {
      if (closed) return;
      closed = true;
      setEdit(null);
      const v = txt === null ? NaN : Number(txt.replace(',', '.'));
      if (Number.isFinite(v) && v > 0 && v !== edit.value) void Commands.resizeCabinet(cab, edit.key, v, stretchMode).catch(() => undefined);
    };
    out.push(
      <foreignObject key="edit" x={edit.x - 50} y={edit.y - 14} width={100} height={28} style={{ pointerEvents: 'all' }}>
        <input
          className="dim-input"
          autoFocus
          defaultValue={String(edit.value)}
          onFocus={(e) => e.currentTarget.select()}
          onPointerDown={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') done(e.currentTarget.value);
            if (e.key === 'Escape') done(null);
          }}
          onBlur={(e) => done(e.currentTarget.value)}
        />
      </foreignObject>,
    );
  }
  // Distance between two selected objects (gap along the most separated axis).
  if (selection.length === 2) {
    const a = engine.boxOf(expandSubtrees(tree, [selection[0]]));
    const b = engine.boxOf(expandSubtrees(tree, [selection[1]]));
    if (!a.isEmpty() && !b.isEmpty()) {
      let best = -Infinity;
      let axis = 0;
      let lo = 0;
      let hi = 0;
      for (let i = 0; i < 3; i++) {
        const k = (['x', 'y', 'z'] as const)[i];
        const g1 = b.min[k] - a.max[k];
        const g2 = a.min[k] - b.max[k];
        const g = Math.max(g1, g2);
        if (g > best) {
          best = g;
          axis = i;
          [lo, hi] = g1 >= g2 ? [a.max[k], b.min[k]] : [b.max[k], a.min[k]];
        }
      }
      if (best > 0) {
        const c = a.getCenter(new THREE.Vector3()).add(b.getCenter(new THREE.Vector3())).multiplyScalar(0.5);
        const p1 = c.clone();
        const p2 = c.clone();
        p1.setComponent(axis, lo);
        p2.setComponent(axis, hi);
        out.push(<DimLine key="gap" a={engine.toScreen(p1)} b={engine.toScreen(p2)} label={`↔ ${fmt(best)}`} />);
      }
    }
  }
  return <g>{out}</g>;
}

export function SnapOverlay({ engine, hints, delta, movingIds }: { engine: ViewportEngine; hints: SnapHint[] | null; delta: Vec3 | null; movingIds: ObjectId[] }) {
  useFrame(engine);
  if (!hints || !delta) return null;
  const box = engine.boxOf(movingIds);
  if (box.isEmpty()) return null;
  box.translate(new THREE.Vector3(...delta));
  const c = box.getCenter(new THREE.Vector3());
  const items: JSX.Element[] = [];
  hints.forEach((h, i) => {
    if (h.kind === 'GRID') return;
    // A line lying in the snapped plane across the moving box.
    const other = [0, 1, 2].filter((k) => k !== h.axis)[0];
    const p1 = c.clone();
    const p2 = c.clone();
    p1.setComponent(h.axis, h.value);
    p2.setComponent(h.axis, h.value);
    p1.setComponent(other, box.min.getComponent(other) - 150);
    p2.setComponent(other, box.max.getComponent(other) + 150);
    const a = engine.toScreen(p1);
    const b = engine.toScreen(p2);
    const dot = engine.toScreen(p1.clone().lerp(p2, 0.5));
    items.push(
      <g key={i}>
        <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} className={h.kind === 'FACE' ? 'snap-line face' : 'snap-line'} />
        <circle cx={dot.x} cy={dot.y} r={4.5} className="snap-dot" />
      </g>,
    );
  });
  const tag = engine.toScreen(new THREE.Vector3(box.max.x, box.max.y, box.max.z));
  const label = `ΔX ${fmt(delta[0], 0)}  ΔY ${fmt(delta[1], 0)}  ΔZ ${fmt(delta[2], 0)}`;
  const kinds = Array.from(new Set(hints.map((h) => h.kind))).join(' · ');
  return (
    <g>
      {items}
      <g transform={`translate(${tag.x + 12} ${tag.y - 12})`}>
        <rect width={label.length * 6.6 + 16} height={36} rx={4} className="snap-tag" />
        <text x={8} y={15} className="snap-tag-text">{label}</text>
        <text x={8} y={29} className="snap-tag-sub">{kinds === 'GRID' ? 'Lưới' : kinds.replace('FACE', 'Mặt').replace('ALIGN', 'Căn').replace('CENTER', 'Tâm').replace('GRID', 'Lưới')}</text>
      </g>
    </g>
  );
}

const CONTACT_COLOR: Record<string, string> = { TOUCH: '#2f9e44', GAP: '#1c7ed6', PENETRATE: '#e03131' };

export function RelationsOverlay({ engine }: { engine: ViewportEngine }) {
  useFrame(engine);
  const { showRelations, relationFilter, selection, revision } = useUi();
  const [data, setData] = useState<{ relations: AssemblyRelation[]; names: Record<string, string> } | null>(null);
  const [hover, setHover] = useState<{ r: AssemblyRelation; x: number; y: number } | null>(null);
  const focus = selection.length === 1 ? selection[0] : null;
  useEffect(() => {
    if (!showRelations) return;
    let alive = true;
    Queries.relations().then((d) => alive && setData(d)).catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [showRelations, revision]);
  if (!showRelations || !data) return null;
  const rels = data.relations.filter((r) => relationFilter[r.contact] && (focus === null || r.source === focus || r.target === focus || !engine.entries.has(focus)));
  return (
    <g>
      {rels.map((r, i) => {
        const pts = r.contact_region.map((p) => engine.toScreen(new THREE.Vector3(...p)));
        if (pts.some((p) => !p.visible)) return null;
        const color = CONTACT_COLOR[r.contact];
        const sb = engine.worldBox(r.source);
        const tb = engine.worldBox(r.target);
        const sc = sb && engine.toScreen(sb.getCenter(new THREE.Vector3()));
        const tc = tb && engine.toScreen(tb.getCenter(new THREE.Vector3()));
        return (
          <g key={i} onMouseEnter={(e) => setHover({ r, x: e.clientX, y: e.clientY })} onMouseLeave={() => setHover(null)} style={{ pointerEvents: 'all' }}>
            {pts.length >= 3 && <polygon points={pts.map((p) => `${p.x},${p.y}`).join(' ')} fill={color} fillOpacity={0.35} stroke={color} strokeWidth={1.5} />}
            {sc && tc && <line x1={sc.x} y1={sc.y} x2={tc.x} y2={tc.y} stroke={color} strokeWidth={1.2} strokeDasharray="4 3" markerEnd="url(#rel-arrow)" />}
          </g>
        );
      })}
      {hover && (
        <foreignObject x={0} y={0} width="100%" height="100%" style={{ pointerEvents: 'none', overflow: 'visible' }}>
          <RelationTooltip r={hover.r} names={data.names} x={hover.x} y={hover.y} />
        </foreignObject>
      )}
    </g>
  );
}

function RelationTooltip({ r, names, x, y }: { r: AssemblyRelation; names: Record<string, string>; x: number; y: number }) {
  const host = useRef<HTMLDivElement>(null);
  const rect = host.current?.parentElement?.getBoundingClientRect();
  const left = x - (rect?.left ?? 0) + 14;
  const top = y - (rect?.top ?? 0) + 14;
  return (
    <div ref={host} className="rel-tip" style={{ left, top }}>
      <div className="rel-tip-title">
        {names[r.source]} → {names[r.target]}
      </div>
      <div className={`rel-chip ${r.contact.toLowerCase()}`}>{CONTACT_LABEL[r.contact]}</div>
      <div>{ORIENT_LABEL[r.orientation]}</div>
      <div>
        {REGION_LABEL[r.source_region]} → {REGION_LABEL[r.target_region]}
      </div>
      {r.contact === 'GAP' && <div>Khe hở: {fmt(r.gap_mm, 2)} mm</div>}
      {r.contact === 'PENETRATE' && <div>Xuyên: {fmt(r.penetration_mm, 2)} mm</div>}
      <div>Diện tích: {fmt(r.contact_area_mm2, 0)} mm²</div>
    </div>
  );
}

/** Direct manipulation of a cabinet's width/height/depth (drag → set_parameter). */
export function CabinetHandles({ engine, id }: { engine: ViewportEngine; id: ObjectId }) {
  useFrame(engine);
  const rev = useSceneRevision((s) => s.rev);
  const [frame, setFrame] = useState<{ world: THREE.Matrix4; dims: { width: number; height: number; depth: number } } | null>(null);
  const [drag, setDrag] = useState<{ key: 'width' | 'height' | 'depth'; value: number } | null>(null);
  useEffect(() => {
    let alive = true;
    Promise.all([transformOf(id), Queries.properties(id)])
      .then(([t, p]) => {
        if (!alive || p.kind !== 'CABINET' || p.locked) return setFrame(null);
        const get = (k: string) => Number(p.groups.flatMap((g) => g.fields).find((f) => f.key === k)?.value ?? 0);
        setFrame({ world: new THREE.Matrix4().fromArray(t.world_matrix), dims: { width: get('width'), height: get('height'), depth: get('depth') } });
      })
      .catch(() => setFrame(null));
    return () => {
      alive = false;
    };
  }, [id, rev]);
  const axes = useMemo(() => ({ width: new THREE.Vector3(1, 0, 0), height: new THREE.Vector3(0, 1, 0), depth: new THREE.Vector3(0, 0, 1) }), []);
  if (!frame) return null;
  const dims = { ...frame.dims };
  if (drag) dims[drag.key] = drag.value;
  const L = (x: number, y: number, z: number) => engine.toScreen(new THREE.Vector3(x, y, z).applyMatrix4(frame.world));
  const { width: w, height: h, depth: d } = dims;
  const handles: { key: 'width' | 'height' | 'depth'; p: P; label: string }[] = [
    { key: 'width', p: L(w, h / 2, d), label: 'Rộng' },
    { key: 'height', p: L(w / 2, h, d), label: 'Cao' },
    { key: 'depth', p: L(w, h, d / 2), label: 'Sâu' },
  ];
  const onDown = (key: 'width' | 'height' | 'depth', e: React.PointerEvent) => {
    e.stopPropagation();
    e.preventDefault();
    const start = frame.dims[key];
    const origin = new THREE.Vector3();
    const tip = axes[key].clone().multiplyScalar(100);
    const p0 = engine.toScreen(origin.clone().applyMatrix4(frame.world));
    const p1 = engine.toScreen(tip.applyMatrix4(frame.world));
    const ax = { x: p1.x - p0.x, y: p1.y - p0.y };
    const pxPer100 = Math.hypot(ax.x, ax.y) || 1;
    const sx = e.clientX;
    const sy = e.clientY;
    let value = start;
    const move = (ev: PointerEvent) => {
      const mm = (((ev.clientX - sx) * ax.x + (ev.clientY - sy) * ax.y) / pxPer100 / pxPer100) * 100;
      value = Math.max(50, Math.round((start + mm) / 5) * 5);
      setDrag({ key, value });
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setDrag(null);
      if (value !== start) void Commands.resizeCabinet(id, key, value, useUi.getState().stretchMode, 'END').catch(() => undefined);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const ghost = drag
    ? (() => {
        const c = [
          L(0, 0, 0), L(w, 0, 0), L(w, h, 0), L(0, h, 0),
          L(0, 0, d), L(w, 0, d), L(w, h, d), L(0, h, d),
        ];
        const e = [[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]];
        return e.map(([a, b], i) => <line key={i} x1={c[a].x} y1={c[a].y} x2={c[b].x} y2={c[b].y} className="ghost-line" />);
      })()
    : null;
  return (
    <g>
      {ghost}
      {handles.map((hd) =>
        hd.p.visible ? (
          <g key={hd.key} className="handle" onPointerDown={(e) => onDown(hd.key, e)} style={{ pointerEvents: 'all', cursor: 'grab' }}>
            <circle cx={hd.p.x} cy={hd.p.y} r={7} />
            <title>{hd.label}</title>
            {drag?.key === hd.key && (
              <g transform={`translate(${hd.p.x + 12} ${hd.p.y - 26})`}>
                <rect width={86} height={22} rx={4} className="handle-tag" />
                <text x={43} y={15} textAnchor="middle" className="handle-tag-text">
                  {fmt(drag.value, 0)} mm
                </text>
              </g>
            )}
          </g>
        ) : null,
      )}
    </g>
  );
}

export function OverlayDefs() {
  return (
    <defs>
      <marker id="dim-arrow" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
        <path d="M0 1 L9 5 L0 9 z" fill="#343a40" />
      </marker>
      <marker id="rel-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto">
        <path d="M0 1 L9 5 L0 9 z" fill="#495057" />
      </marker>
    </defs>
  );
}
