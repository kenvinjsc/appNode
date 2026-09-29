// Material browser. Texture/colour is presentation; the logical material is the code.
import { useMemo, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { Material } from '../../core-api/types';
import { Icon } from '../../shared/icons';

const KIND: Record<string, string> = { MDF: 'MDF', PLYWOOD: 'Plywood', PARTICLEBOARD: 'Ván dăm', HDF: 'HDF', SOLID_WOOD: 'Gỗ tự nhiên' };

export function MaterialBrowser() {
  const { materials, selection, set } = useUi();
  const [q, setQ] = useState('');
  const [kind, setKind] = useState<string | null>(null);
  const [thick, setThick] = useState<number | null>(null);
  const kinds = Array.from(new Set(materials.map((m) => m.kind)));
  const thicks = Array.from(new Set(materials.map((m) => m.thickness_mm))).sort((a, b) => a - b);
  const list = useMemo(
    () =>
      materials.filter(
        (m) =>
          (!q || `${m.name} ${m.id}`.toLowerCase().includes(q.toLowerCase())) && (!kind || m.kind === kind) && (thick === null || m.thickness_mm === thick),
      ),
    [materials, q, kind, thick],
  );
  const apply = async (m: Material, slot?: 'carcass' | 'front' | 'back') => {
    for (const id of selection) {
      try {
        await Commands.setMaterial(id, m.id, slot);
      } catch {
        /* toast shown */
      }
    }
  };
  return (
    <div className="drawer-panel">
      <div className="panel-header">
        <Icon name="material" size={16} />
        <span>Thư viện vật liệu</span>
        <div className="spacer" />
        <button className="icon-btn" onClick={() => set({ drawer: null })}><Icon name="x" size={16} /></button>
      </div>
      <div className="tree-search">
        <Icon name="search" size={14} />
        <input placeholder="Tìm vật liệu…" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <div className="chips">
        <button className={!kind ? 'on' : ''} onClick={() => setKind(null)}>Tất cả</button>
        {kinds.map((k) => (
          <button key={k} className={kind === k ? 'on' : ''} onClick={() => setKind(kind === k ? null : k)}>{KIND[k]}</button>
        ))}
      </div>
      <div className="chips">
        {thicks.map((t) => (
          <button key={t} className={thick === t ? 'on' : ''} onClick={() => setThick(thick === t ? null : t)}>{t} mm</button>
        ))}
      </div>
      <div className="mat-grid">
        {list.map((m) => (
          <div key={m.id} className="mat-card">
            <div className={`mat-thumb ${m.has_grain ? 'grain' : ''}`} style={{ background: m.color }} />
            <div className="mat-name">{m.name}</div>
            <div className="mat-meta">
              {m.thickness_mm} mm · {m.has_grain ? 'Có vân' : 'Không vân'}
            </div>
            <div className="mat-code">{m.id}</div>
            <div className="mat-actions">
              <button disabled={!selection.length} onClick={() => void apply(m)} title="Áp dụng cho vùng chọn (thùng tủ / tấm)">Áp dụng</button>
              <button disabled={!selection.length} onClick={() => void apply(m, 'front')} title="Áp dụng cho cánh / mặt ngăn kéo của tủ">Cánh</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
