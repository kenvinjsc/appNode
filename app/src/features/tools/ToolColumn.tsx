// "Tool" column: switches + the 19 cabinet tools. Tools only collect parameters;
// the resulting machining/modification is stored by the core as part mods.
import { useEffect, useRef, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Actions } from '../../app/actions';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { Corner, EdgeSide, FaceSide, FlatPanel, MachiningFeature, ObjectId, RelationKind } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { Num, Radio, Steps } from '../../shared/ui';
import { cabinetOf } from '../cabinet/useZones';

const TOOLS: [string, string, boolean][] = [
  ['01', 'Xoá tấm', true],
  ['02', 'Ẩn tấm', true],
  ['03', 'Khấu góc tủ', true],
  ['04', 'Cắt tự do', true],
  ['05', 'Cắt có liên kết', false],
  ['06', 'Hợp tấm', true],
  ['07', 'Ghép bề mặt', false],
  ['08', 'Khấu bề mặt', true],
  ['09', 'Cắt theo tấm', true],
  ['10', 'Bo/Vác góc', true],
  ['11', 'Co giãn tấm', true],
  ['12', 'Tạo Rãnh', true],
  ['13', 'Đảo phủ/lọt', true],
  ['14', 'Ghép bề dày', true],
  ['15', 'Mộng đan tay', false],
  ['16', 'Bào rãnh LED', true],
  ['17', 'Tạo ray kéo', true],
  ['18', 'Tạo Vbit', true],
  ['19', 'Xoá tool cả tủ', true],
  ['20', 'Chia tấm', true],
  ['21', 'Cung cạnh', true],
  ['22', 'Biên dạng tự do', true],
  ['23', 'Quan hệ 2 tấm', true],
];

export function ToolColumn() {
  const { toolId, set, showZones, isolate } = useUi();
  return (
    <div className="panel scene-tree tool-column">
      <div className="panel-header">
        <Icon name="settings" size={16} />
        <span>Thiết kế tủ · Tool</span>
      </div>
      <div className="tool-switches">
        <label className="check toggle-row">
          <input type="checkbox" checked={showZones} onChange={(e) => set({ showZones: e.target.checked })} /> Ẩn/Hiện vùng ~
        </label>
        <label className="check toggle-row">
          <input type="checkbox" checked={isolate} onChange={(e) => set({ isolate: e.target.checked })} /> Cô lập tủ Alt+~
        </label>
      </div>
      <div className="tool-list-col">
        <button className="btn primary wide-sm" onClick={() => set({ revision: useUi.getState().revision + 1 })} title="Tính lại vùng trống">
          Dọn Zone
        </button>
        {TOOLS.map(([n, label, ok]) => (
          <div key={n}>
            <button className={`tool-btn ${toolId === n ? 'on' : ''}`} disabled={!ok} title={ok ? label : 'Sắp có'} onClick={() => set({ toolId: toolId === n ? null : n })}>
              <span className="tool-no">{n}.</span> {label}
            </button>
            {toolId === n && <ToolForm id={n} />}
          </div>
        ))}
      </div>
    </div>
  );
}

function useTargets() {
  const { selection, tree } = useUi();
  return { selection, tree, cabinet: cabinetOf(tree, selection[0] ?? null) };
}

function ToolForm({ id }: { id: string }) {
  switch (id) {
    case '01':
      return <DeleteTool />;
    case '02':
      return <HideTool />;
    case '03':
      return <NotchTool />;
    case '04':
      return <FreeCutTool />;
    case '06':
      return <MergeTool />;
    case '08':
      return <PocketTool />;
    case '09':
      return <CutByPanelTool />;
    case '10':
      return <CornerTool />;
    case '14':
      return <LaminateTool />;
    case '11':
      return <StretchTool />;
    case '12':
      return <GrooveTool />;
    case '13':
      return <FlipJoinTool />;
    case '16':
      return <GrooveLineTool tool="16. Bào rãnh LED" defaults={{ width: 10, depth: 8, offset: 30 }} />;
    case '17':
      return <SlideTool />;
    case '18':
      return <GrooveLineTool tool="18. Tạo Vbit" defaults={{ width: 6, depth: 3, offset: 100 }} />;
    case '19':
      return <ClearToolsTool />;
    case '20':
      return <SplitPartTool />;
    case '21':
      return <EdgeArcTool />;
    case '22':
      return <PolygonTool />;
    case '23':
      return <RelationTool />;
    default:
      return null;
  }
}

function DeleteTool() {
  const { selection } = useTargets();
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm cần xóa', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <button className="btn danger-btn" disabled={!selection.length} onClick={() => confirm(`Xóa ${selection.length} tấm? Vùng kề sẽ được gộp lại.`) && void Actions.delete()}>
        Xóa tấm
      </button>
    </div>
  );
}

function HideTool() {
  const { selection } = useTargets();
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm cần ẩn', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <div className="row-btns">
        <button className="btn" disabled={!selection.length} onClick={() => void Commands.setVisible(selection, false).catch(() => undefined)}>
          Ẩn tấm
        </button>
        <button className="btn" disabled={!selection.length} onClick={() => void Commands.setVisible(selection, true).catch(() => undefined)}>
          Hiện lại
        </button>
      </div>
    </div>
  );
}

/** Apply generated features to every selected panel through the core's part mods. */
async function addFeatures(ids: number[], tool: string, make: (w: number, h: number, t: number) => MachiningFeature[]) {
  for (const id of ids) {
    const sheet = await Queries.properties(id).catch(() => null);
    if (!sheet || sheet.kind !== 'PANEL') continue;
    const v = (k: string) => Number(sheet.groups.flatMap((g) => g.fields).find((f) => f.key === k)?.value ?? 0);
    await Commands.setPartMod(id, { add_features: make(v('width'), v('height'), v('thickness')), tool }).catch(() =>
      Commands.addFeature(id, make(v('width'), v('height'), v('thickness'))[0]).catch(() => undefined),
    );
  }
}

function NotchTool() {
  const { selection } = useTargets();
  const [corner, setCorner] = useState<'TL' | 'TR' | 'BL' | 'BR'>('TR');
  const [w, setW] = useState(100);
  const [d, setD] = useState(100);
  const apply = () =>
    void addFeatures(selection, '03. Khấu góc tủ', (pw, ph, t) => [
      {
        type: 'POCKET',
        x: corner.endsWith('L') ? 0 : pw - w,
        y: corner.startsWith('B') ? 0 : ph - d,
        width: w,
        height: d,
        depth: t + 1,
        side: 'A',
        corner_radius: 0,
      },
    ]);
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <Radio value={corner} onChange={setCorner} options={[['TL', 'Góc trên trái'], ['TR', 'Góc trên phải'], ['BL', 'Góc dưới trái'], ['BR', 'Góc dưới phải']]} />
      <div className="form-row"><label>Rộng khấu</label><Num value={w} onCommit={setW} /></div>
      <div className="form-row"><label>Sâu khấu</label><Num value={d} onCommit={setD} /></div>
      <button className="btn primary" disabled={!selection.length} onClick={apply}>Khấu góc</button>
    </div>
  );
}

function PocketTool() {
  const { selection } = useTargets();
  const [p, setP] = useState({ x: 50, y: 50, width: 100, height: 40, depth: 8 });
  const [side, setSide] = useState<FaceSide>('A');
  const apply = () => void addFeatures(selection, '08. Khấu bề mặt', () => [{ type: 'POCKET', ...p, side, corner_radius: 0 }]);
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      {(['x', 'y', 'width', 'height', 'depth'] as const).map((k) => (
        <div className="form-row" key={k}>
          <label>{{ x: 'Cách trái', y: 'Cách dưới', width: 'Rộng', height: 'Cao', depth: 'Sâu' }[k]}</label>
          <Num value={p[k]} onCommit={(v) => setP({ ...p, [k]: v })} />
        </div>
      ))}
      <Radio value={side} onChange={setSide} options={[['A', 'Mặt A'], ['B', 'Mặt B']]} />
      <button className="btn primary" disabled={!selection.length} onClick={apply}>Khấu bề mặt</button>
    </div>
  );
}

function StretchTool() {
  const { selection } = useTargets();
  const [edge, setEdge] = useState<EdgeSide | null>(null);
  const [mm, setMm] = useState('');
  const [bonus, setBonus] = useState(0);
  const [saved, setSaved] = useState(0);
  const idx: Record<EdgeSide, number> = { LEFT: 0, RIGHT: 1, BOTTOM: 2, TOP: 3 };
  const apply = async () => {
    const v = Number(mm.replace(',', '.')) + bonus;
    if (!edge || !Number.isFinite(v) || v === 0) return;
    const d: [number, number, number, number] = [0, 0, 0, 0];
    d[idx[edge]] = v;
    for (const id of selection) await Commands.setPartMod(id, { extend_delta: d }).catch(() => undefined);
    setSaved(saved + 1);
    setMm('');
  };
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Chọn 01 hoặc nhiều tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length} tấm` },
          { label: 'Chọn tay nắm', done: edge !== null },
          { label: 'Nhập số', done: false },
        ]}
      />
      <Radio value={edge ?? ('' as EdgeSide)} onChange={setEdge} options={[['LEFT', 'Cạnh trái'], ['RIGHT', 'Cạnh phải'], ['BOTTOM', 'Cạnh dưới'], ['TOP', 'Cạnh trên']]} />
      <div className="form-row">
        <label>Số mm</label>
        <input
          className="field"
          value={mm}
          placeholder="+20 / -10, Enter"
          onChange={(e) => setMm(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') void apply();
            if (e.key === 'Escape') setEdge(null);
          }}
        />
      </div>
      <div className="form-row"><label>± mm bù thêm</label><Num value={bonus} onCommit={setBonus} /></div>
      <p className="muted small">Gõ số mm + Enter = số cố định (cộng dồn được). Số dương = nở ra · số âm = thu vào · giữ độ dày. Esc để chọn cạnh khác.</p>
      <div className="muted small">Co giãn đã lưu: {saved}</div>
      <button className="btn" disabled={!selection.length} onClick={() => selection.forEach((id) => void Commands.setPartMod(id, { extend: [0, 0, 0, 0] }).catch(() => undefined))}>
        Bỏ co giãn tấm đã chọn
      </button>
    </div>
  );
}

function GrooveTool() {
  const { selection, cabinet, tree } = useTargets();
  const [value, setValue] = useState(13);
  const back = selection.length === 1 && tree ? selection[0] : null;
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Chọn tấm chui vào rãnh (hậu)', done: back !== null && cabinet !== null },
          { label: 'Tấm bị xẻ rãnh: hồi, nóc, đáy của tủ', done: cabinet !== null, detail: 'Tự động' },
        ]}
      />
      <div className="form-row"><label>Giá trị rãnh</label><Num value={value} onCommit={setValue} /></div>
      <div className="form-row"><label>Tiết diện</label><select className="field" value="R"><option value="R">Rãnh</option></select></div>
      <button className="btn primary" disabled={cabinet === null} onClick={() => cabinet !== null && void Commands.setParameter(cabinet, 'back_groove', String(value)).catch(() => undefined)}>
        [TAB] Tạo rãnh
      </button>
      <button className="btn" disabled={cabinet === null} onClick={() => cabinet !== null && void Commands.setParameter(cabinet, 'back_groove', '0').catch(() => undefined)}>
        Xoá tiết diện tấm này
      </button>
      <p className="muted small">Hậu được kéo dài thêm giá trị rãnh mỗi phía; hồi/nóc/đáy có rãnh rộng = dày hậu + 0.5.</p>
    </div>
  );
}

function FlipJoinTool() {
  const { cabinet } = useTargets();
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn một tấm của tủ', done: cabinet !== null }]} />
      <div className="row-btns">
        <button className="btn" disabled={cabinet === null} onClick={() => void flip(cabinet!, 'top_style')}>Đảo nóc phủ/lọt</button>
        <button className="btn" disabled={cabinet === null} onClick={() => void flip(cabinet!, 'bottom_style')}>Đảo đáy phủ/lọt</button>
      </div>
    </div>
  );
}

async function flip(cab: number, key: 'top_style' | 'bottom_style') {
  const s = await Queries.properties(cab).catch(() => null);
  const cur = s?.groups.flatMap((g) => g.fields).find((f) => f.key === key)?.value;
  await Commands.setParameter(cab, key, cur === 'OVERLAY' ? 'INSET' : 'OVERLAY').catch(() => undefined);
}

function SlideTool() {
  const { cabinet } = useTargets();
  return (
    <div className="tool-form">
      <p className="muted small">Ray bi được gắn tự động cho mỗi ngăn kéo (chiều dài theo chiều sâu vùng). Thêm ngăn kéo ở tab Tạo tấm.</p>
      <button className="btn" disabled={cabinet === null} onClick={() => useUi.getState().set({ designerTab: 'create', createType: 'drawer' })}>Mở Tạo ngăn kéo</button>
    </div>
  );
}

function GrooveLineTool({ tool, defaults }: { tool: string; defaults: { width: number; depth: number; offset: number } }) {
  const { selection } = useTargets();
  const [g, setG] = useState(defaults);
  const [dir, setDir] = useState<'X' | 'Y'>('X');
  const [side, setSide] = useState<FaceSide>('A');
  const apply = () =>
    void addFeatures(selection, tool, (w, h) => [
      dir === 'X'
        ? { type: 'GROOVE', x: 0, y: g.offset, length: w, width: g.width, depth: g.depth, direction: 'X', side }
        : { type: 'GROOVE', x: g.offset, y: 0, length: h, width: g.width, depth: g.depth, direction: 'Y', side },
    ]);
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <Radio value={dir} onChange={setDir} options={[['X', 'Chạy theo chiều rộng'], ['Y', 'Chạy theo chiều cao']]} />
      <div className="form-row"><label>Cách mép</label><Num value={g.offset} onCommit={(v) => setG({ ...g, offset: v })} /></div>
      <div className="form-row"><label>Rộng</label><Num value={g.width} onCommit={(v) => setG({ ...g, width: v })} /></div>
      <div className="form-row"><label>Sâu</label><Num value={g.depth} onCommit={(v) => setG({ ...g, depth: v })} /></div>
      <Radio value={side} onChange={setSide} options={[['A', 'Mặt A'], ['B', 'Mặt B']]} />
      <button className="btn primary" disabled={!selection.length} onClick={apply}>Áp dụng</button>
    </div>
  );
}

function ClearToolsTool() {
  const { cabinet, tree } = useTargets();
  const apply = async () => {
    if (cabinet === null || !confirm('Gỡ mọi tool đã áp trên toàn tủ?')) return;
    const walk = (n: { id: number; children: { id: number; children: unknown[] }[] }): number[] => [n.id, ...n.children.flatMap((c) => walk(c as never))];
    const root = tree?.roots.flatMap((r) => walk(r as never)) ?? [];
    const ids = root.filter((id) => cabinetOf(tree, id) === cabinet && id !== cabinet);
    for (const id of ids) await Commands.setPartMod(id, { clear_tools: true }).catch(() => undefined);
  };
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tủ', done: cabinet !== null }]} />
      <button className="btn danger-btn" disabled={cabinet === null} onClick={() => void apply()}>Xoá tool cả tủ</button>
    </div>
  );
}

/** Panel size from the property sheet (w, h, t) and whether it belongs to a cabinet. */
async function panelSize(id: ObjectId): Promise<{ w: number; h: number; t: number; generated: boolean } | null> {
  const sheet = await Queries.properties(id).catch(() => null);
  if (!sheet || sheet.kind !== 'PANEL') return null;
  const v = (k: string) => Number(sheet.groups.flatMap((g) => g.fields).find((f) => f.key === k)?.value ?? 0);
  const node = findIn(useUi.getState().tree?.roots ?? [], id);
  return { w: v('width'), h: v('height'), t: v('thickness'), generated: !!node?.generated };
}

function findIn(nodes: { id: number; generated?: boolean; children: unknown[] }[], id: number): { generated?: boolean } | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const f = findIn(n.children as never, id);
    if (f) return f;
  }
  return null;
}

const CORNERS: [Corner, string][] = [
  ['TOP_LEFT', 'Trên trái'],
  ['TOP_RIGHT', 'Trên phải'],
  ['BOTTOM_LEFT', 'Dưới trái'],
  ['BOTTOM_RIGHT', 'Dưới phải'],
];

function CornerPicker({ value, onChange, multi }: { value: Corner[]; onChange: (v: Corner[]) => void; multi: boolean }) {
  return (
    <div className="corner-pick" role="group" aria-label="Góc">
      {CORNERS.map(([c, l]) => (
        <button
          key={c}
          type="button"
          className={`${c.toLowerCase().replace('_', '-')} ${value.includes(c) ? 'on' : ''}`}
          title={l}
          onClick={() => onChange(multi ? (value.includes(c) ? value.filter((x) => x !== c) : [...value, c]) : [c])}
        />
      ))}
      <span className="corner-panel" />
    </div>
  );
}

function ClearShape({ ids }: { ids: ObjectId[] }) {
  return (
    <button className="btn" disabled={!ids.length} onClick={() => ids.forEach((id) => void Commands.setPartMod(id, { clear_shape: true }).catch(() => undefined))}>
      Bỏ hình dạng (về chữ nhật)
    </button>
  );
}

function CornerTool() {
  const { selection } = useTargets();
  const [corners, setCorners] = useState<Corner[]>(['TOP_LEFT', 'TOP_RIGHT']);
  const [size, setSize] = useState(50);
  const [mode, setMode] = useState<'R' | 'C'>('R');
  const apply = () => void Commands.shapeTool(selection, { kind: 'CORNERS', corners, size, chamfer: mode === 'C' }).catch(() => undefined);
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Chọn 01 hoặc nhiều tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` },
          { label: 'Chọn góc', done: corners.length > 0, detail: `${corners.length} góc` },
        ]}
      />
      <CornerPicker value={corners} onChange={setCorners} multi />
      <Radio value={mode} onChange={setMode} options={[['R', 'Bo tròn (R)'], ['C', 'Vát góc (C)']]} />
      <div className="form-row"><label>{mode === 'R' ? 'Bán kính R' : 'Cạnh vát C'}</label><Num value={size} onCommit={setSize} /></div>
      <button className="btn primary" disabled={!selection.length || !corners.length} onClick={apply}>{mode === 'R' ? 'Bo góc' : 'Vát góc'}</button>
      <ClearShape ids={selection} />
      <p className="muted small">Góc tính theo mặt A của tấm (trục X = rộng, Y = cao). Bo góc có đường gia công CNC theo biên dạng mới.</p>
    </div>
  );
}

function FreeCutTool() {
  const { selection } = useTargets();
  const [mode, setMode] = useState<'CORNER' | 'LINE'>('CORNER');
  const [corner, setCorner] = useState<Corner[]>(['TOP_RIGHT']);
  const [dx, setDx] = useState(100);
  const [dy, setDy] = useState(100);
  const [pts, setPts] = useState({ x1: 0, y1: 0, x2: 100, y2: 100 });
  const [keep, setKeep] = useState<'AUTO' | 'LEFT' | 'RIGHT'>('AUTO');
  const apply = async () => {
    for (const id of selection) {
      const s = await panelSize(id);
      if (!s) continue;
      let a: [number, number];
      let b: [number, number];
      if (mode === 'LINE') {
        a = [pts.x1, pts.y1];
        b = [pts.x2, pts.y2];
      } else {
        const c = corner[0];
        const cx = c.endsWith('LEFT') ? 0 : s.w;
        const cy = c.startsWith('BOTTOM') ? 0 : s.h;
        const sx = c.endsWith('LEFT') ? 1 : -1;
        const sy = c.startsWith('BOTTOM') ? 1 : -1;
        a = [cx + sx * dx, cy];
        b = [cx, cy + sy * dy];
      }
      await Commands.shapeTool([id], { kind: 'CUT_LINE', a, b, keep: mode === 'LINE' ? keep : 'AUTO' }).catch(() => undefined);
    }
  };
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <Radio value={mode} onChange={setMode} options={[['CORNER', 'Cắt xiên một góc'], ['LINE', 'Cắt theo 2 điểm']]} />
      {mode === 'CORNER' ? (
        <>
          <CornerPicker value={corner} onChange={setCorner} multi={false} />
          <div className="form-row"><label>Theo chiều rộng</label><Num value={dx} onCommit={setDx} /></div>
          <div className="form-row"><label>Theo chiều cao</label><Num value={dy} onCommit={setDy} /></div>
        </>
      ) : (
        <>
          {(['x1', 'y1', 'x2', 'y2'] as const).map((k) => (
            <div className="form-row" key={k}>
              <label>{{ x1: 'Điểm 1 · X', y1: 'Điểm 1 · Y', x2: 'Điểm 2 · X', y2: 'Điểm 2 · Y' }[k]}</label>
              <Num value={pts[k]} onCommit={(v) => setPts({ ...pts, [k]: v })} />
            </div>
          ))}
          <Radio value={keep} onChange={setKeep} options={[['AUTO', 'Giữ phần lớn'], ['LEFT', 'Giữ bên trái đường 1→2'], ['RIGHT', 'Giữ bên phải đường 1→2']]} />
        </>
      )}
      <button className="btn primary" disabled={!selection.length} onClick={() => void apply()}>Cắt</button>
      <ClearShape ids={selection} />
    </div>
  );
}

function CutByPanelTool() {
  const { selection, tree } = useTargets();
  const [cutter, setCutter] = useState<ObjectId | null>(null);
  const [clearance, setClearance] = useState(0);
  const name = (id: ObjectId | null) => {
    const walk = (ns: { id: number; name: string; children: unknown[] }[]): string | null => {
      for (const n of ns) {
        if (n.id === id) return n.name;
        const r = walk(n.children as never);
        if (r) return r;
      }
      return null;
    };
    return id === null ? '' : walk(tree?.roots ?? []) ?? `#${id}`;
  };
  const targets = selection.filter((id) => id !== cutter);
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Chọn tấm cắt (tấm đâm xuyên)', done: cutter !== null, detail: name(cutter) },
          { label: 'Chọn các tấm bị cắt', done: targets.length > 0, detail: `Đã chọn ${targets.length}` },
        ]}
      />
      <button className="btn" disabled={selection.length !== 1} onClick={() => setCutter(selection[0])}>Lấy tấm đang chọn làm tấm cắt</button>
      <div className="form-row"><label>Khe hở mỗi phía</label><Num value={clearance} onCommit={setClearance} /></div>
      <button className="btn primary" disabled={cutter === null || !targets.length} onClick={() => void Commands.shapeTool(targets, { kind: 'CUT_BY_PANEL', cutter: cutter!, clearance }).catch(() => undefined)}>
        Cắt theo tấm
      </button>
      <ClearShape ids={targets} />
      <p className="muted small">Xuyên hết chiều dày: ở mép → đổi biên dạng, ở giữa → khoét lỗ. Không xuyên hết → khấu mặt (pocket) đúng độ sâu.</p>
    </div>
  );
}

function MergeTool() {
  const { selection } = useTargets();
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Ctrl+click chọn ≥ 2 tấm cùng mặt phẳng, cùng độ dày', done: selection.length > 1, detail: `Đã chọn ${selection.length}` },
          { label: 'Tấm chọn đầu tiên được giữ lại', done: selection.length > 1 },
        ]}
      />
      <button className="btn primary" disabled={selection.length < 2} onClick={() => void Commands.mergePanels(selection).then((r) => useUi.getState().select([r.id])).catch(() => undefined)}>
        Hợp tấm
      </button>
    </div>
  );
}

function LaminateTool() {
  const { selection } = useTargets();
  const [layers, setLayers] = useState(2);
  const apply = async () => {
    for (const id of selection) {
      const s = await panelSize(id);
      if (!s) continue;
      const t = Math.round(s.t * layers * 100) / 100;
      const run = s.generated
        ? Commands.setPartMod(id, { thickness: t, tool: '14. Ghép bề dày', add_features: [] })
        : Commands.setParameter(id, 'thickness', String(t));
      await run.catch(() => undefined);
    }
  };
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <div className="form-row"><label>Số lớp ghép</label><Num value={layers} onCommit={(v) => setLayers(Math.max(2, Math.round(v)))} /></div>
      <button className="btn primary" disabled={!selection.length} onClick={() => void apply()}>Ghép bề dày</button>
      <button className="btn" disabled={!selection.length} onClick={() => selection.forEach((id) => void Commands.setPartMod(id, { thickness: null }).catch(() => undefined))}>
        Bỏ ghép (độ dày gốc)
      </button>
    </div>
  );
}

function SplitPartTool() {
  const { selection } = useTargets();
  const [axis, setAxis] = useState<'X' | 'Y'>('Y');
  const [count, setCount] = useState(2);
  const [gap, setGap] = useState(2);
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm thuộc tủ (hậu, cánh, hồi…)', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <Radio value={axis} onChange={setAxis} options={[['Y', 'Chia theo chiều cao (ngang)'], ['X', 'Chia theo chiều rộng (dọc)']]} />
      <div className="form-row"><label>Số tấm</label><Num value={count} onCommit={(v) => setCount(Math.min(50, Math.max(2, Math.round(v))))} /></div>
      <div className="form-row"><label>Khe giữa các tấm</label><Num value={gap} onCommit={setGap} /></div>
      <button className="btn primary" disabled={!selection.length} onClick={() => selection.forEach((id) => void Commands.setPartMod(id, { split: { axis, count, gap } }).catch(() => undefined))}>
        Chia tấm
      </button>
      <button className="btn" disabled={!selection.length} onClick={() => selection.forEach((id) => void Commands.setPartMod(id, { split: null }).catch(() => undefined))}>
        Bỏ chia (chọn tấm thứ nhất)
      </button>
      <p className="muted small">Các tấm con tự cập nhật khi tủ đổi kích thước. Gia công nằm trên tấm con nào thì giữ ở tấm đó.</p>
    </div>
  );
}

function EdgeArcTool() {
  const { selection } = useTargets();
  const [edge, setEdge] = useState<EdgeSide>('TOP');
  const [depth, setDepth] = useState(50);
  const [dir, setDir] = useState<'OUT' | 'IN'>('IN');
  return (
    <div className="tool-form">
      <Steps steps={[{ label: 'Chọn tấm', done: selection.length > 0, detail: `Đã chọn ${selection.length}` }]} />
      <Radio value={edge} onChange={setEdge} options={[['TOP', 'Cạnh trên'], ['BOTTOM', 'Cạnh dưới'], ['LEFT', 'Cạnh trái'], ['RIGHT', 'Cạnh phải']]} />
      <Radio value={dir} onChange={setDir} options={[['IN', 'Cung lõm (khoét vào)'], ['OUT', 'Cung lồi (phình ra)']]} />
      <div className="form-row"><label>Độ cong (mm)</label><Num value={depth} onCommit={(v) => setDepth(Math.abs(v))} /></div>
      <button
        className="btn primary"
        disabled={!selection.length}
        onClick={() => void Commands.shapeTool(selection, { kind: 'EDGE_ARC', edge, sagitta: dir === 'OUT' ? depth : -depth }).catch(() => undefined)}
      >
        Tạo cung
      </button>
      <ClearShape ids={selection} />
      <p className="muted small">Độ cong = khoảng cách từ giữa cạnh thẳng tới đỉnh cung. Cạnh trái/phải/trên/dưới tính theo mặt A của tấm.</p>
    </div>
  );
}

/** Biên dạng tự do: click on the panel sketch to place points (snap 5 mm, Shift 1 mm). */
function PolygonTool() {
  const { selection, revision } = useUi();
  const id = selection.length === 1 ? selection[0] : null;
  const [flat, setFlat] = useState<FlatPanel | null>(null);
  const [pts, setPts] = useState<[number, number][]>([]);
  const [mode, setMode] = useState<'SUBTRACT' | 'HOLE' | 'OUTLINE'>('SUBTRACT');
  const [xy, setXy] = useState({ x: 0, y: 0 });
  const svg = useRef<SVGSVGElement>(null);
  useEffect(() => {
    if (id === null) return setFlat(null);
    Queries.manufacturing(id)
      .then(setFlat)
      .catch(() => setFlat(null));
  }, [id, revision]);
  useEffect(() => setPts([]), [id]);
  const pad = flat ? Math.max(flat.width, flat.height) * 0.08 : 10;
  const vb = flat ? `${-pad} ${-flat.height - pad} ${flat.width + 2 * pad} ${flat.height + 2 * pad}` : '0 0 100 100';
  const u = flat ? Math.max(flat.width, flat.height) / 100 : 1;
  const path = (poly: { points: { x: number; y: number }[] }) => poly.points.map((p, i) => `${i ? 'L' : 'M'}${p.x} ${-p.y}`).join(' ') + ' Z';
  const add = (e: React.MouseEvent) => {
    if (!svg.current || !flat) return;
    const pt = svg.current.createSVGPoint();
    pt.x = e.clientX;
    pt.y = e.clientY;
    const p = pt.matrixTransform(svg.current.getScreenCTM()!.inverse());
    const step = e.shiftKey ? 1 : 5;
    setPts([...pts, [Math.round(p.x / step) * step, Math.round(-p.y / step) * step]]);
  };
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Chọn 1 tấm', done: id !== null },
          { label: 'Click trên sơ đồ để đặt điểm (≥ 3)', done: pts.length >= 3, detail: `${pts.length} điểm` },
        ]}
      />
      <Radio value={mode} onChange={setMode} options={[['SUBTRACT', 'Cắt bỏ vùng'], ['HOLE', 'Khoét lỗ xuyên'], ['OUTLINE', 'Thay cả biên dạng']]} />
      {flat && (
        <svg ref={svg} className="poly-canvas" viewBox={vb} onClick={add}>
          <path d={path(flat.outer)} className="pc-outer" />
          {flat.inner.map((h, i) => (
            <path key={i} d={path(h)} className="pc-hole" />
          ))}
          {pts.length > 1 && <polyline points={[...pts, pts[0]].map((p) => `${p[0]},${-p[1]}`).join(' ')} className="pc-new" />}
          {pts.map((p, i) => (
            <circle key={i} cx={p[0]} cy={-p[1]} r={u * 1.4} className="pc-pt" />
          ))}
        </svg>
      )}
      <div className="form-row">
        <label>X / Y</label>
        <div className="xy-row">
          <Num value={xy.x} onCommit={(v) => setXy({ ...xy, x: v })} />
          <Num value={xy.y} onCommit={(v) => setXy({ ...xy, y: v })} />
          <button className="btn" onClick={() => setPts([...pts, [xy.x, xy.y]])}>+</button>
        </div>
      </div>
      <div className="pts-list">
        {pts.map((p, i) => (
          <span key={i} className="chip">
            {i + 1}: {p[0]}, {p[1]}
            <button className="icon-btn" onClick={() => setPts(pts.filter((_, j) => j !== i))}>
              <Icon name="x" size={10} />
            </button>
          </span>
        ))}
      </div>
      <div className="row-btns">
        <button className="btn" disabled={!pts.length} onClick={() => setPts(pts.slice(0, -1))}>Bỏ điểm cuối</button>
        <button className="btn" disabled={!pts.length} onClick={() => setPts([])}>Xóa hết</button>
      </div>
      <button
        className="btn primary"
        disabled={id === null || pts.length < 3}
        onClick={() => id !== null && void Commands.shapeTool([id], { kind: 'POLYGON', points: pts, mode }).then(() => setPts([])).catch(() => undefined)}
      >
        Áp biên dạng
      </button>
      {id !== null && <ClearShape ids={[id]} />}
      <p className="muted small">Tọa độ theo mặt A (gốc góc dưới trái). Bắt lưới 5 mm, giữ Shift để 1 mm. Điểm có thể nằm ngoài tấm khi cắt bỏ vùng ở mép.</p>
    </div>
  );
}

function RelationTool() {
  const { selection, tree } = useTargets();
  const [gap, setGap] = useState(3);
  const name = (id: ObjectId | undefined) => (id === undefined ? '–' : findName(tree?.roots ?? [], id) ?? `#${id}`);
  const ok = selection.length === 2;
  const run = (k: RelationKind) => ok && void Commands.setRelation(selection[0], selection[1], k, gap).catch(() => undefined);
  return (
    <div className="tool-form">
      <Steps
        steps={[
          { label: 'Tấm A (chọn trước)', done: selection.length >= 1, detail: name(selection[0]) },
          { label: 'Tấm B (Ctrl+click)', done: ok, detail: name(selection[1]) },
        ]}
      />
      <div className="rel-grid">
        <button className="btn" disabled={!ok} onClick={() => run('OVERLAY')} title="A phủ lên cạnh B, B dừng ở A">A phủ B</button>
        <button className="btn" disabled={!ok} onClick={() => run('INSET')} title="A lọt giữa, dừng ở mặt trong B">A lọt B</button>
        <button className="btn" disabled={!ok} onClick={() => run('FLUSH')} title="Cạnh trước A bằng mặt cạnh trước B">Bằng mặt trước</button>
        <button className="btn" disabled={!ok} onClick={() => run('GAP')}>Khe {gap} mm</button>
      </div>
      <div className="form-row"><label>Khe (mm)</label><Num value={gap} onCommit={setGap} /></div>
      <button className="btn" disabled={!ok} onClick={() => run('NONE')}>Bỏ quan hệ A–B</button>
      <p className="muted small">Quan hệ được lưu thành ràng buộc: đổi kích thước tủ vẫn giữ. Ví dụ Đáy (A) phủ Hồi trái (B): đáy chạy dưới hồi, hồi đứng trên đáy.</p>
    </div>
  );
}

function findName(nodes: { id: number; name: string; children: unknown[] }[], id: number): string | null {
  for (const n of nodes) {
    if (n.id === id) return n.name;
    const r = findName(n.children as never, id);
    if (r) return r;
  }
  return null;
}
