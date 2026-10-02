// Công cụ "Chia khoang" (như Chia ngang / Chia dọc của plugin): nhập công thức,
// rồi bấm vào một khoang trong 3D hoặc trên bản vẽ 2D để chia. Công thức và cách
// tính kích thước khoang nằm ở core (`split_zone`); UI chỉ gửi yêu cầu.
import { useEffect } from 'react';
import { useUi, type SplitToolState } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { ObjectId, SplitKind } from '../../core-api/types';
import { Icon } from '../../shared/icons';

const DEFAULT: SplitToolState = { dir: 'H', formula: '500', fromEnd: true, panel: true, adjustable: false };

export function openSplitTool(dir?: 'H' | 'V') {
  const s = useUi.getState();
  const cur = s.splitTool ?? DEFAULT;
  s.set({ splitTool: { ...cur, dir: dir ?? cur.dir }, zoneMenu: null });
}

function kindOf(t: SplitToolState): SplitKind {
  if (!t.panel) return t.dir === 'H' ? 'VIRTUAL_H' : 'VIRTUAL_V';
  if (t.dir === 'V') return 'DIVIDER';
  return t.adjustable ? 'SHELF_ADJUSTABLE' : 'SHELF_FIXED';
}

/** Chia khoang `zone` của tủ `cabinet` theo công cụ đang mở. */
export function applySplit(cabinet: ObjectId, zone: number): Promise<unknown> {
  const t = useUi.getState().splitTool;
  if (!t) return Promise.resolve();
  return Commands.splitZone({ cabinet, zone, kind: kindOf(t), formula: t.formula, from_end: t.fromEnd }).catch(() => undefined);
}

export function SplitDialog() {
  const { splitTool: t, set } = useUi();
  useEffect(() => {
    if (!t) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !(e.target instanceof HTMLInputElement)) set({ splitTool: null });
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [t, set]);
  if (!t) return null;
  const patch = (p: Partial<SplitToolState>) => set({ splitTool: { ...t, ...p } });
  const YesNo = ({ label, value, on }: { label: string; value: boolean; on: (v: boolean) => void }) => (
    <label className="struct-row">
      <span>{label}</span>
      <select value={value ? 'y' : 'n'} onChange={(e) => on(e.target.value === 'y')}>
        <option value="y">Có</option>
        <option value="n">Không</option>
      </select>
    </label>
  );
  return (
    <div className="struct-dlg split-dlg" onPointerDown={(e) => e.stopPropagation()}>
      <div className="struct-head">
        <Icon name="divider" size={15} />
        <div className="seg">
          <button className={t.dir === 'H' ? 'active' : ''} onClick={() => patch({ dir: 'H' })}>
            Chia ngang
          </button>
          <button className={t.dir === 'V' ? 'active' : ''} onClick={() => patch({ dir: 'V' })}>
            Chia dọc
          </button>
        </div>
        <div className="spacer" />
        <button className="icon-btn" title="Đóng (Esc)" onClick={() => set({ splitTool: null })}>
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className="struct-body">
        <label className="struct-row">
          <span>Công thức chia</span>
          <input
            autoFocus
            value={t.formula}
            onChange={(e) => patch({ formula: e.target.value })}
            onKeyDown={(e) => e.key === 'Escape' && (e.target as HTMLInputElement).blur()}
          />
        </label>
        <YesNo label={t.dir === 'H' ? 'Trên xuống dưới' : 'Phải sang trái'} value={t.fromEnd} on={(v) => patch({ fromEnd: v })} />
        <YesNo label="Tạo tấm" value={t.panel} on={(v) => patch({ panel: v })} />
        {t.dir === 'H' && t.panel && <YesNo label="Đợt di động" value={t.adjustable} on={(v) => patch({ adjustable: v })} />}
        <div className="split-help">
          <b>Bấm vào khoang</b> trong 3D hoặc 2D để chia. <code>500</code> khoang 500 + phần còn lại · <code>500,300</code> ·{' '}
          <code>30%,*</code> · <code>3*400</code> · <code>/3</code> chia đều 3.
          {!t.panel && ' Không tạo tấm: chỉ tách khoang để gắn cánh / ngăn kéo từng phần.'}
        </div>
      </div>
    </div>
  );
}
