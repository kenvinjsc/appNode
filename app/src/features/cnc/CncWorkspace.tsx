// CNC workspace: toolpaths from the core, visualised with a simple playback.
// The simulator only animates the core's moves; no G-code is generated here.
import { useEffect, useMemo, useRef, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { CncMove, CncProgram, NestingPlacement, PartsReport } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { fmt } from '../../shared/i18n';
import { runJob } from '../../shared/jobs';

const TOOL_COLOR = ['#e8590c', '#1c7ed6', '#2f9e44', '#ae3ec9', '#f08c00', '#0c8599'];

interface Flat {
  m: CncMove;
  op: number;
  tool: number;
}

export function CncWorkspace() {
  const { revision } = useUi();
  const [parts, setParts] = useState<PartsReport | null>(null);
  const [material, setMaterial] = useState<string | null>(null);
  const [sheet, setSheet] = useState(0);
  const [data, setData] = useState<{ program: CncProgram; placements: NestingPlacement[]; sheet_count: number } | null>(null);
  const [step, setStep] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(4);
  const gref = useRef<HTMLPreElement>(null);

  useEffect(() => {
    Queries.parts().then((p) => {
      setParts(p);
      if (!material && p.materials.length) setMaterial([...p.materials].sort((a, b) => b.count - a.count)[0].material_id);
    }).catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revision]);

  const generate = async (mat = material, sh = sheet) => {
    if (!mat) return;
    const r = await runJob('Tạo đường chạy dao', (signal) => Queries.cnc(mat, sh, signal));
    if (r) {
      setData(r);
      setStep(0);
      setPlaying(false);
    }
  };

  useEffect(() => {
    if (material) void generate(material, sheet);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [material, sheet]);

  const flat: Flat[] = useMemo(() => {
    if (!data) return [];
    const out: Flat[] = [];
    data.program.operations.forEach((o, oi) => o.moves.forEach((m) => out.push({ m, op: oi, tool: o.tool_id })));
    return out;
  }, [data]);

  useEffect(() => {
    if (!playing) return;
    const t = setInterval(() => setStep((s) => (s + speed >= flat.length ? (setPlaying(false), flat.length - 1) : s + speed)), 30);
    return () => clearInterval(t);
  }, [playing, speed, flat.length]);

  const prog = data?.program;
  const cur = flat[step];
  const lines = useMemo(() => prog?.gcode.split('\n') ?? [], [prog]);
  // Approximate current G-code line: count motion lines.
  const gLine = useMemo(() => {
    if (!prog) return 0;
    let n = -1;
    for (let i = 0; i < lines.length; i++) {
      if (/^G[01] /.test(lines[i])) n++;
      if (n >= step) return i;
    }
    return lines.length - 1;
  }, [lines, step, prog]);
  useEffect(() => {
    const el = gref.current?.querySelector(`[data-l="${gLine}"]`) as HTMLElement | null;
    el?.scrollIntoView({ block: 'center' });
  }, [gLine]);

  const download = () => {
    if (!prog) return;
    const blob = new Blob([prog.gcode], { type: 'text/plain' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${material}-sheet${sheet + 1}.nc`;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  const toolIdx = (id: number) => (prog?.tools.findIndex((t) => t.id === id) ?? 0);
  const pathFor = (upto: number) => {
    const segs: Record<number, string[]> = {};
    for (let i = 1; i <= upto && i < flat.length; i++) {
      const a = flat[i - 1].m;
      const b = flat[i].m;
      if (b.rapid || (a.x === b.x && a.y === b.y)) continue;
      (segs[flat[i].tool] ??= []).push(`M${a.x} ${prog!.sheet_height - a.y}L${b.x} ${prog!.sheet_height - b.y}`);
    }
    return segs;
  };
  const done = pathFor(step);
  const all = pathFor(flat.length - 1);

  return (
    <div className="ws ws-cnc">
      <div className="ws-toolbar">
        <label>
          Vật liệu
          <select value={material ?? ''} onChange={(e) => { setMaterial(e.target.value); setSheet(0); }}>
            {parts?.materials.map((m) => (
              <option key={m.material_id} value={m.material_id}>
                {m.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          Tấm
          <select value={sheet} onChange={(e) => setSheet(Number(e.target.value))}>
            {Array.from({ length: data?.sheet_count ?? 1 }, (_, i) => (
              <option key={i} value={i}>
                {i + 1}
              </option>
            ))}
          </select>
        </label>
        <div className="player">
          <button title="Về đầu" onClick={() => { setStep(0); setPlaying(false); }}><Icon name="stop" size={16} /></button>
          <button title={playing ? 'Tạm dừng' : 'Chạy'} className="primary" onClick={() => setPlaying(!playing)}><Icon name={playing ? 'pause' : 'play'} size={16} /></button>
          <button title="Bước" onClick={() => setStep((s) => Math.min(s + 1, flat.length - 1))}><Icon name="step" size={16} /></button>
          <input type="range" min={0} max={Math.max(flat.length - 1, 0)} value={step} onChange={(e) => setStep(Number(e.target.value))} />
          <select value={speed} onChange={(e) => setSpeed(Number(e.target.value))} title="Tốc độ mô phỏng">
            {[1, 2, 4, 10, 25, 60].map((s) => (
              <option key={s} value={s}>×{s}</option>
            ))}
          </select>
        </div>
        <div className="spacer" />
        <button className="btn" onClick={() => void generate()}><Icon name="rotate" size={16} /> Tạo lại</button>
        <button className="btn primary" disabled={!prog} onClick={download}><Icon name="download" size={16} /> Xuất G-code</button>
      </div>
      <div className="ws-body">
        <div className="panel cnc-sheet">
          <div className="panel-header">
            <Icon name="cnc" size={16} />
            <span>{prog ? `Tấm ${sheet + 1} · ${prog.sheet_width} × ${prog.sheet_height} × ${prog.thickness}` : 'Tấm'}</span>
            {cur && (
              <span className="muted">
                &nbsp;· X {fmt(cur.m.x)} Y {fmt(cur.m.y)} Z {fmt(cur.m.z)} · {prog?.tools.find((t) => t.id === cur.tool)?.name}
              </span>
            )}
          </div>
          {prog && (
            <div className="sheet-view">
              <svg viewBox={`-40 -40 ${prog.sheet_width + 80} ${prog.sheet_height + 80}`} preserveAspectRatio="xMidYMid meet">
                <rect x={0} y={0} width={prog.sheet_width} height={prog.sheet_height} className="sheet-rect" />
                {data!.placements.map((p, i) => (
                  <rect key={i} x={p.x_mm} y={prog.sheet_height - p.y_mm - p.height_mm} width={p.width_mm} height={p.height_mm} className="cnc-part" />
                ))}
                {Object.entries(all).map(([tool, d]) => (
                  <path key={`a${tool}`} d={d.join('')} stroke={TOOL_COLOR[toolIdx(Number(tool)) % TOOL_COLOR.length]} strokeOpacity={0.18} strokeWidth={3} fill="none" />
                ))}
                {Object.entries(done).map(([tool, d]) => (
                  <path key={`d${tool}`} d={d.join('')} stroke={TOOL_COLOR[toolIdx(Number(tool)) % TOOL_COLOR.length]} strokeWidth={4} fill="none" />
                ))}
                {prog.operations
                  .filter((o) => o.kind === 'DRILL' && o.moves.length === 3)
                  .map((o, i) => (
                    <circle key={i} cx={o.moves[0].x} cy={prog.sheet_height - o.moves[0].y} r={prog.tools.find((t) => t.id === o.tool_id)!.diameter / 2 + 1} fill="none" stroke={TOOL_COLOR[toolIdx(o.tool_id) % TOOL_COLOR.length]} strokeWidth={2} />
                  ))}
                {cur && (
                  <g transform={`translate(${cur.m.x} ${prog.sheet_height - cur.m.y})`}>
                    <circle r={18} className={`spindle ${cur.m.z < 0 ? 'cutting' : ''}`} />
                    <circle r={4} fill="#212529" />
                  </g>
                )}
              </svg>
            </div>
          )}
        </div>
        <div className="panel cnc-side">
          <div className="panel-header"><Icon name="settings" size={16} /><span>Máy & dao</span></div>
          <div className="mfg-info">
            <div className="kv"><span>Máy</span><b>CNC router 3 trục (mô phỏng)</b></div>
            <div className="kv"><span>Đổi dao</span><b>{prog?.stats.tool_changes ?? 0}</b></div>
            <div className="kv"><span>Chiều dài cắt</span><b>{fmt((prog?.stats.cut_length_mm ?? 0) / 1000, 1)} m</b></div>
            <div className="kv"><span>Chạy không</span><b>{fmt((prog?.stats.rapid_length_mm ?? 0) / 1000, 1)} m</b></div>
            <div className="kv"><span>Thời gian ước tính</span><b>{fmt((prog?.stats.estimated_time_s ?? 0) / 60, 1)} phút</b></div>
          </div>
          <div className="tool-list">
            {prog?.tools
              .filter((t) => prog.operations.some((o) => o.tool_id === t.id))
              .map((t) => (
                <div key={t.id} className={`tool ${cur?.tool === t.id ? 'on' : ''}`}>
                  <i style={{ background: TOOL_COLOR[toolIdx(t.id) % TOOL_COLOR.length] }} />
                  <b>T{t.id}</b> {t.name}
                  <span className="muted">{t.rpm} v/ph</span>
                </div>
              ))}
          </div>
          {prog && prog.warnings.length > 0 && (
            <details className="warnings">
              <summary><Icon name="warning" size={14} /> {prog.warnings.length} cảnh báo</summary>
              {prog.warnings.slice(0, 40).map((w, i) => <div key={i}>{w}</div>)}
            </details>
          )}
          <div className="panel-header sub"><Icon name="report" size={16} /><span>G-code</span></div>
          <pre className="gcode" ref={gref}>
            {lines.slice(Math.max(0, gLine - 200), gLine + 200).map((l, i) => {
              const n = Math.max(0, gLine - 200) + i;
              return (
                <div key={n} data-l={n} className={n === gLine ? 'cur' : ''}>
                  <span>{n + 1}</span>
                  {l}
                </div>
              );
            })}
          </pre>
        </div>
      </div>
    </div>
  );
}
