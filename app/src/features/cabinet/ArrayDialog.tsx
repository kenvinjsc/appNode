// Nhân dãy tủ (D28): số lượng + trục (X / Y / Z) + khe, hoặc công thức kích thước từng tủ mới
// (`400,600,800`, `3*600`). Core nhân và đặt tủ (array_cabinet), một bước undo.
import { useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Icon } from '../../shared/icons';

export function ArrayDialog() {
  const { arrayOf, set } = useUi();
  const [count, setCount] = useState('1');
  const [axis, setAxis] = useState<0 | 1 | 2>(0);
  const [gap, setGap] = useState('0');
  const [sizes, setSizes] = useState('');
  if (arrayOf === null) return null;
  const close = () => set({ arrayOf: null });
  const run = () =>
    void Commands.arrayCabinet(arrayOf, Math.max(1, Math.round(Number(count)) || 1), axis, Number(gap.replace(',', '.')) || 0, sizes.trim() || undefined)
      .then(close)
      .catch(() => undefined);
  const stop = (e: React.KeyboardEvent) => {
    e.stopPropagation();
    if (e.key === 'Enter') run();
    if (e.key === 'Escape') close();
  };
  return (
    <div className="modal-back" onPointerDown={close}>
      <div className="modal array-dlg" onPointerDown={(e) => e.stopPropagation()}>
        <div className="struct-head">
          <Icon name="duplicate" size={15} />
          <b>Nhân dãy tủ</b>
          <div className="spacer" />
          <button className="icon-btn" title="Đóng" onClick={close}>
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="struct-body">
          <label className="struct-row">
            <span>Trục</span>
            <div className="seg">
              {(['X (ngang)', 'Y (chồng lên)', 'Z (ra trước)'] as const).map((l, i) => (
                <button key={l} className={axis === i ? 'active' : ''} onClick={() => setAxis(i as 0 | 1 | 2)}>
                  {l}
                </button>
              ))}
            </div>
          </label>
          <label className="struct-row">
            <span>Số tủ thêm</span>
            <input className="field" autoFocus value={count} disabled={!!sizes.trim()} onChange={(e) => setCount(e.target.value)} onKeyDown={stop} />
          </label>
          <label className="struct-row">
            <span>Khe (mm)</span>
            <input className="field" value={gap} onChange={(e) => setGap(e.target.value)} onKeyDown={stop} />
          </label>
          <label className="struct-row">
            <span>Kích thước từng tủ</span>
            <input className="field" placeholder="trống = như tủ gốc · 400,600,800 · 3*600" value={sizes} onChange={(e) => setSizes(e.target.value)} onKeyDown={stop} />
          </label>
          <div className="muted small">Có công thức thì số tủ = số giá trị; kích thước theo trục đã chọn (rộng / cao / sâu).</div>
          <button className="btn primary" onClick={run}>
            Nhân dãy
          </button>
        </div>
      </div>
    </div>
  );
}
