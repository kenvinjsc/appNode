// React host of the 3D viewport: wires pointer interaction, selection, gizmo,
// placement tool and overlays to the engine. All mutations go to the core.
import { useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { useUi } from '../app/uiStore';
import { Commands } from '../core-api/commands';
import { Queries } from '../core-api/queries';
import type { CabinetKind, ObjectId, SnapHint, Vec3 } from '../core-api/types';
import { Icon } from '../shared/icons';
import { CABINET_KINDS, fmt } from '../shared/i18n';
import { ViewportEngine } from './renderer/ViewportEngine';
import { TransformGizmo } from './gizmo/TransformGizmo';
import { CabinetHandles, DimensionOverlay, OverlayDefs, RelationsOverlay, SnapOverlay, useFrame } from './overlays/Overlays';
import type { VisualState } from './materials/materials';
import { installSceneSync, syncAll, refreshTree } from './sceneSync';
import { expandSubtrees, setEngine, useSceneRevision, View } from './viewportBus';
import { findNode } from '../app/uiStore';

/** Preset sizes, used only to draw the placement ghost (the core owns real presets). */
const GHOST: Record<CabinetKind, Vec3> = {
  WARDROBE: [1000, 2100, 598],
  BASE: [800, 720, 578],
  WALL: [800, 720, 338],
  DRAWER: [800, 720, 578],
  OPEN_SHELF: [800, 1600, 350],
  DOOR: [450, 720, 578],
};

interface PlaceState {
  kind: CabinetKind;
  pos: Vec3;
  screen: { x: number; y: number };
}

export function Viewport() {
  const host = useRef<HTMLDivElement>(null);
  const [engine, setEng] = useState<ViewportEngine | null>(null);
  const gizmo = useRef<TransformGizmo | null>(null);
  const ghost = useRef<THREE.LineSegments | null>(null);
  const [hints, setHints] = useState<{ h: SnapHint[] | null; d: Vec3 | null }>({ h: null, d: null });
  const [dragging, setDragging] = useState(false);
  const [box, setBox] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const [place, setPlace] = useState<PlaceState | null>(null);
  const ui = useUi();
  const rev = useSceneRevision((s) => s.rev);

  // ---------------------------------------------------------------- setup
  useEffect(() => {
    const el = host.current!;
    const e = new ViewportEngine(el);
    setEngine(e);
    setEng(e);
    installSceneSync();
    void syncAll().then(() => View.set('iso'));
    void refreshTree();
    gizmo.current = new TransformGizmo(e, {
      snap: async (id, delta) => {
        const s = useUi.getState();
        if (!s.snap) return { delta, hints: [] };
        try {
          return await Queries.snap(id, delta, s.gridSize);
        } catch {
          return null;
        }
      },
      commit: (id, t) => void Commands.setTransform(id, t).catch(() => void syncAll()),
      hints: (h, d) => setHints({ h, d }),
      dragging: setDragging,
    });
    return () => {
      setEngine(null);
      e.dispose();
    };
  }, []);

  // Shift+drag = box selection: disable orbit *before* OrbitControls sees the event.
  useEffect(() => {
    const el = host.current;
    if (!el || !engine) return;
    const cap = (e: PointerEvent) => {
      if (e.button === 0 && e.shiftKey) engine.controls.enabled = false;
    };
    el.addEventListener('pointerdown', cap, { capture: true });
    return () => el.removeEventListener('pointerdown', cap, { capture: true });
  }, [engine]);

  useEffect(() => {
    if (!engine) return;
    engine.setProjection(ui.projection);
    gizmo.current?.setCamera(engine.camera);
  }, [engine, ui.projection]);

  useEffect(() => {
    if (!engine) return;
    const b = engine.allBox();
    engine.setClipping(ui.section, b.isEmpty() ? 1000 : (b.min.y + b.max.y) / 2);
  }, [engine, ui.section]);

  // ---------------------------------------------------------- visual state
  const selectedAll = useMemo(() => expandSubtrees(ui.tree, ui.selection), [ui.tree, ui.selection]);
  useEffect(() => {
    if (!engine) return;
    const m = new Map<ObjectId, VisualState>();
    if (ui.hovered !== null) for (const id of expandSubtrees(ui.tree, [ui.hovered])) m.set(id, 'hover');
    for (const id of selectedAll) m.set(id, 'selected');
    if (ui.active !== null && engine.entries.has(ui.active)) m.set(ui.active, 'active');
    engine.setStates(m);
    engine.highlightFace(ui.face?.id ?? null, ui.face?.faceId ?? null);
    engine.highlightEdge(ui.edge?.id ?? null, ui.edge?.edgeId ?? null);
  }, [engine, selectedAll, ui.hovered, ui.active, ui.face, ui.edge, ui.tree, rev]);

  // ---------------------------------------------------------------- gizmo
  const toolType = ui.tool.type;
  useEffect(() => {
    const g = gizmo.current;
    if (!engine || !g || dragging) return;
    g.snapEnabled = ui.snap;
    const id = ui.active;
    if ((toolType === 'move' || toolType === 'rotate') && id !== null && ui.selection.length === 1) {
      const node = findNode(ui.tree, id)?.node;
      if (!node || node.locked) return g.detach();
      const ids = expandSubtrees(ui.tree, [id]);
      const b = engine.boxOf(ids);
      if (b.isEmpty()) return g.detach();
      const pivot = b.getCenter(new THREE.Vector3());
      if (toolType === 'move') pivot.y = b.min.y;
      void g.attach(id, ids, toolType === 'move' ? 'translate' : 'rotate', pivot);
    } else {
      g.detach();
    }
  }, [engine, toolType, ui.active, ui.selection.length, ui.tree, rev, dragging, ui.snap]);

  // ------------------------------------------------------------ placement
  useEffect(() => {
    if (!engine) return;
    if (ghost.current) {
      engine.overlay.remove(ghost.current);
      ghost.current = null;
    }
    if (ui.tool.type === 'place-cabinet') {
      const [w, h, d] = GHOST[ui.tool.kind];
      const geo = new THREE.EdgesGeometry(new THREE.BoxGeometry(w, h, d).translate(w / 2, h / 2, d / 2));
      ghost.current = new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: '#e8590c' }));
      ghost.current.visible = false;
      engine.overlay.add(ghost.current);
    }
    setPlace(null);
    engine.requestRender();
  }, [engine, ui.tool]);

  // ---------------------------------------------------------- pointer I/O
  const down = useRef<{ x: number; y: number; shift: boolean; button: number } | null>(null);
  const lastHover = useRef(0);

  const snapGrid = (v: number) => {
    const g = useUi.getState().gridSize || 10;
    return Math.round(v / g) * g;
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (!engine) return;
    down.current = { x: e.clientX, y: e.clientY, shift: e.shiftKey, button: e.button };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (!engine) return;
    const d = down.current;
    if (d && d.shift && d.button === 0) {
      setBox({ x0: d.x, y0: d.y, x1: e.clientX, y1: e.clientY });
      return;
    }
    const tool = useUi.getState().tool;
    if (tool.type === 'place-cabinet' && ghost.current && !place) {
      const p = engine.groundPoint(e.clientX, e.clientY);
      if (p) {
        ghost.current.position.set(snapGrid(p.x), 0, snapGrid(p.z));
        ghost.current.visible = true;
        useUi.getState().set({ cursor: [ghost.current.position.x, 0, ghost.current.position.z] });
        engine.requestRender();
      }
      return;
    }
    if (d || gizmo.current?.controls.dragging) return;
    const now = performance.now();
    if (now - lastHover.current < 40) return;
    lastHover.current = now;
    const hit = engine.pick(e.clientX, e.clientY);
    const hov = hit ? hit.id : null;
    const s = useUi.getState();
    if (s.hovered !== hov) s.set({ hovered: hov });
    const gp = hit?.point ?? engine.groundPoint(e.clientX, e.clientY);
    if (gp) s.set({ cursor: [Math.round(gp.x), Math.round(gp.y), Math.round(gp.z)] });
  };

  const onPointerUp = (e: React.PointerEvent) => {
    if (!engine) return;
    const d = down.current;
    down.current = null;
    if (!gizmo.current?.controls.dragging) engine.controls.enabled = true;
    if (!d) return;
    const s = useUi.getState();
    if (box) {
      const ids = engine.boxSelect(box.x0, box.y0, box.x1, box.y1);
      s.select(ids, e.ctrlKey || e.metaKey ? 'toggle' : 'add');
      setBox(null);
      return;
    }
    const moved = Math.hypot(e.clientX - d.x, e.clientY - d.y) > 4;
    if (moved || d.button !== 0 || gizmo.current?.controls.dragging || dragging) return;

    if (s.tool.type === 'place-cabinet') {
      const p = engine.groundPoint(e.clientX, e.clientY);
      if (!p) return;
      const r = host.current!.getBoundingClientRect();
      setPlace({ kind: s.tool.kind, pos: [snapGrid(p.x), 0, snapGrid(p.z)], screen: { x: e.clientX - r.left, y: e.clientY - r.top } });
      return;
    }
    const hit = engine.pick(e.clientX, e.clientY, { edges: s.selectionMode === 'edge' });
    if (!hit) {
      if (!e.ctrlKey && !e.shiftKey) s.clearSelection();
      return;
    }
    // Clicking a generated part selects its cabinet first; clicking again drills in.
    let id = hit.id;
    const n = findNode(s.tree, id);
    const cab = engine.entries.get(id)?.ro.cabinet ?? null;
    if (cab !== null && s.selectionMode === 'object' && !s.selection.includes(cab) && !s.selection.includes(id) && !e.altKey) {
      id = cab;
    }
    if (n?.node.locked && s.selectionMode !== 'object') return;
    const mode = e.ctrlKey || e.metaKey ? 'toggle' : e.shiftKey ? 'add' : 'replace';
    s.select([id], mode);
    if (s.selectionMode === 'face' && hit.faceId !== null) useUi.getState().set({ face: { id: hit.id, faceId: hit.faceId }, active: hit.id, selection: [hit.id] });
    if (s.selectionMode === 'edge' && hit.edgeId !== null) useUi.getState().set({ edge: { id: hit.id, edgeId: hit.edgeId }, active: hit.id, selection: [hit.id] });
  };

  const onContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    if (!engine || (down.current && Math.hypot(e.clientX - down.current.x, e.clientY - down.current.y) > 4)) return;
    const hit = engine.pick(e.clientX, e.clientY);
    const s = useUi.getState();
    if (!hit) return s.set({ contextMenu: null });
    const cab = engine.entries.get(hit.id)?.ro.cabinet ?? null;
    const id = s.selection.includes(hit.id) ? hit.id : cab !== null && !s.selection.includes(hit.id) && !s.selection.includes(cab) ? cab : hit.id;
    if (!s.selection.includes(id)) s.select([id]);
    s.set({ contextMenu: { x: e.clientX, y: e.clientY, id } });
  };

  const onDoubleClick = (e: React.MouseEvent) => {
    if (!engine) return;
    const hit = engine.pick(e.clientX, e.clientY);
    if (hit) useUi.getState().select([hit.id]);
  };

  const movingIds = gizmo.current?.attachedId != null ? expandSubtrees(ui.tree, [gizmo.current.attachedId]) : [];
  const singleCabinet = ui.selection.length === 1 && ui.tool.type === 'select' ? findNode(ui.tree, ui.selection[0])?.node : null;

  return (
    <div className="viewport" onContextMenu={onContextMenu}>
      <div
        ref={host}
        className={`viewport-canvas tool-${ui.tool.type}`}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => useUi.getState().set({ hovered: null })}
        onDoubleClick={onDoubleClick}
      />
      {engine && (
        <svg className="viewport-overlay">
          <OverlayDefs />
          <RelationsOverlay engine={engine} />
          {!dragging && <DimensionOverlay engine={engine} />}
          <SnapOverlay engine={engine} hints={hints.h} delta={hints.d} movingIds={movingIds} />
          {singleCabinet?.kind === 'CABINET' && !singleCabinet.locked && <CabinetHandles engine={engine} id={singleCabinet.id} />}
        </svg>
      )}
      {box && (
        <div
          className="box-select"
          style={{
            left: Math.min(box.x0, box.x1) - (host.current?.getBoundingClientRect().left ?? 0),
            top: Math.min(box.y0, box.y1) - (host.current?.getBoundingClientRect().top ?? 0),
            width: Math.abs(box.x1 - box.x0),
            height: Math.abs(box.y1 - box.y0),
          }}
        />
      )}
      {place && <PlacePopover state={place} onClose={() => setPlace(null)} />}
      {engine && <AxisGizmo engine={engine} />}
      <ViewToolbar />
      {ui.tool.type === 'place-cabinet' && !place && (
        <div className="viewport-hint">Nhấp để đặt {CABINET_KINDS.find((k) => ui.tool.type === 'place-cabinet' && k.kind === ui.tool.kind)?.label.toLowerCase()} · Esc để hủy</div>
      )}
      {(ui.tool.type === 'move' || ui.tool.type === 'rotate') && ui.selection.length !== 1 && (
        <div className="viewport-hint">Chọn một đối tượng để {ui.tool.type === 'move' ? 'di chuyển' : 'xoay'}</div>
      )}
    </div>
  );
}

function PlacePopover({ state, onClose }: { state: PlaceState; onClose: () => void }) {
  const [w, h, d] = GHOST[state.kind];
  const [vals, setVals] = useState({ width: String(w), height: String(h), depth: String(Math.round(d - 18)) });
  const first = useRef<HTMLInputElement>(null);
  useEffect(() => first.current?.select(), []);
  const submit = async () => {
    const num = (s: string) => Number(s.replace(',', '.'));
    try {
      const r = await Commands.createCabinet(state.kind, state.pos, { width: num(vals.width), height: num(vals.height), depth: num(vals.depth) });
      useUi.getState().select([r.id]);
      useUi.getState().setTool({ type: 'select' });
    } catch {
      /* toast shown */
    }
    onClose();
  };
  return (
    <form
      className="place-popover"
      style={{ left: state.screen.x + 12, top: state.screen.y + 12 }}
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
      onKeyDown={(e) => e.key === 'Escape' && onClose()}
    >
      <div className="place-title">{CABINET_KINDS.find((k) => k.kind === state.kind)?.label}</div>
      {(['width', 'height', 'depth'] as const).map((k, i) => (
        <label key={k}>
          <span>{k === 'width' ? 'Rộng' : k === 'height' ? 'Cao' : 'Sâu'}</span>
          <input ref={i === 0 ? first : undefined} value={vals[k]} onChange={(e) => setVals({ ...vals, [k]: e.target.value })} inputMode="decimal" />
          <em>mm</em>
        </label>
      ))}
      <div className="place-pos">
        Vị trí: {fmt(state.pos[0], 0)}, {fmt(state.pos[2], 0)}
      </div>
      <div className="place-actions">
        <button type="button" className="btn ghost" onClick={onClose}>
          Hủy
        </button>
        <button type="submit" className="btn primary">
          Tạo tủ
        </button>
      </div>
    </form>
  );
}

function AxisGizmo({ engine }: { engine: ViewportEngine }) {
  useFrame(engine);
  const q = engine.camera.quaternion.clone().invert();
  const axes = [
    { v: new THREE.Vector3(1, 0, 0), c: '#e03131', l: 'X' },
    { v: new THREE.Vector3(0, 1, 0), c: '#2f9e44', l: 'Y' },
    { v: new THREE.Vector3(0, 0, 1), c: '#1c7ed6', l: 'Z' },
  ].map((a) => ({ ...a, p: a.v.applyQuaternion(q) }));
  axes.sort((a, b) => a.p.z - b.p.z);
  return (
    <svg className="axis-gizmo" viewBox="-40 -40 80 80">
      {axes.map((a) => (
        <g key={a.l}>
          <line x1={0} y1={0} x2={a.p.x * 26} y2={-a.p.y * 26} stroke={a.c} strokeWidth={2.4} strokeLinecap="round" />
          <text x={a.p.x * 33} y={-a.p.y * 33 + 4} fill={a.c} fontSize={11} fontWeight={600} textAnchor="middle">
            {a.l}
          </text>
        </g>
      ))}
    </svg>
  );
}

function ViewToolbar() {
  const { projection, selection, tree, set } = useUi();
  return (
    <div className="view-toolbar">
      <button title="Phóng vừa (Shift+F)" onClick={() => View.fitAll()}>
        <Icon name="fit" />
      </button>
      <button title="Phóng vừa vùng chọn (F)" onClick={() => View.fit(selection, tree)}>
        <Icon name="inspect" />
      </button>
      <button title="Mặt trước (1)" onClick={() => View.set('front', selection, tree)}>
        <Icon name="front" />
      </button>
      <button title="Mặt bên phải (4)" onClick={() => View.set('right', selection, tree)}>
        <Icon name="side" />
      </button>
      <button title="Mặt trên (5)" onClick={() => View.set('top', selection, tree)}>
        <Icon name="top" />
      </button>
      <button title="Phối cảnh (0)" onClick={() => View.set('iso', selection, tree)}>
        <Icon name="view" />
      </button>
      <select value={projection} onChange={(e) => set({ projection: e.target.value as 'perspective' | 'orthographic' })}>
        <option value="perspective">Phối cảnh</option>
        <option value="orthographic">Trực giao</option>
      </select>
    </div>
  );
}
