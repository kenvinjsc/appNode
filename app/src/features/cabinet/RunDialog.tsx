// Bảng "Dãy tủ": mặt đá, len chân liền, tấm lấp, che trần của một dãy tủ. Core sinh
// các tấm dãy và tự sinh lại khi tủ trong dãy đổi kích thước / vị trí; UI chỉ sửa luật.
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { RunDef, RunRules } from '../../core-api/types';
import { Icon } from '../../shared/icons';

export function RunDialog() {
  const { runOf, revision, set, materials } = useUi();
  const [run, setRun] = useState<RunDef | null>(null);
  useEffect(() => {
    if (!runOf) return setRun(null);
    Commands.getRuns()
      .then((r) => setRun(r.runs.find((x) => x.name === runOf) ?? null))
      .catch(() => setRun(null));
  }, [runOf, revision]);
  if (!runOf || !run) return null;
  const r = run.rules;
  const patch = (p: Partial<RunRules>) => void Commands.updateRun(run.name, { ...r, ...p }).catch(() => undefined);
  const Num = ({ k, label }: { k: keyof RunRules; label: string }) => <NumRow label={label} value={Number(r[k])} onCommit={(v) => patch({ [k]: v } as Partial<RunRules>)} />;
  const Flag = ({ k, label }: { k: keyof RunRules; label: string }) => (
    <label className="struct-row">
      <span>{label}</span>
      <input type="checkbox" checked={Boolean(r[k])} onChange={(e) => patch({ [k]: e.target.checked } as Partial<RunRules>)} />
    </label>
  );
  const stones = materials.filter((m) => m.kind === 'STONE');
  return (
    <div className="struct-dlg run-dlg" onPointerDown={(e) => e.stopPropagation()}>
      <div className="struct-head">
        <Icon name="cabinet" size={15} />
        <b>
          {run.name} · {run.cabinets.length} tủ
        </b>
        <div className="spacer" />
        <button className="icon-btn" title="Đóng" onClick={() => set({ runOf: null })}>
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className="struct-body">
        <h5 className="struct-section">Mặt đá</h5>
        <Flag k="countertop" label="Có mặt đá / mặt bàn" />
        {r.countertop && (
          <>
            <label className="struct-row">
              <span>Vật liệu</span>
              <select className="field" value={r.top_material} onChange={(e) => patch({ top_material: e.target.value })}>
                {(stones.length ? stones : materials).map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name}
                  </option>
                ))}
              </select>
            </label>
            <Num k="top_thickness" label="Dày mặt" />
            <Num k="overhang_front" label="Nhô trước" />
            <Num k="overhang_left" label="Nhô trái" />
            <Num k="overhang_right" label="Nhô phải" />
          </>
        )}
        <h5 className="struct-section">Len chân</h5>
        <Flag k="continuous_plinth" label="Len chân liền cả dãy" />
        {r.continuous_plinth && <Num k="plinth_setback" label="Len giật vào" />}
        <h5 className="struct-section">Tấm lấp · che trần</h5>
        <Num k="filler_left" label="Tấm lấp trái (0 = không)" />
        <Num k="filler_right" label="Tấm lấp phải (0 = không)" />
        <Num k="ceiling" label="Cao độ trần (0 = không che)" />
        <p className="muted small">Đổi kích thước hay dời tủ trong dãy: mặt đá, len chân, tấm lấp tự cập nhật.</p>
        <button
          className="btn tiny danger"
          onClick={() =>
            void Commands.deleteRun(run.name)
              .then(() => set({ runOf: null }))
              .catch(() => undefined)
          }
        >
          Xóa dãy (bỏ mặt đá, len chân liền)
        </button>
      </div>
    </div>
  );
}

function NumRow({ label, value, onCommit }: { label: string; value: number; onCommit: (v: number) => void }) {
  const [v, setV] = useState(String(value));
  useEffect(() => setV(String(Math.round(value * 100) / 100)), [value]);
  const done = () => {
    const n = Number(v.replace(',', '.'));
    if (Number.isFinite(n) && n !== value) onCommit(Math.max(0, n));
  };
  return (
    <label className="struct-row">
      <span>{label}</span>
      <input
        className="field"
        value={v}
        onChange={(e) => setV(e.target.value)}
        onBlur={done}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
        }}
      />
    </label>
  );
}
