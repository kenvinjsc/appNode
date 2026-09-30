// Bản vẽ in (D29): core chiếu và xếp trang (get_drawing_sheet), UI chỉ vẽ primitive thành SVG
// theo mm giấy rồi mở hộp thoại in của trình duyệt (in ra PDF / máy in).
import { useState } from 'react';
import { findNode, useUi } from '../../app/uiStore';
import { Commands, type DrawingSheet, type SheetItem } from '../../core-api/commands';
import { Icon } from '../../shared/icons';

const esc = (s: string) => s.replace(/[&<>"]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[ch] ?? ch);

function itemSvg(it: SheetItem): string {
  switch (it.t) {
    case 'rect':
      return `<rect class="${it.cls}" x="${it.x}" y="${it.y}" width="${it.w}" height="${it.h}"/>`;
    case 'line':
      return `<line class="${it.cls}" x1="${it.x1}" y1="${it.y1}" x2="${it.x2}" y2="${it.y2}"/>`;
    case 'text': {
      const rot = it.rotate ? ` transform="rotate(${it.rotate} ${it.x} ${it.y})"` : '';
      return `<text class="${it.cls}" x="${it.x}" y="${it.y}" font-size="${it.size}" text-anchor="${it.anchor}"${rot}>${esc(it.s)}</text>`;
    }
  }
}

/** HTML in: mỗi trang một SVG kích thước thật (mm). */
export function sheetsHtml(d: DrawingSheet, title: string): string {
  const [pw, ph] = d.sheets[0]?.paper ?? [420, 297];
  const pages = d.sheets
    .map((s) => `<svg class="page" width="${s.paper[0]}mm" height="${s.paper[1]}mm" viewBox="0 0 ${s.paper[0]} ${s.paper[1]}">${s.items.map(itemSvg).join('')}</svg>`)
    .join('');
  return `<!doctype html><html><head><meta charset="utf-8"><title>${esc(title)}</title><style>
@page{size:${pw}mm ${ph}mm;margin:0}body{margin:0;font-family:"Segoe UI",Arial,sans-serif}.page{display:block;page-break-after:always;background:#fff}
rect{fill:none;stroke:#212529;stroke-width:.25}rect.part{fill:#fff}rect.front{fill:#fff;stroke-width:.3}rect.hw{fill:#f1f3f5;stroke:#868e96;stroke-width:.15}
rect.cab{fill:#f8f9fa;stroke-width:.3}rect.frame{stroke-width:.5}rect.cell{stroke-width:.2}
line{stroke:#212529;stroke-width:.18}line.open{stroke:#868e96;stroke-dasharray:1.5 1}line.ext{stroke:#868e96;stroke-width:.12}line.dim{stroke:#e8590c;stroke-width:.2}
text{fill:#212529}text.dimtext{fill:#e8590c}text.title{font-weight:700}text.key{fill:#868e96}text.val{font-weight:600}text.label{fill:#495057}
@media screen{body{background:#dee2e6}.page{margin:12px auto;box-shadow:0 1px 6px rgba(0,0,0,.25)}}
</style></head><body>${pages}<script>setTimeout(()=>print(),400)</script></body></html>`;
}

/** Nút "Bản vẽ in" + hộp tùy chọn (khổ giấy, hình, người vẽ). */
export function PrintSheetButton() {
  const [open, setOpen] = useState(false);
  const [paper, setPaper] = useState<'A3' | 'A4'>('A3');
  const [portrait, setPortrait] = useState(false);
  const [views, setViews] = useState<Record<string, boolean>>({ ELEVATION: true, PLAN: true, DETAIL: false });
  const [hideFronts, setHideFronts] = useState(true);
  const [drawer, setDrawer] = useState(() => {
    try {
      return localStorage.getItem('aic.drawer') ?? '';
    } catch {
      return '';
    }
  });
  const run = () => {
    const s = useUi.getState();
    // Phạm vi: phòng của tủ đang chọn (cùng tầng); chưa chọn thì cả dự án.
    const cab = s.selection.map((id) => findNode(s.tree, id)?.node).find((n) => n?.kind === 'CABINET');
    try {
      localStorage.setItem('aic.drawer', drawer);
    } catch {
      /* bỏ qua */
    }
    const req = {
      room: cab?.room || undefined,
      floor: cab?.room ? cab.floor || undefined : undefined,
      ids: cab && !cab.room ? [cab.id] : [],
      paper,
      portrait,
      views: Object.keys(views).filter((k) => views[k]),
      hide_fronts: hideFronts,
      drawer,
      date: new Date().toLocaleDateString('vi-VN'),
    };
    void Commands.getDrawingSheet(req)
      .then((d) => {
        const w = window.open('', '_blank');
        if (!w) return;
        w.document.write(sheetsHtml(d, `Bản vẽ ${cab?.room || cab?.name || ''}`.trim()));
        w.document.close();
        setOpen(false);
      })
      .catch(() => undefined);
  };
  return (
    <div className="print-sheet">
      <button className={`icon-btn ${open ? 'on' : ''}`} title="Bản vẽ in (A3 / A4, khung tên)" onClick={() => setOpen(!open)}>
        <Icon name="print" size={16} />
      </button>
      {open && (
        <div className="print-pop" onPointerDown={(e) => e.stopPropagation()}>
          <b>Bản vẽ in</b>
          <div className="muted small">Phạm vi: phòng của tủ đang chọn (chưa chọn: cả dự án).</div>
          <label className="struct-row">
            <span>Khổ giấy</span>
            <select value={`${paper}${portrait ? 'P' : 'L'}`} onChange={(e) => (setPaper(e.target.value.slice(0, 2) as 'A3' | 'A4'), setPortrait(e.target.value.endsWith('P')))}>
              <option value="A3L">A3 ngang</option>
              <option value="A3P">A3 dọc</option>
              <option value="A4L">A4 ngang</option>
              <option value="A4P">A4 dọc</option>
            </select>
          </label>
          {[
            ['ELEVATION', 'Mặt đứng'],
            ['PLAN', 'Mặt bằng'],
            ['DETAIL', 'Chi tiết từng tủ (trước + bên)'],
          ].map(([k, l]) => (
            <label key={k} className="struct-row">
              <span>{l}</span>
              <input type="checkbox" checked={views[k]} onChange={(e) => setViews({ ...views, [k]: e.target.checked })} />
            </label>
          ))}
          <label className="struct-row">
            <span>Chi tiết: ẩn cánh</span>
            <input type="checkbox" checked={hideFronts} onChange={(e) => setHideFronts(e.target.checked)} />
          </label>
          <label className="struct-row">
            <span>Người vẽ</span>
            <input className="field" value={drawer} onChange={(e) => setDrawer(e.target.value)} onKeyDown={(e) => e.stopPropagation()} />
          </label>
          <button className="btn primary" onClick={run}>
            Xem &amp; in
          </button>
        </div>
      )}
    </div>
  );
}
