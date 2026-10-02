// Cut list (bóc chi tiết) from the core's part data, exportable as CSV.
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Queries } from '../../core-api/queries';
import type { PartsReport as Report } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { EDGE_LABEL, fmt, roleLabel } from '../../shared/i18n';

export function PartsReport() {
  const { revision, set, materials } = useUi();
  const [r, setR] = useState<Report | null>(null);
  useEffect(() => {
    Queries.parts().then(setR).catch(() => undefined);
  }, [revision]);
  const matName = (id: string) => materials.find((m) => m.id === id)?.name ?? id;
  const csv = () => {
    if (!r) return;
    const head = ['STT', 'Tủ', 'Tên', 'Vai trò', 'Vật liệu', 'Dài', 'Rộng', 'Dày', 'Dán cạnh', 'Khoan', 'Rãnh', 'Hốc'];
    const rows = r.parts.map((p, i) => [i + 1, p.cabinet ?? '', p.name, roleLabel(p.role), matName(p.material_id), fmt(p.length, 1), fmt(p.width, 1), p.thickness, p.edge_bands.map((e) => EDGE_LABEL[e]).join('/'), p.drills, p.grooves, p.pockets]);
    const text = [head, ...rows].map((row) => row.map((c) => `"${String(c).replace(/"/g, '""')}"`).join(',')).join('\n');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['﻿' + text], { type: 'text/csv' }));
    a.download = 'boc-chi-tiet.csv';
    a.click();
  };
  return (
    <div className="drawer-panel wide">
      <div className="panel-header">
        <Icon name="report" size={16} />
        <span>Báo cáo · Bóc chi tiết</span>
        <div className="spacer" />
        <button className="btn" onClick={csv}><Icon name="download" size={14} /> CSV</button>
        <button className="icon-btn" onClick={() => set({ drawer: null })}><Icon name="x" size={16} /></button>
      </div>
      <div className="report-summary">
        {r?.materials.map((m) => (
          <div key={m.material_id} className="sum-card">
            <b>{m.name}</b>
            <span>{m.count} tấm · {fmt(m.area_m2, 2)} m²</span>
          </div>
        ))}
      </div>
      <div className="table-wrap">
        <table className="report">
          <thead>
            <tr>
              <th>#</th><th>Tủ</th><th>Tên</th><th>Vật liệu</th><th className="n">Dài</th><th className="n">Rộng</th><th className="n">Dày</th><th>Dán cạnh</th><th className="n">Khoan</th>
            </tr>
          </thead>
          <tbody>
            {r?.parts.map((p, i) => (
              <tr key={p.id} onClick={() => useUi.getState().select([p.id])}>
                <td>{i + 1}</td>
                <td>{p.cabinet}</td>
                <td>{p.name}</td>
                <td>{matName(p.material_id)}</td>
                <td className="n">{fmt(p.length, 1)}</td>
                <td className="n">{fmt(p.width, 1)}</td>
                <td className="n">{p.thickness}</td>
                <td>{p.edge_bands.map((e) => EDGE_LABEL[e]).join(' / ')}</td>
                <td className="n">{p.drills}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
