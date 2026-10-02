// Manufacturing workspace: panel list · flattened 2D view · 3D preview · features.
import { useEffect, useMemo, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { FlatPanel, PartsReport } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { EDGE_LABEL, GRAIN_LABEL, fmt, roleLabel } from '../../shared/i18n';
import { FlatPanelView, describe } from './FlatPanelView';
import { PanelPreview3D } from './PanelPreview3D';

const ORIGIN: Record<string, string> = { USER: 'Người dùng', RULE: 'Quy tắc', JOINT: 'Liên kết' };

export function ManufacturingWorkspace() {
  const { mfgPanel, set, revision, materials } = useUi();
  const [parts, setParts] = useState<PartsReport | null>(null);
  const [flat, setFlat] = useState<FlatPanel | null>(null);
  const [side, setSide] = useState<'A' | 'B'>('A');
  const [hl, setHl] = useState<number | null>(null);
  const [q, setQ] = useState('');

  useEffect(() => {
    Queries.parts().then(setParts).catch(() => undefined);
  }, [revision]);
  useEffect(() => {
    if (!mfgPanel && parts?.parts.length) set({ mfgPanel: parts.parts[0].id });
  }, [parts, mfgPanel, set]);
  useEffect(() => {
    if (mfgPanel === null) return setFlat(null);
    Queries.manufacturing(mfgPanel).then(setFlat).catch(() => setFlat(null));
  }, [mfgPanel, revision]);

  const groups = useMemo(() => {
    const m = new Map<string, PartsReport['parts']>();
    for (const p of parts?.parts ?? []) {
      if (q && !`${p.name} ${p.cabinet}`.toLowerCase().includes(q.toLowerCase())) continue;
      const k = p.cabinet ?? 'Tấm rời';
      m.set(k, [...(m.get(k) ?? []), p]);
    }
    return Array.from(m);
  }, [parts, q]);

  const mat = materials.find((m) => m.id === flat?.material_id);

  return (
    <div className="ws ws-mfg">
      <div className="panel mfg-list">
        <div className="panel-header">
          <Icon name="panel" size={16} />
          <span>Danh sách tấm</span>
          <span className="badge">{parts?.parts.length ?? 0}</span>
        </div>
        <div className="tree-search">
          <Icon name="search" size={14} />
          <input placeholder="Tìm tấm…" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
        <div className="list-body">
          {groups.map(([cab, rows]) => (
            <div key={cab}>
              <div className="list-group">{cab}</div>
              {rows.map((p) => (
                <div key={p.id} className={`list-row ${mfgPanel === p.id ? 'selected' : ''}`} onClick={() => set({ mfgPanel: p.id })}>
                  <span className="list-name">{p.name}</span>
                  <span className="list-meta">
                    {fmt(p.length, 0)}×{fmt(p.width, 0)}×{fmt(p.thickness, 0)}
                  </span>
                  {p.drills > 0 && <span className="pill">{p.drills}</span>}
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>

      <div className="panel mfg-main">
        <div className="panel-header">
          <Icon name="drawing" size={16} />
          <span>{flat ? `${flat.name} · ${fmt(flat.width, 1)} × ${fmt(flat.height, 1)} × ${fmt(flat.thickness, 0)}` : 'Bản vẽ trải phẳng'}</span>
          <div className="spacer" />
          <div className="seg">
            <button className={side === 'A' ? 'active' : ''} onClick={() => setSide('A')}>
              Mặt A
            </button>
            <button className={side === 'B' ? 'active' : ''} onClick={() => setSide('B')}>
              Mặt B
            </button>
          </div>
        </div>
        {flat ? <FlatPanelView flat={flat} side={side} highlight={hl} onHover={setHl} /> : <div className="empty">Chọn một tấm.</div>}
        <div className="legend">
          <span><i style={{ background: '#1c7ed6' }} /> Chốt gỗ</span>
          <span><i style={{ background: '#2f9e44' }} /> Chốt đợt</span>
          <span><i style={{ background: '#e8590c' }} /> Khoét bản lề</span>
          <span><i style={{ background: '#5f3dc4' }} /> Hốc</span>
          <span><i style={{ background: '#0c8599' }} /> Rãnh</span>
          <span><i className="band" /> Dán cạnh</span>
          <span><i className="dashed" /> Mặt đối diện / cạnh</span>
        </div>
      </div>

      <div className="panel mfg-side">
        <div className="panel-header">
          <Icon name="box3d" size={16} />
          <span>Mô hình 3D</span>
        </div>
        {flat && <PanelPreview3D id={flat.id} />}
        {flat && (
          <div className="mfg-info">
            <div className="kv"><span>Vai trò</span><b>{roleLabel(flat.role)}</b></div>
            <div className="kv"><span>Vật liệu</span><b>{mat?.name ?? flat.material_id}</b></div>
            <div className="kv"><span>Vân gỗ</span><b>{GRAIN_LABEL[flat.grain]}</b></div>
            <div className="kv"><span>Khoan</span><b>{flat.summary.drills + flat.summary.edge_drills}</b></div>
            <div className="kv"><span>Hốc</span><b>{flat.summary.pockets}</b></div>
            <div className="kv"><span>Rãnh</span><b>{flat.summary.grooves}</b></div>
            <div className="kv"><span>Dán cạnh</span><b>{flat.edge_bands.length ? flat.edge_bands.map((b) => EDGE_LABEL[b.edge]).join(' / ') : '—'}</b></div>
          </div>
        )}
        <div className="panel-header sub">
          <Icon name="drill" size={16} />
          <span>Gia công</span>
        </div>
        <div className="feat-table">
          {flat?.features.map((f, i) => (
            <div key={i} className={`feat-row ${hl === i ? 'hl' : ''}`} onMouseEnter={() => setHl(i)} onMouseLeave={() => setHl(null)}>
              <span className="idx">{i + 1}</span>
              <span className="desc">{describe(f.feature)}</span>
              <span className="origin">{ORIGIN[f.origin.kind]}</span>
            </div>
          ))}
          {flat && flat.features.length === 0 && <div className="empty small">Không có gia công.</div>}
        </div>
      </div>
    </div>
  );
}
