// Báo cáo: Costing Report · Danh sách cắt · Cabinet List. Quantities come from
// the core; unit prices are edited here and stored in the project (undoable).
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { CostLine, Costing, CutGroup, CutRow, PricingMode } from '../../core-api/types';
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
  const [tab, setTab] = useState<'cost' | 'quote' | 'cut' | 'cabs'>('cost');
  const [grouped, setGrouped] = useState(false);
  const [c, setC] = useState<Costing | null>(null);
  useEffect(() => {
    Queries.costing().then(setC).catch(() => undefined);
  }, [revision]);
  const edgeText = (edges: [string, string][]) => edges.map(([e, code]) => `${EDGE_LABEL[e as keyof typeof EDGE_LABEL] ?? e} ${code}`).join(' / ');
  const csv = () => {
    if (!c) return;
    const head = grouped
      ? ['STT', 'Mã tấm', 'Tên tấm', 'Vật liệu', 'Dài', 'Rộng', 'Dày', 'Dài cắt', 'Rộng cắt', 'SL', 'Dán cạnh', 'Gia công']
      : ['STT', 'Mã tấm', 'Phòng', 'Tủ', 'Tên tấm', 'Vật liệu', 'Dài', 'Rộng', 'Dày', 'Dài cắt', 'Rộng cắt', 'SL', 'Dán cạnh', 'Gia công', 'Ghi chú'];
    const rows = grouped
      ? c.cut_groups.map((g, i) => [i + 1, g.codes.join(' '), g.name, g.material, fmt(g.length, 1), fmt(g.width, 1), g.thickness, fmt(g.cut_length, 1), fmt(g.cut_width, 1), g.qty, edgeText(g.edges), g.machining.join('; ')])
      : c.cut_list.map((r, i) => [i + 1, r.code, r.room, r.cabinet, r.full_name, r.material, fmt(r.length, 1), fmt(r.width, 1), r.thickness, fmt(r.cut_length, 1), fmt(r.cut_width, 1), r.qty, edgeText(r.edges), r.machining.join('; '), r.note]);
    const text = [head, ...rows].map((row) => row.map((x) => `"${String(x).replace(/"/g, '""')}"`).join(',')).join('\n');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['\ufeff' + text], { type: 'text/csv' }));
    a.download = grouped ? 'danh-sach-cat-gop.csv' : 'danh-sach-cat.csv';
    a.click();
  };
  const labels = () => {
    if (c) printLabels(c.cut_list);
  };
  return (
    <div className="drawer-panel wide report-window">
      <div className="panel-header">
        <div className="seg">
          <button className={tab === 'cost' ? 'active' : ''} onClick={() => setTab('cost')}>Costing Report</button>
          <button className={tab === 'quote' ? 'active' : ''} onClick={() => setTab('quote')}>Báo giá</button>
          <button className={tab === 'cut' ? 'active' : ''} onClick={() => setTab('cut')}>Danh sách cắt</button>
          <button className={tab === 'cabs' ? 'active' : ''} onClick={() => setTab('cabs')}>Cabinet List</button>
        </div>
        <div className="spacer" />
        {tab === 'cut' && (
          <>
            <label className="chk">
              <input type="checkbox" checked={grouped} onChange={(e) => setGrouped(e.target.checked)} /> Gộp tấm giống nhau
            </label>
            <button className="btn" onClick={labels}>
              <Icon name="report" size={14} /> In nhãn
            </button>
            <button className="btn" onClick={csv}>
              <Icon name="download" size={14} /> Excel / CSV
            </button>
          </>
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
        {c && tab === 'cut' && !grouped && (
          <div className="table-wrap">
            <table className="report">
              <thead>
                <tr>
                  <th>#</th><th>Mã tấm</th><th>Tủ</th><th>Tên tấm</th><th>Vật liệu</th><th className="n">Dài</th><th className="n">Rộng</th><th className="n">Dày</th><th className="n">Cắt (D×R)</th><th className="n">SL</th><th>Dán cạnh</th><th>Rãnh / gia công</th><th>Ghi chú</th>
                </tr>
              </thead>
              <tbody>
                {c.cut_list.map((r, i) => (
                  <tr key={r.id} onClick={() => select([r.id])}>
                    <td>{i + 1}</td>
                    <td className="small">{r.code}</td>
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
        {c && tab === 'cut' && grouped && <GroupTable groups={c.cut_groups} onPick={(ids) => select(ids)} />}
        {c && tab === 'quote' && <QuoteView c={c} />}
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

function GroupTable({ groups, onPick }: { groups: CutGroup[]; onPick: (ids: number[]) => void }) {
  const total = groups.reduce((n, g) => n + g.qty, 0);
  return (
    <div className="table-wrap">
      <p className="muted small">
        {groups.length} dòng · {total} tấm (gộp tấm cùng vật liệu, kích thước cắt, dán cạnh và gia công)
      </p>
      <table className="report">
        <thead>
          <tr>
            <th>#</th><th>Tên tấm</th><th>Vật liệu</th><th className="n">Dài</th><th className="n">Rộng</th><th className="n">Dày</th><th className="n">Cắt (D×R)</th><th className="n">SL</th><th>Dán cạnh</th><th>Mã tấm</th>
          </tr>
        </thead>
        <tbody>
          {groups.map((g, i) => (
            <tr key={i} onClick={() => onPick(g.ids)}>
              <td>{i + 1}</td>
              <td>{g.name}</td>
              <td>{g.material}</td>
              <td className="n">{fmt(g.length, 1)}</td>
              <td className="n">{fmt(g.width, 1)}</td>
              <td className="n">{g.thickness}</td>
              <td className="n">{fmt(g.cut_length, 1)} × {fmt(g.cut_width, 1)}</td>
              <td className="n"><b>{g.qty}</b></td>
              <td>{g.edges.map(([e]) => EDGE_LABEL[e]).join(' / ') || '—'}</td>
              <td className="small">{g.codes.join(', ')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const MODE_LABEL: Record<PricingMode, string> = { AUTO: 'Theo loại tủ', DETAIL: 'Bóc chi tiết', LINEAR_M: 'Mét dài', FACADE_M2: 'm² mặt đứng' };

function SettingInput({ k, value, suffix }: { k: string; value: number; suffix: string }) {
  const [v, setV] = useState(String(value));
  useEffect(() => setV(String(value)), [value]);
  const commit = () => {
    const n = Number(v.replace(/\./g, '').replace(',', '.'));
    if (Number.isFinite(n) && n !== value) void Commands.setPrice(k, n).catch(() => undefined);
  };
  return (
    <span className="quote-setting">
      <input className="price" value={v} onChange={(e) => setV(e.target.value)} onBlur={commit} onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()} />
      {suffix}
    </span>
  );
}

function QuoteView({ c }: { c: Costing }) {
  const q = c.quote;
  const pricing = new Map(c.cabinets.map((x) => [x.id, x.pricing]));
  const rooms = q.rooms;
  return (
    <div className="quote">
      <div className="quote-settings">
        <label>Hao hụt (bóc chi tiết) <SettingInput k="quote:waste_pct" value={q.settings.waste_pct} suffix="%" /></label>
        <label>Công <SettingInput k="quote:labor_pct" value={q.settings.labor_pct} suffix="%" /></label>
        <label>Lợi nhuận <SettingInput k="quote:margin_pct" value={q.settings.margin_pct} suffix="%" /></label>
        <label>VAT <SettingInput k="quote:vat_pct" value={q.settings.vat_pct} suffix="%" /></label>
      </div>
      {rooms.map((r) => (
        <section key={`${r.floor}|${r.room}`} className="cost-block">
          <div className="cost-head">
            <b>{[r.floor, r.room || 'Chưa gán phòng'].filter(Boolean).join(' · ')}</b>
            <span className="muted">{vnd(r.amount)}</span>
          </div>
          <table className="report cost">
            <thead>
              <tr>
                <th>Tủ</th><th>Cách tính</th><th className="n">Khối lượng</th><th className="n">Đơn giá</th><th className="n">Thành tiền</th>
              </tr>
            </thead>
            <tbody>
              {q.rows
                .filter((x) => x.floor === r.floor && x.room === r.room)
                .map((x) => (
                  <tr key={x.id}>
                    <td>{x.name}</td>
                    <td>
                      <select className="field" value={pricing.get(x.id) ?? 'AUTO'} onChange={(e) => void Commands.setParameter(x.id, 'pricing', e.target.value).catch(() => undefined)}>
                        {(Object.keys(MODE_LABEL) as PricingMode[]).map((m) => (
                          <option key={m} value={m}>
                            {m === 'AUTO' ? `${MODE_LABEL[m]} (${MODE_LABEL[x.mode]})` : MODE_LABEL[m]}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="n">
                      {fmt(x.qty, 2)} {x.unit}
                    </td>
                    <td className="n">{x.price_key ? <SettingInput k={x.price_key} value={x.price} suffix="" /> : vnd(x.price)}</td>
                    <td className="n">{vnd(x.amount)}</td>
                  </tr>
                ))}
            </tbody>
          </table>
        </section>
      ))}
      <div className="grand-total small-lines">
        <span>Cộng</span>
        <b>{vnd(q.subtotal)}</b>
        {q.margin > 0 && (
          <>
            <span>Lợi nhuận</span>
            <b>{vnd(q.margin)}</b>
          </>
        )}
        <span>VAT</span>
        <b>{vnd(q.vat)}</b>
        <span>TỔNG BÁO GIÁ</span>
        <b>{vnd(q.total)}</b>
      </div>
    </div>
  );
}

/** Nhãn tấm 60 × 40 mm (in từ trình duyệt): mã, tên, kích thước cắt, vật liệu, cạnh dán. */
function printLabels(rows: CutRow[]) {
  const esc = (s: string) => s.replace(/[&<>"]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[ch] ?? ch);
  const side = (r: CutRow, e: string) => (r.edges.some(([x]) => x === e) ? 'on' : '');
  const cells = rows
    .map(
      (r) => `<div class="lb">
  <div class="code">${esc(r.code)}</div>
  <div class="name">${esc(r.full_name)}</div>
  <div class="size">${fmt(r.cut_length, 1)} × ${fmt(r.cut_width, 1)} × ${r.thickness}</div>
  <div class="mat">${esc(r.material)}</div>
  <div class="edge"><i class="t ${side(r, 'TOP')}"></i><i class="b ${side(r, 'BOTTOM')}"></i><i class="l ${side(r, 'LEFT')}"></i><i class="r ${side(r, 'RIGHT')}"></i></div>
</div>`,
    )
    .join('');
  const css = `@page{size:A4;margin:8mm}body{font-family:sans-serif;margin:0}.grid{display:grid;grid-template-columns:repeat(3,60mm);gap:3mm}
.lb{width:60mm;height:40mm;border:1px solid #333;box-sizing:border-box;padding:2mm;position:relative;break-inside:avoid;font-size:8pt}
.code{font-weight:700;font-size:11pt}.name{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.size{font-size:12pt;font-weight:700;margin-top:1mm}
.mat{color:#444}.edge{position:absolute;right:2mm;bottom:2mm;width:14mm;height:10mm;border:1px dashed #999}
.edge i{position:absolute;background:transparent}.edge i.on{background:#e8590c}.edge .t{top:-1px;left:0;right:0;height:2px}.edge .b{bottom:-1px;left:0;right:0;height:2px}
.edge .l{left:-1px;top:0;bottom:0;width:2px}.edge .r{right:-1px;top:0;bottom:0;width:2px}`;
  const w = window.open('', '_blank');
  if (!w) return;
  w.document.write(`<!doctype html><html><head><meta charset="utf-8"><title>Nhãn tấm</title><style>${css}</style></head><body><div class="grid">${cells}</div><script>setTimeout(()=>print(),300)</script></body></html>`);
  w.document.close();
}
