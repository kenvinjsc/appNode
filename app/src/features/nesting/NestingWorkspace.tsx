// Nesting workspace: the core computes placements; the UI sends the job and draws.
import { useEffect, useMemo, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { NestingJobResult, NestingSettings, PartsReport } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { fmt } from '../../shared/i18n';
import { runJob } from '../../shared/jobs';

const PALETTE = ['#ffd8a8', '#d0ebff', '#d3f9d8', '#e5dbff', '#fff3bf', '#ffe3e3', '#c5f6fa', '#f3d9fa'];

export function NestingWorkspace() {
  const { materials, revision, set } = useUi();
  const [parts, setParts] = useState<PartsReport | null>(null);
  const [material, setMaterial] = useState<string | null>(null);
  const [settings, setSettings] = useState<NestingSettings>({ spacing_mm: 12, margin_mm: 10, allow_rotation: true });
  const [job, setJob] = useState<NestingJobResult | null>(null);
  const [sheet, setSheet] = useState(0);
  const [stale, setStale] = useState(false);

  useEffect(() => {
    Queries.parts().then((p) => {
      setParts(p);
      if (!material && p.materials.length) setMaterial([...p.materials].sort((a, b) => b.count - a.count)[0].material_id);
    }).catch(() => undefined);
    if (job) setStale(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [revision]);

  const run = async () => {
    if (!material) return;
    const r = await runJob('Xếp tấm', (signal) => Queries.nesting(material, settings, signal));
    if (r) {
      setJob(r.jobs[0] ?? null);
      setSheet(0);
      setStale(false);
    }
  };

  useEffect(() => {
    setJob(null);
  }, [material]);

  const partsOfMat = useMemo(() => {
    const m = new Map<string, number>();
    for (const p of parts?.parts ?? []) {
      if (p.material_id !== material) continue;
      const k = `${p.name}|${fmt(p.length, 0)}×${fmt(p.width, 0)}`;
      m.set(k, (m.get(k) ?? 0) + 1);
    }
    return Array.from(m);
  }, [parts, material]);

  const res = job?.result;
  const sh = res?.sheets[sheet];
  const places = res?.placements.filter((p) => p.sheet_id === sheet) ?? [];
  const colorOf = (id: number) => PALETTE[id % PALETTE.length];

  return (
    <div className="ws ws-nest">
      <div className="ws-toolbar">
        <label>
          Vật liệu
          <select value={material ?? ''} onChange={(e) => setMaterial(e.target.value)}>
            {parts?.materials.map((m) => (
              <option key={m.material_id} value={m.material_id}>
                {m.name} ({m.count} tấm)
              </option>
            ))}
          </select>
        </label>
        <label>
          Khoảng cách
          <input type="number" value={settings.spacing_mm} onChange={(e) => setSettings({ ...settings, spacing_mm: Number(e.target.value) })} />
          <em>mm</em>
        </label>
        <label>
          Lề tấm
          <input type="number" value={settings.margin_mm} onChange={(e) => setSettings({ ...settings, margin_mm: Number(e.target.value) })} />
          <em>mm</em>
        </label>
        <label className="check">
          <input type="checkbox" checked={settings.allow_rotation} onChange={(e) => setSettings({ ...settings, allow_rotation: e.target.checked })} />
          Cho phép xoay (theo vân)
        </label>
        <div className="spacer" />
        {stale && <span className="warn"><Icon name="warning" size={14} /> Mô hình đã thay đổi</span>}
        <button className="btn primary" onClick={() => void run()} disabled={!material}>
          <Icon name="nesting" size={16} /> Chạy xếp tấm
        </button>
        <button className="btn" disabled={!job} onClick={() => set({ workspace: 'cnc' })}>
          <Icon name="cnc" size={16} /> Sang CNC
        </button>
      </div>
      <div className="ws-body">
        <div className="panel nest-parts">
          <div className="panel-header">
            <Icon name="panel" size={16} />
            <span>Chi tiết</span>
          </div>
          <div className="list-body">
            {partsOfMat.map(([k, n]) => {
              const [name, dims] = k.split('|');
              return (
                <div key={k} className="list-row">
                  <span className="list-name">{name}</span>
                  <span className="list-meta">{dims}</span>
                  <span className="pill">×{n}</span>
                </div>
              );
            })}
          </div>
        </div>
        <div className="panel nest-sheet">
          <div className="panel-header">
            <Icon name="grid" size={16} />
            <span>{job ? `Tấm ${sheet + 1} / ${res!.sheets.length} · ${job.sheet.width_mm} × ${job.sheet.height_mm} × ${job.sheet.thickness_mm}` : 'Tấm ván'}</span>
            <div className="spacer" />
            {res && (
              <div className="seg">
                {res.sheets.map((s) => (
                  <button key={s.id} className={s.id === sheet ? 'active' : ''} onClick={() => setSheet(s.id)}>
                    {s.id + 1}
                  </button>
                ))}
              </div>
            )}
          </div>
          {!job && <div className="empty">Chọn vật liệu và bấm “Chạy xếp tấm”. Thuật toán chạy trong lõi CAD.</div>}
          {job && sh && (
            <div className="sheet-view">
              <svg viewBox={`-40 -40 ${sh.width_mm + 80} ${sh.height_mm + 80}`} preserveAspectRatio="xMidYMid meet">
                <rect x={0} y={0} width={sh.width_mm} height={sh.height_mm} className="sheet-rect" />
                {job.sheet.has_grain &&
                  Array.from({ length: 12 }, (_, i) => <line key={i} x1={0} x2={sh.width_mm} y1={(sh.height_mm * (i + 1)) / 13} y2={(sh.height_mm * (i + 1)) / 13} className="sheet-grain" />)}
                {places.map((p, i) => {
                  const y = sh.height_mm - p.y_mm - p.height_mm;
                  const fs = Math.max(18, Math.min(46, Math.min(p.width_mm, p.height_mm) / 5));
                  return (
                    <g key={i}>
                      <rect x={p.x_mm} y={y} width={p.width_mm} height={p.height_mm} fill={colorOf(p.part_id)} stroke="#495057" strokeWidth={2} />
                      <text x={p.x_mm + p.width_mm / 2} y={y + p.height_mm / 2} textAnchor="middle" fontSize={fs} className="part-label">
                        {job.names[String(p.part_id)]}
                      </text>
                      <text x={p.x_mm + p.width_mm / 2} y={y + p.height_mm / 2 + fs * 1.15} textAnchor="middle" fontSize={fs * 0.8} className="part-dims">
                        {fmt(p.rotation_deg === 90 ? p.height_mm : p.width_mm, 0)} × {fmt(p.rotation_deg === 90 ? p.width_mm : p.height_mm, 0)}
                        {p.rotation_deg ? ' ↻' : ''}
                      </text>
                    </g>
                  );
                })}
              </svg>
            </div>
          )}
        </div>
      </div>
      <div className="ws-stats">
        {res ? (
          <>
            <span>Số tấm: <b>{res.sheets.length}</b></span>
            <span>Chi tiết: <b>{res.placements.length}</b></span>
            <span>Hao hụt: <b>{fmt(res.waste_ratio * 100, 1)}%</b></span>
            {sh && <span>Tấm {sheet + 1} sử dụng: <b>{fmt(sh.utilization * 100, 1)}%</b></span>}
            {res.unplaced.length > 0 && <span className="warn">Không xếp được: {res.unplaced.length}</span>}
            <span className="muted">Vật liệu: {materials.find((m) => m.id === material)?.name}</span>
          </>
        ) : (
          <span className="muted">Chưa chạy xếp tấm</span>
        )}
      </div>
    </div>
  );
}
