// Báo cáo: Costing Report · Danh sách cắt · Cabinet List. Quantities come from
// the core; unit prices are edited here and stored in the project (undoable).
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { CostLine, Costing } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { EDGE_LABEL, fmt } from '../../shared/i18n';

const vnd = (n: number) => n.toLocaleString('vi-VN', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + ' đ';

function Price({ line }: { line: CostLine }) {
  const [v, setV] = useState(String(line.price));
  useEffect(() => setV(String(line.price)), [line.price]);
  const commit = () => {
    const n = Number(v.replace(/\./g, '').replace(',', '.'));
    if (Number.isFinite(n) && n !== line.price) void Commands.setPrice(line.key, n).catch(() => undefined);
  };
  return (
    <input
      className="price"
      value={v}
      onChange={(e) => setV(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
      }}
    />
  );
}

function Block({ title, lines, qtyLabel, extra }: { title: string; lines: CostLine[]; qtyLabel: string; extra?: 'edge' }) {
  const [open, setOpen] = useState(true);
  const [sortAsc, setSortAsc] = useState(true);
  const total = lines.reduce((s, l) => s + l.amount, 0);
  const sorted = [...lines].sort((a, b) => (sortAsc ? 1 : -1) * a.name.localeCompare(b.name, 'vi'));
  return (
    <section className="cost-block">
      <button className="cost-head" onClick={() => setOpen(!open)}>
        <Icon name={open ? 'chevronDown' : 'chevronRight'} size={12} />
        <b>{title}</b>
        <span className="muted">
          {lines.length} loại · {vnd(total)}
        </span>
      </button>
      {open && (
        <table className="report cost">
          <thead>
            <tr>
              <th>#</th>
              <th className="sortable" onClick={() => setSortAsc(!sortAsc)}>
                {extra === 'edge' ? 'Loại chỉ' : title.startsWith('Panel') ? 'Vật liệu, độ dày' : 'Phụ kiện'} {sortAsc ? '▲' : '▼'}
              </th>
              {extra === 'edge' && <th className="n">Hệ số</th>}
              <th className="n">{qtyLabel}</th>
              <th className="n">Đơn giá</th>
              <th className="n">Thành tiền</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((l, i) => (
              <tr key={l.key}>
                <td>{i + 1}</td>
                <td>{l.name}</td>
                {extra === 'edge' && <td className="n">{l.factor ?? 1}</td>}
                <td className="n">
                  {fmt(l.qty, 1)} {l.unit}
                </td>
                <td className="n">
                  <Price line={l} />
                </td>
                <td className="n">{vnd(l.amount)}</td>
              </tr>
            ))}
            <tr className="total-row">
              <td />
              <td>Total</td>
              {extra === 'edge' && <td />}
              <td className="n">{title.startsWith('Panel') ? `${fmt(lines.reduce((s, l) => s + l.qty, 0), 1)} m²` : extra === 'edge' ? `${fmt(lines.reduce((s, l) => s + l.qty, 0), 1)} m` : ''}</td>
              <td />
              <td className="n">{vnd(total)}</td>
            </tr>
          </tbody>
        </table>
      )}
    </section>
  );
}

export function ReportWindow() {
  const { revision, set, select } = useUi();
  const [tab, setTab] = useState<'cost' | 'cut' | 'cabs'>('cost');
  const [c, setC] = useState<Costing | null>(null);
  useEffect(() => {
    Queries.costing().then(setC).catch(() => undefined);
  }, [revision]);
  const csv = () => {
    if (!c) return;
    const head = ['STT', 'Phòng', 'Tủ', 'Tên tấm', 'Vật liệu', 'Dài', 'Rộng', 'Dày', 'Dài cắt', 'Rộng cắt', 'SL', 'Dán cạnh', 'Gia công', 'Ghi chú'];
    const rows = c.cut_list.map((r, i) => [i + 1, r.room, r.cabinet, r.full_name, r.material, fmt(r.length, 1), fmt(r.width, 1), r.thickness, fmt(r.cut_length, 1), fmt(r.cut_width, 1), r.qty, r.edges.map(([e]) => EDGE_LABEL[e]).join('/'), r.machining.join('; '), r.note]);
    const text = [head, ...rows].map((row) => row.map((x) => `"${String(x).replace(/"/g, '""')}"`).join(',')).join('\n');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['﻿' + text], { type: 'text/csv' }));
    a.download = 'danh-sach-cat.csv';
    a.click();
  };
  return (
    <div className="drawer-panel wide report-window">
      <div className="panel-header">
        <div className="seg">
          <button className={tab === 'cost' ? 'active' : ''} onClick={() => setTab('cost')}>Costing Report</button>
          <button className={tab === 'cut' ? 'active' : ''} onClick={() => setTab('cut')}>Danh sách cắt</button>
          <button className={tab === 'cabs' ? 'active' : ''} onClick={() => setTab('cabs')}>Cabinet List</button>
        </div>
        <div className="spacer" />
        {tab === 'cut' && (
          <button className="btn" onClick={csv}>
            <Icon name="download" size={14} /> Excel / CSV
          </button>
        )}
        <button className="icon-btn" onClick={() => set({ drawer: null })}>
          <Icon name="x" size={16} />
        </button>
      </div>
      <div className="report-body">
        {!c && <div className="empty">Đang tính…</div>}
        {c && tab === 'cost' && (
          <>
            <Block title="Panel — Tấm" lines={c.panels} qtyLabel="Diện tích" />
            <Block title="Edge Banding — Dán cạnh" lines={c.edges} qtyLabel="Mét dài" extra="edge" />
            <Block title="Fittings — Phụ kiện liên kết" lines={c.fittings} qtyLabel="Khối lượng" />
            <div className="grand-total">
              <span>TỔNG CỘNG</span>
              <b>{vnd(c.totals.total)}</b>
            </div>
          </>
        )}
        {c && tab === 'cut' && (
          <div className="table-wrap">
            <table className="report">
              <thead>
                <tr>
                  <th>#</th><th>Tủ</th><th>Tên tấm</th><th>Vật liệu</th><th className="n">Dài</th><th className="n">Rộng</th><th className="n">Dày</th><th className="n">Cắt (D×R)</th><th className="n">SL</th><th>Dán cạnh</th><th>Rãnh / gia công</th><th>Ghi chú</th>
                </tr>
              </thead>
              <tbody>
                {c.cut_list.map((r, i) => (
                  <tr key={r.id} onClick={() => select([r.id])}>
                    <td>{i + 1}</td>
                    <td>{r.room ? `${r.room} - ${r.cabinet}` : r.cabinet}</td>
                    <td>{r.name}</td>
                    <td>{r.material}</td>
                    <td className="n">{fmt(r.length, 1)}</td>
                    <td className="n">{fmt(r.width, 1)}</td>
                    <td className="n">{r.thickness}</td>
                    <td className="n">{fmt(r.cut_length, 1)} × {fmt(r.cut_width, 1)}</td>
                    <td className="n">{r.qty}</td>
                    <td>{r.edges.map(([e]) => EDGE_LABEL[e]).join(' / ') || '—'}</td>
                    <td className="small">{r.machining.join(', ')}</td>
                    <td className="small">{r.note}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {c && tab === 'cabs' && (
          <table className="report">
            <thead>
              <tr>
                <th>#</th><th>Phòng</th><th>Tên tủ</th><th>Kiểu khung</th><th className="n">R × C × S</th><th className="n">Số tấm</th><th className="n">Thành tiền</th>
              </tr>
            </thead>
            <tbody>
              {c.cabinets.map((x, i) => (
                <tr key={x.id} onClick={() => select([x.id])}>
                  <td>{i + 1}</td>
                  <td>{x.room || '—'}</td>
                  <td>{x.name}</td>
                  <td>{x.frame}</td>
                  <td className="n">{fmt(x.size[0], 0)} × {fmt(x.size[1], 0)} × {fmt(x.size[2], 0)}</td>
                  <td className="n">{x.panels}</td>
                  <td className="n">{vnd(x.amount)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
