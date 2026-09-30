// 2D editor layer (front view of the current cabinet): editable dimensions with
// LOCK / AUTO / % modes, drag of dividers and shelves (local PREVIEW, COMMIT on
// release), zone pinning and the zone context menu. Every edit is a core
// request; this layer only shows positions the core returned.
import { useRef, useState } from 'react';
import { findNode, useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { BayInfo, BayMode, FrontBay, ObjectId, PanelSide, Vec3, ZonesInfo } from '../../core-api/types';
import { fmt } from '../../shared/i18n';
import type { Item } from './Drawing2D';

interface Rect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** Front projection (world X, −world Y) of a cabinet-frame box. */
function project(m: number[], min: Vec3, size: Vec3): Rect {
  const r: Rect = { x0: Infinity, y0: Infinity, x1: -Infinity, y1: -Infinity };
  for (let i = 0; i < 8; i++) {
    const p = [min[0] + (i & 1 ? size[0] : 0), min[1] + (i & 2 ? size[1] : 0), min[2] + (i & 4 ? size[2] : 0)];
    const x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    const y = -(m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13]);
    r.x0 = Math.min(r.x0, x);
    r.x1 = Math.max(r.x1, x);
    r.y0 = Math.min(r.y0, y);
    r.y1 = Math.max(r.y1, y);
  }
  return r;
}

const MODE_TAG: Record<BayMode, string> = { LOCK: 'KHÓA', AUTO: 'AUTO', PERCENT: '%' };
const NEXT: Record<BayMode, BayMode> = { LOCK: 'PERCENT', PERCENT: 'AUTO', AUTO: 'LOCK' };

type Edit =
  | { kind: 'bay'; bay: BayInfo; x: number; y: number }
  | { kind: 'front'; fb: FrontBay; x: number; y: number }
  | { kind: 'cab'; name: 'width' | 'height'; value: number; x: number; y: number };

interface Drag {
  id: ObjectId;
  axis: 0 | 1;
  base: number;
  total: number;
  from: number;
  before: number;
  moved: boolean;
}

export function EditLayer({ info, items, unit, svg }: { info: ZonesInfo; items: Item[]; unit: number; svg: SVGSVGElement | null }) {
  const { pinned, set, select, selection, tree, resizeMode } = useUi();
  type EDrag = { id: ObjectId; side: PanelSide; from: number; delta: number };
  const [edrag, setEdrag] = useState<EDrag | null>(null);
  const edragRef = useRef<EDrag | null>(null);
  const [edit, setEdit] = useState<Edit | null>(null);
  const [hover, setHover] = useState<number | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const dragRef = useRef<Drag | null>(null);
  type FDrag = { uid: number; index: number; base: number; total: number; from: number; before: number; moved: boolean };
  const [fdrag, setFdrag] = useState<FDrag | null>(null);
  const fdragRef = useRef<FDrag | null>(null);
  const m = info.matrix;
  // Editing assumes an unrotated cabinet (front view = cabinet XY).
  const straight = Math.abs(m[0] - 1) < 1e-6 && Math.abs(m[5] - 1) < 1e-6;
  const zoneRect = (id: number) => {
    const z = info.zones.find((q) => q.id === id);
    return z ? project(m, z.min, z.size) : null;
  };
  const cab = project(m, [0, 0, 0], info.size);
  const pins = pinned.cabinet === info.cabinet ? pinned.zones : [];
  const fs = unit * 1.6;

  const toSvg = (e: { clientX: number; clientY: number }) => {
    if (!svg) return { x: 0, y: 0 };
    const pt = svg.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const p = pt.matrixTransform(svg.getScreenCTM()!.inverse());
    return { x: p.x, y: p.y };
  };

  const commitValue = async (raw: string) => {
    const e = edit;
    setEdit(null);
    if (!e) return;
    const txt = raw.trim().replace(',', '.');
    if (!txt) return;
    const pct = txt.endsWith('%');
    const v = Number(pct ? txt.slice(0, -1) : txt);
    if (!Number.isFinite(v)) return;
    if (e.kind === 'cab') {
      await Commands.resizeCabinet(info.cabinet, e.name, v, useUi.getState().stretchMode).catch(() => undefined);
      return;
    }
    if (e.kind === 'front') {
      const f = e.fb;
      if (pct) await Commands.setDrawerHeight(info.cabinet, f.uid, f.index, 'PERCENT', v).catch(() => undefined);
      else if (f.mode === 'PERCENT') await Commands.setDrawerHeight(info.cabinet, f.uid, f.index, 'PERCENT', f.usable > 0 ? (v / f.usable) * 100 : 0).catch(() => undefined);
      else await Commands.setDrawerHeight(info.cabinet, f.uid, f.index, 'LOCK', v).catch(() => undefined);
      return;
    }
    const b = e.bay;
    if (pct) await Commands.setBay(info.cabinet, b.zone, b.index, 'PERCENT', v).catch(() => undefined);
    else if (b.mode === 'PERCENT') await Commands.setBay(info.cabinet, b.zone, b.index, 'PERCENT', b.usable > 0 ? (v / b.usable) * 100 : 0).catch(() => undefined);
    else await Commands.setBay(info.cabinet, b.zone, b.index, 'LOCK', v).catch(() => undefined);
  };

  const label = (x: number, y: number, text: string, onClick: () => void, title: string, vertical = false) => (
    <text
      x={x}
      y={y}
      className="d2e-num"
      textAnchor="middle"
      dominantBaseline="middle"
      fontSize={fs}
      transform={vertical ? `rotate(-90 ${x} ${y})` : undefined}
      onClick={(ev) => {
        ev.stopPropagation();
        onClick();
      }}
    >
      <title>{title}</title>
      {text}
    </text>
  );

  // Bay chains, one per split along X (dividers) or Y (shelves).
  const chains: JSX.Element[] = [];
  const bySplit = new Map<number, BayInfo[]>();
  for (const b of info.bays) if (b.axis !== 2) bySplit.set(b.zone, [...(bySplit.get(b.zone) ?? []), b]);
  for (const [zone, bays] of bySplit) {
    const parent = zoneRect(zone);
    if (!parent) continue;
    bays.sort((a, b) => a.index - b.index);
    for (const b of bays) {
      const r = zoneRect(b.child);
      if (!r) continue;
      const d = drag && info.panels.find((p) => p.id === drag.id);
      // Live values while dragging a panel of this split.
      const pos = d ? info.positions.find((p) => p.uid === d.uid) : null;
      let size = b.size;
      if (drag && pos && pos.zone === zone) {
        const i = info.positions.filter((p) => p.zone === zone).findIndex((p) => p.uid === pos.uid);
        if (b.index === i) size = drag.before;
        if (b.index === i + 1) size = drag.total - drag.before;
      }
      const mode = b.mode;
      const tag = mode ? MODE_TAG[mode] : '·';
      const horiz = b.axis === 0;
      const lx = horiz ? (r.x0 + r.x1) / 2 : r.x0 + unit * 2.2;
      const ly = horiz ? r.y1 - unit * 2 : (r.y0 + r.y1) / 2;
      chains.push(
        <g key={`${zone}-${b.index}`} className={`d2e-bay ${mode === 'LOCK' ? 'lock' : mode === 'PERCENT' ? 'pct' : ''}`}>
          {horiz ? (
            <line x1={r.x0} y1={ly + unit * 0.9} x2={r.x1} y2={ly + unit * 0.9} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
          ) : (
            <line x1={lx + unit * 0.9} y1={r.y0} x2={lx + unit * 0.9} y2={r.y1} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
          )}
          {label(lx, ly, fmt(size, 1), () => setEdit({ kind: 'bay', bay: b, x: lx, y: ly }), 'Bấm để nhập kích thước khoang (mm, hoặc 40%)', !horiz)}
          <text
            x={horiz ? lx : lx}
            y={horiz ? ly - unit * 1.35 : ly}
            dx={horiz ? 0 : unit * 2.3}
            className="d2e-mode"
            textAnchor="middle"
            dominantBaseline="middle"
            fontSize={unit * 1.05}
            onClick={(ev) => {
              ev.stopPropagation();
              void Commands.setBay(info.cabinet, b.zone, b.index, mode ? NEXT[mode] : 'LOCK').catch(() => undefined);
            }}
          >
            <title>Chế độ khoang: KHÓA (giữ mm) → % (giữ tỉ lệ) → AUTO (chia phần còn lại). Bấm để đổi.</title>
            {tag}
          </text>
        </g>,
      );
    }
  }

  // Drawer stacks: height chain on the right of the fronts + drag of the dividers.
  const fronts: JSX.Element[] = [];
  const byStack = new Map<number, FrontBay[]>();
  for (const f of info.front_bays) if (!byStack.has(f.uid) || !byStack.get(f.uid)!.some((x) => x.index === f.index)) byStack.set(f.uid, [...(byStack.get(f.uid) ?? []), f]);
  for (const [uid, list] of byStack) {
    list.sort((a, b) => a.index - b.index);
    list.forEach((f, i) => {
      const r = project(m, [f.x0, f.start, f.z], [f.x1 - f.x0, f.size, 0]);
      let size = f.size;
      if (fdrag && fdrag.uid === uid) {
        if (i === fdrag.index) size = fdrag.before;
        if (i === fdrag.index + 1) size = fdrag.total - fdrag.before;
      }
      const lx = r.x1 + unit * 2.6;
      const ly = (r.y0 + r.y1) / 2;
      const mode = f.mode;
      fronts.push(
        <g key={`f${uid}-${i}`} className={`d2e-bay ${mode === 'LOCK' ? 'lock' : mode === 'PERCENT' ? 'pct' : ''}`}>
          <line x1={r.x1 + unit * 1.2} y1={r.y0} x2={r.x1 + unit * 1.2} y2={r.y1} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
          {label(lx, ly, fmt(size, 1), () => setEdit({ kind: 'front', fb: f, x: lx, y: ly }), 'Cao mặt ngăn kéo: nhập mm hoặc 40%', true)}
          <text
            x={lx}
            y={ly}
            dx={unit * 2.3}
            className="d2e-mode"
            textAnchor="middle"
            dominantBaseline="middle"
            fontSize={unit * 1.05}
            onClick={(ev) => {
              ev.stopPropagation();
              void Commands.setDrawerHeight(info.cabinet, uid, f.index, mode ? NEXT[mode] : 'LOCK').catch(() => undefined);
            }}
          >
            <title>Chế độ ngăn: KHÓA → % → AUTO. Bấm để đổi.</title>
            {mode ? MODE_TAG[mode] : '·'}
          </text>
        </g>,
      );
      // Divider handle between this front and the next.
      if (i + 1 < list.length && straight) {
        const next = list[i + 1];
        const gapTop = project(m, [f.x0, f.start + f.size, f.z], [f.x1 - f.x0, Math.max(next.start - (f.start + f.size), 0.5), 0]);
        const shift = fdrag && fdrag.uid === uid && fdrag.index === i ? -(fdrag.before - f.size) : 0;
        const hh = Math.max(gapTop.y1 - gapTop.y0, unit * 0.7);
        fronts.push(
          <rect
            key={`fh${uid}-${i}`}
            className={`d2e-handle y ${fdrag && fdrag.uid === uid && fdrag.index === i ? 'on' : ''}`}
            x={gapTop.x0}
            y={(gapTop.y0 + gapTop.y1) / 2 - hh / 2 + shift}
            width={gapTop.x1 - gapTop.x0}
            height={hh}
            onPointerDown={(e) => {
              if (e.button !== 0) return;
              e.stopPropagation();
              (e.target as Element).setPointerCapture(e.pointerId);
              const d = { uid, index: i, base: f.size, total: f.size + next.size, from: toSvg(e).y, before: f.size, moved: false };
              fdragRef.current = d;
              setFdrag(d);
            }}
            onPointerMove={(e) => {
              const d = fdragRef.current;
              if (!d || d.uid !== uid || d.index !== i) return;
              const delta = d.from - toSvg(e).y;
              let before = d.base + delta;
              if (Math.abs(before - d.total / 2) < unit * 0.8) before = d.total / 2;
              else before = e.shiftKey ? Math.round(before / 10) * 10 : Math.round(before * 2) / 2;
              before = Math.min(Math.max(before, 1), d.total - 1);
              const n2 = { ...d, before, moved: d.moved || Math.abs(delta) > unit * 0.2 };
              fdragRef.current = n2;
              setFdrag(n2);
            }}
            onPointerUp={() => {
              const d = fdragRef.current;
              fdragRef.current = null;
              setFdrag(null);
              if (d && d.moved && Math.abs(d.before - d.base) > 0.01) void Commands.moveDrawerDivider(info.cabinet, d.uid, d.index, d.before).catch(() => undefined);
            }}
          >
            <title>Kéo để đổi chiều cao 2 ngăn kề (Shift: bước 10 mm)</title>
          </rect>,
        );
      }
    });
  }

  // Edge handles of the selected cabinet part (kéo 4 cạnh).
  const edges: JSX.Element[] = [];
  const selId = selection.length === 1 ? selection[0] : null;
  const selNode = selId !== null ? findNode(tree, selId) : null;
  const selItem = selId !== null ? items.find((i) => i.id === selId) : undefined;
  if (straight && selItem && selNode?.node.kind === 'PANEL' && selNode.node.generated && selNode.parent === info.cabinet) {
    const it = selItem;
    const d = edrag && edrag.id === it.id ? edrag : null;
    // Preview rectangle (2D: x right, y down).
    const px0 = it.x0 - (d?.side === 'LEFT' ? d.delta : 0);
    const px1 = it.x1 + (d?.side === 'RIGHT' ? d.delta : 0);
    const py0 = it.y0 - (d?.side === 'TOP' ? d.delta : 0);
    const py1 = it.y1 + (d?.side === 'BOTTOM' ? d.delta : 0);
    const s = unit * 0.9;
    const handle = (side: PanelSide, cx: number, cy: number) => (
      <rect
        key={side}
        className={`d2e-edge ${side === 'LEFT' || side === 'RIGHT' ? 'x' : 'y'} ${d?.side === side ? 'on' : ''}`}
        x={cx - s / 2}
        y={cy - s / 2}
        width={s}
        height={s}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          e.stopPropagation();
          (e.target as Element).setPointerCapture(e.pointerId);
          const p = toSvg(e);
          const v: EDrag = { id: it.id, side, from: side === 'LEFT' || side === 'RIGHT' ? p.x : p.y, delta: 0 };
          edragRef.current = v;
          setEdrag(v);
        }}
        onPointerMove={(e) => {
          const v = edragRef.current;
          if (!v || v.side !== side) return;
          const p = toSvg(e);
          const raw = side === 'LEFT' || side === 'RIGHT' ? p.x - v.from : p.y - v.from;
          // Growing: right / bottom (svg) move +, left / top move −.
          let delta = side === 'RIGHT' || side === 'BOTTOM' ? raw : -raw;
          delta = e.shiftKey ? Math.round(delta / 10) * 10 : Math.round(delta * 2) / 2;
          const n = { ...v, delta };
          edragRef.current = n;
          setEdrag(n);
        }}
        onPointerUp={() => {
          const v = edragRef.current;
          edragRef.current = null;
          setEdrag(null);
          if (v && Math.abs(v.delta) > 0.01) {
            // 2D "bottom" is −Y in the cabinet: map svg sides to cabinet sides.
            const side3: PanelSide = v.side === 'TOP' ? 'TOP' : v.side === 'BOTTOM' ? 'BOTTOM' : v.side;
            void Commands.resizePanelSide(v.id, side3, v.delta, resizeMode === 'constrained').catch(() => undefined);
          }
        }}
      >
        <title>{`Kéo cạnh ${side === 'LEFT' ? 'trái' : side === 'RIGHT' ? 'phải' : side === 'TOP' ? 'trên' : 'dưới'} (${resizeMode === 'constrained' ? 'giữ ràng buộc' : 'tự do'}; Shift: bước 10 mm)`}</title>
      </rect>
    );
    edges.push(
      <g key="edges" className="d2e-edges">
        {d && <rect className="d2e-ghost" x={px0} y={py0} width={Math.max(px1 - px0, 0.5)} height={Math.max(py1 - py0, 0.5)} />}
        {d && (
          <text className="d2e-num" x={(px0 + px1) / 2} y={py0 - unit * 1.2} textAnchor="middle" fontSize={fs}>
            {fmt(px1 - px0, 1)} × {fmt(py1 - py0, 1)} ({d.delta > 0 ? '+' : ''}
            {fmt(d.delta, 1)})
          </text>
        )}
        {handle('LEFT', px0, (py0 + py1) / 2)}
        {handle('RIGHT', px1, (py0 + py1) / 2)}
        {handle('TOP', (px0 + px1) / 2, py0)}
        {handle('BOTTOM', (px0 + px1) / 2, py1)}
      </g>,
    );
  }

  // Leaf zones: click = pin, Ctrl+click = add, right-click = quick build menu.
  const leaves = info.zones.filter((z) => z.leaf);

  // Drag handles over split panels.
  const handles = straight
    ? info.panels.flatMap(({ uid, id }) => {
        const it = items.find((i) => i.id === id);
        const pos = info.positions.find((p) => p.uid === uid);
        if (!it || !pos || pos.axis === 2) return [];
        const axis = pos.axis as 0 | 1;
        const dx = drag?.id === id ? (axis === 0 ? drag.before - drag.base : -(drag.before - drag.base)) : 0;
        return [
          <rect
            key={id}
            className={`d2e-handle ${axis === 0 ? 'x' : 'y'} ${drag?.id === id ? 'on' : ''}`}
            x={it.x0 + (axis === 0 ? dx : 0)}
            y={it.y0 + (axis === 1 ? dx : 0)}
            width={Math.max(it.x1 - it.x0, unit * 0.6)}
            height={Math.max(it.y1 - it.y0, unit * 0.6)}
            onPointerDown={(e) => {
              if (e.button !== 0) return;
              e.stopPropagation();
              (e.target as Element).setPointerCapture(e.pointerId);
              const p = toSvg(e);
              const d: Drag = { id, axis, base: pos.cell_before, total: pos.cell_before + pos.cell_after, from: axis === 0 ? p.x : p.y, before: pos.cell_before, moved: false };
              dragRef.current = d;
              setDrag(d);
            }}
            onPointerMove={(e) => {
              const d = dragRef.current;
              if (!d || d.id !== id) return;
              const p = toSvg(e);
              const delta = d.axis === 0 ? p.x - d.from : d.from - p.y;
              let before = d.base + delta;
              const half = d.total / 2;
              if (Math.abs(before - half) < unit * 0.8) before = half; // snap: chia đều
              else before = e.shiftKey ? Math.round(before / 10) * 10 : Math.round(before * 2) / 2;
              before = Math.min(Math.max(before, 1), d.total - 1);
              const next = { ...d, before, moved: d.moved || Math.abs(delta) > unit * 0.2 };
              dragRef.current = next;
              setDrag(next);
            }}
            onPointerUp={(e) => {
              const d = dragRef.current;
              dragRef.current = null;
              setDrag(null);
              if (!d) return;
              if (!d.moved) {
                select([id], e.ctrlKey ? 'toggle' : 'replace');
                return;
              }
              if (Math.abs(d.before - d.base) > 0.01) void Commands.moveSplitPanel(id, d.before).catch(() => undefined);
            }}
          >
            <title>{axis === 0 ? 'Kéo ngang để dời vách (Shift: bước 10 mm)' : 'Kéo dọc để dời kệ (Shift: bước 10 mm)'}</title>
          </rect>,
        ];
      })
    : [];

  return (
    <g className="d2e">
      {leaves.map((z) => {
        const r = project(m, z.min, z.size);
        const on = pins.includes(z.id);
        return (
          <rect
            key={z.id}
            className={`d2e-zone ${on ? 'on' : ''} ${hover === z.id ? 'hover' : ''}`}
            x={r.x0}
            y={r.y0}
            width={r.x1 - r.x0}
            height={r.y1 - r.y0}
            onMouseEnter={() => setHover(z.id)}
            onMouseLeave={() => setHover(null)}
            onClick={(e) => {
              const cur = useUi.getState().pinned;
              const same = cur.cabinet === info.cabinet;
              const zones = e.ctrlKey || e.metaKey ? (same ? (cur.zones.includes(z.id) ? cur.zones.filter((x) => x !== z.id) : [...cur.zones, z.id]) : [z.id]) : [z.id];
              set({ pinned: { cabinet: info.cabinet, zones } });
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              set({ pinned: { cabinet: info.cabinet, zones: [z.id] }, zoneMenu: { x: e.clientX, y: e.clientY, cabinet: info.cabinet, zone: z.id }, contextMenu: null });
            }}
          >
            <title>{`Vùng #${z.id} · ${fmt(z.size[0], 1)} × ${fmt(z.size[1], 1)} — click ghim, chuột phải: dựng nhanh`}</title>
          </rect>
        );
      })}
      {handles}
      {chains}
      {fronts}
      {edges}
      {/* Cabinet W / H (editable). */}
      <g className="d2e-cab">
        <line x1={cab.x0} y1={cab.y0 - unit * 2.5} x2={cab.x1} y2={cab.y0 - unit * 2.5} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
        <line x1={cab.x0 - unit * 2.5} y1={cab.y0} x2={cab.x0 - unit * 2.5} y2={cab.y1} markerStart="url(#d2a)" markerEnd="url(#d2a)" />
        {label((cab.x0 + cab.x1) / 2, cab.y0 - unit * 3.4, fmt(info.size[0], 1), () => setEdit({ kind: 'cab', name: 'width', value: info.size[0], x: (cab.x0 + cab.x1) / 2, y: cab.y0 - unit * 3.4 }), 'Bấm để đổi chiều rộng tủ')}
        {label(cab.x0 - unit * 3.4, (cab.y0 + cab.y1) / 2, fmt(info.size[1], 1), () => setEdit({ kind: 'cab', name: 'height', value: info.size[1], x: cab.x0 - unit * 3.4, y: (cab.y0 + cab.y1) / 2 }), 'Bấm để đổi chiều cao tủ', true)}
      </g>
      {edit && (
        <foreignObject x={edit.x - unit * 5} y={edit.y - unit * 1.4} width={unit * 10} height={unit * 2.8}>
          <input
            className="d2e-input"
            autoFocus
            style={{ fontSize: `${fs}px` }}
            defaultValue={
              edit.kind === 'cab'
                ? String(edit.value)
                : edit.kind === 'front'
                  ? edit.fb.mode === 'PERCENT'
                    ? `${Math.round(edit.fb.value * 100) / 100}%`
                    : String(Math.round(edit.fb.size * 10) / 10)
                  : edit.bay.mode === 'PERCENT'
                    ? `${Math.round(edit.bay.value * 100) / 100}%`
                    : String(Math.round(edit.bay.size * 10) / 10)
            }
            onFocus={(e) => e.currentTarget.select()}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === 'Enter') void commitValue(e.currentTarget.value);
              if (e.key === 'Escape') setEdit(null);
            }}
            onBlur={(e) => void commitValue(e.currentTarget.value)}
          />
        </foreignObject>
      )}
    </g>
  );
}
