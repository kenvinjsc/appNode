// Bảng "Thuộc tính kết cấu" (như plugin): tabs Thông số chung · Liên kết · Hậu ·
// Thanh giằng · Len chân · Lùi đợt. Tabs and fields come from the core
// (get_structure); each tab's values can be saved as a small preset in the shared
// library and applied to any cabinet later.
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { MaterialSet, StructureField, StructureInfo } from '../../core-api/types';
import { Icon } from '../../shared/icons';

export function StructureDialog() {
  const { structureOf, revision, set, selection } = useUi();
  const [info, setInfo] = useState<StructureInfo | null>(null);
  const [tab, setTab] = useState('back');
  const [preset, setPreset] = useState('');
  const [standard, setStandard] = useState('');
  const [msets, setMsets] = useState<{ set: MaterialSet; builtin: boolean }[]>([]);
  const [mset, setMset] = useState('');
  useEffect(() => {
    if (structureOf === null) return;
    Commands.getMaterialSets()
      .then((r) => setMsets(r.sets))
      .catch(() => setMsets([]));
  }, [structureOf, revision]);
  useEffect(() => {
    if (structureOf === null) return setInfo(null);
    Commands.getStructure(structureOf)
      .then(setInfo)
      .catch(() => set({ structureOf: null }));
  }, [structureOf, revision, set]);
  if (structureOf === null || !info) return null;
  const cur = info.tabs.find((t) => t.key === tab) ?? info.tabs[0];
  const commit = (key: string, v: string) => void Commands.setParameter(info.cabinet, key, v).catch(() => undefined);
  // Apply to every selected cabinet when several are selected (else this one).
  const targets = selection.length > 1 ? selection : [info.cabinet];
  return (
    <div className="struct-dlg" onPointerDown={(e) => e.stopPropagation()}>
      <div className="struct-head">
        <Icon name="settings" size={15} />
        <b>Thuộc tính · {info.name}</b>
        <div className="spacer" />
        <button className="icon-btn" title="Đóng" onClick={() => set({ structureOf: null })}>
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className="struct-presets struct-std" title="Chuẩn xưởng = giá trị của mọi tab (trừ kích thước tủ), dùng lại cho mọi dự án">
        <b>Chuẩn xưởng</b>
        <select value={standard} onChange={(e) => setStandard(e.target.value)}>
          <option value="">— chọn —</option>
          {info.standards.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <button
          className="btn tiny"
          disabled={!standard}
          onClick={() =>
            void Commands.applyGroupPreset(targets, 'all', standard)
              .then(() => useUi.getState().toast({ kind: 'success', title: `Đã áp chuẩn “${standard}” cho ${targets.length} tủ` }))
              .catch(() => undefined)
          }
        >
          Áp
        </button>
        <button
          className="btn tiny on"
          onClick={() =>
            set({
              prompt: {
                title: 'Lưu chuẩn xưởng',
                label: 'Tên chuẩn (ví dụ: Xưởng MFC 18) — lưu mọi tab của tủ này',
                value: '',
                ok: (v) =>
                  void Commands.saveGroupPreset(info.cabinet, 'all', v)
                    .then(() => {
                      setStandard(v);
                      useUi.getState().toast({ kind: 'success', title: `Đã lưu chuẩn “${v}”` });
                    })
                    .catch(() => undefined),
              },
            })
          }
        >
          Lưu chuẩn
        </button>
        {standard && (
          <button className="icon-btn danger" title="Xóa chuẩn" onClick={() => void Commands.deleteGroupPreset('all', standard).then(() => setStandard('')).catch(() => undefined)}>
            <Icon name="x" size={12} />
          </button>
        )}
      </div>
      <div className="struct-presets struct-std" title="Bộ vật liệu: thùng / cánh / hậu + chỉ dán cánh và thùng">
        <b>Bộ vật liệu</b>
        <select value={mset} onChange={(e) => setMset(e.target.value)}>
          <option value="">— chọn —</option>
          {msets.map(({ set: m, builtin }) => (
            <option key={m.name} value={m.name}>
              {m.name}
              {builtin ? '' : ' (của tôi)'}
            </option>
          ))}
        </select>
        <button
          className="btn tiny"
          disabled={!mset}
          title="Áp cho tủ đang chọn"
          onClick={() =>
            void Commands.applyMaterialSet(mset, targets)
              .then((r) => useUi.getState().toast({ kind: 'success', title: `Đã áp “${mset}” cho ${r.cabinets} tủ` }))
              .catch(() => undefined)
          }
        >
          Áp
        </button>
        <button
          className="btn tiny"
          disabled={!mset || !info.room}
          title={info.room ? `Áp cho mọi tủ trong phòng ${info.room}` : 'Tủ chưa gán phòng'}
          onClick={() =>
            void Commands.applyMaterialSet(mset, [], info.room)
              .then((r) => useUi.getState().toast({ kind: 'success', title: `Đã áp “${mset}” cho ${r.cabinets} tủ phòng ${info.room}` }))
              .catch(() => undefined)
          }
        >
          Cả phòng
        </button>
        <button
          className="btn tiny on"
          title="Lưu vật liệu + chỉ dán của tủ này thành bộ"
          onClick={() =>
            set({
              prompt: {
                title: 'Lưu bộ vật liệu',
                label: 'Tên bộ (lưu vật liệu thùng / cánh / hậu và chỉ dán của tủ này)',
                value: '',
                ok: (v) =>
                  void Commands.saveMaterialSet(info.cabinet, v)
                    .then(() => {
                      setMset(v);
                      useUi.getState().toast({ kind: 'success', title: `Đã lưu bộ “${v}”` });
                    })
                    .catch(() => undefined),
              },
            })
          }
        >
          Lưu bộ
        </button>
      </div>
      <div className="struct-tabs">
        {info.tabs.map((t) => (
          <button key={t.key} className={t.key === cur.key ? 'on' : ''} onClick={() => setTab(t.key)}>
            {t.title}
          </button>
        ))}
      </div>
      <div className="struct-presets">
        <select value={preset} onChange={(e) => setPreset(e.target.value)} title="Mẫu đã lưu cho tab này (thư viện dùng chung mọi dự án)">
          <option value="">— Mẫu {cur.title} —</option>
          {cur.presets.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <button className="btn tiny" disabled={!preset} onClick={() => void Commands.applyGroupPreset(targets, cur.key, preset).catch(() => undefined)}>
          Áp
        </button>
        <button
          className="btn tiny on"
          onClick={() =>
            set({
              prompt: {
                title: `Lưu mẫu ${cur.title}`,
                label: 'Tên mẫu (lưu vào thư viện, dùng lại cho mọi dự án)',
                value: '',
                ok: (v) =>
                  void Commands.saveGroupPreset(info.cabinet, cur.key, v)
                    .then(() => {
                      setPreset(v);
                      useUi.getState().toast({ kind: 'success', title: `Đã lưu mẫu “${v}”` });
                    })
                    .catch(() => undefined),
              },
            })
          }
        >
          Lưu mẫu
        </button>
        {preset && (
          <button className="icon-btn danger" title="Xóa mẫu" onClick={() => void Commands.deleteGroupPreset(cur.key, preset).then(() => setPreset('')).catch(() => undefined)}>
            <Icon name="x" size={12} />
          </button>
        )}
      </div>
      {cur.key === 'top_rails' && !info.rails_active && (
        <p className="muted small warn">Thanh giằng chỉ dùng khi Nóc = "Thanh giằng" (tab Liên kết).</p>
      )}
      <div className="struct-body">
        {cur.fields.map((f, i) => (
          <StructField key={`${cur.key}-${f.key}-${i}`} f={f} onCommit={commit} />
        ))}
      </div>
    </div>
  );
}

function StructField({ f, onCommit }: { f: StructureField; onCommit: (key: string, v: string) => void }) {
  const [v, setV] = useState(String(f.value ?? ''));
  useEffect(() => setV(typeof f.value === 'number' ? String(Math.round(f.value * 100) / 100) : String(f.value ?? '')), [f.value]);
  if (f.kind === 'section') return <h5 className="struct-section">{f.label}</h5>;
  if (f.kind === 'bool')
    return (
      <label className="struct-row">
        <span>{f.label}</span>
        <input type="checkbox" checked={Boolean(f.value)} onChange={(e) => onCommit(f.key, e.target.checked ? 'on' : 'off')} />
      </label>
    );
  if (f.kind === 'select')
    return (
      <label className="struct-row">
        <span>{f.label}</span>
        <select className="field" value={String(f.value)} onChange={(e) => onCommit(f.key, e.target.value)}>
          {f.options?.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </label>
    );
  const done = () => v.trim() !== String(f.value ?? '') && onCommit(f.key, v.trim().replace(',', f.kind === 'number' ? '.' : ','));
  return (
    <label className="struct-row" title={f.hint}>
      <span>{f.label}</span>
      <input
        className="field"
        value={v}
        placeholder={f.hint}
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
