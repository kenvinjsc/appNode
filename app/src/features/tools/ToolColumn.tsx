// "Tool" column: switches + the 19 cabinet tools. Tools only collect parameters;
// the resulting machining/modification is stored by the core as part mods.
import { useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Actions } from '../../app/actions';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { EdgeSide, FaceSide, MachiningFeature } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { Num, Radio, Steps } from '../../shared/ui';
import { cabinetOf } from '../cabinet/useZones';

const TOOLS: [string, string, boolean][] = [
  ['01', 'Xoá tấm', true],
  ['02', 'Ẩn tấm', true],
  ['03', 'Khấu góc tủ', true],
  ['04', 'Cắt tự do', false],
  ['05', 'Cắt có liên kết', false],
  ['06', 'Hợp tấm', false],
  ['07', 'Ghép bề mặt', false],
  ['08', 'Khấu bề mặt', true],
  ['09', 'Cắt theo tấm', false],
  ['10', 'Bo/Vác góc', false],
  ['11', 'Co giãn tấm', true],
  ['12', 'Tạo Rãnh', true],
  ['13', 'Đảo phủ/lọt', true],
  ['14', 'Ghép bề dày', false],
  ['15', 'Mộng đan tay', false],
  ['16', 'Bào rãnh LED', true],
  ['17', 'Tạo ray kéo', true],
  ['18', 'Tạo Vbit', true],
  ['19', 'Xoá tool cả tủ', true],
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
    case '08':
      return <PocketTool />;
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
