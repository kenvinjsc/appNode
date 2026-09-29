// Right-hand "Thiết kế tủ" panel: Khung · Tạo tấm · Chỉnh tấm · Quản lý · Thư viện · Cài đặt.
import { useEffect, useState } from 'react';
import { findNode, useUi, type DesignerTab } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { Costing, TemplatesInfo } from '../../core-api/types';
import { Collapse } from '../../shared/ui';
import { cabinetOf } from './useZones';
import { Icon } from '../../shared/icons';
import { FIELD_LABEL, fmt } from '../../shared/i18n';
import { View } from '../../viewport/viewportBus';
import { MaterialBrowser } from '../materials/MaterialBrowser';
import { PropertiesPanel } from '../properties/PropertiesPanel';
import { CreateTab } from './CreateTab';
import { FrameTab } from './FrameTab';

const TABS: [DesignerTab, string][] = [
  ['frame', 'Khung'],
  ['create', 'Tạo tấm'],
  ['edit', 'Chỉnh tấm'],
  ['manage', 'Quản lý'],
  ['library', 'Thư viện'],
  ['settings', 'Cài đặt'],
];

export function CabinetDesigner() {
  const { designerTab, set, selection, tree, pinned } = useUi();
  // Contextual inspector: a part (or several objects) → Chỉnh tấm; a pinned zone → Tạo tấm.
  const selKey = selection.join(',');
  useEffect(() => {
    if (!selection.length) return;
    const kind = findNode(tree, selection[0])?.node.kind;
    if (selection.length > 1 || kind === 'PANEL' || kind === 'HARDWARE') set({ designerTab: 'edit' });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selKey]);
  const pinKey = `${pinned.cabinet}:${pinned.zones.join(',')}`;
  useEffect(() => {
    if (pinned.zones.length) set({ designerTab: 'create' });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pinKey]);
  return (
    <div className="panel designer">
      <div className="designer-tabs">
        {TABS.map(([k, l]) => (
          <button key={k} className={designerTab === k ? 'active' : ''} onClick={() => set({ designerTab: k })}>
            {l}
          </button>
        ))}
      </div>
      <div className="designer-body">
        {designerTab === 'frame' && <FrameTab />}
        {designerTab === 'create' && <CreateTab />}
        {designerTab === 'edit' && <PropertiesPanel embedded />}
        {designerTab === 'manage' && <ManageTab />}
        {designerTab === 'library' && <MaterialBrowser embedded />}
        {designerTab === 'settings' && <SettingsTab />}
      </div>
    </div>
  );
}

function ManageTab() {
  const { revision, tree, select, selection } = useUi();
  const [c, setC] = useState<Costing | null>(null);
  useEffect(() => {
    Queries.costing().then(setC).catch(() => undefined);
  }, [revision]);
  const rooms = new Map<string, Costing['cabinets']>();
  for (const cab of c?.cabinets ?? []) rooms.set(cab.room || 'Chưa đặt phòng', [...(rooms.get(cab.room || 'Chưa đặt phòng') ?? []), cab]);
  return (
    <div className="designer-form">
      {c && c.cabinets.length === 0 && <div className="empty">Chưa có tủ nào.</div>}
      {Array.from(rooms).map(([room, cabs]) => (
        <div key={room}>
          <div className="list-group">{room}</div>
          {cabs.map((cab) => (
            <div key={cab.id} className={`manage-row ${selection.includes(cab.id) ? 'selected' : ''}`} onClick={() => select([cab.id])}>
              <div>
                <b>{cab.name}</b> <span className="muted small">{cab.frame}</span>
                <div className="muted small">
                  {fmt(cab.size[0], 0)} × {fmt(cab.size[1], 0)} × {fmt(cab.size[2], 0)} · {cab.panels} tấm · {fmt(cab.amount, 0)} đ
                </div>
              </div>
              <span className="row-actions">
                <button className="icon-btn" title="Phóng vừa" onClick={(e) => { e.stopPropagation(); View.fit([cab.id], tree); }}>
                  <Icon name="fit" size={15} />
                </button>
                <button className="icon-btn danger" title="Xóa tủ" onClick={(e) => { e.stopPropagation(); if (confirm(`Xóa ${cab.name}?`)) void Commands.deleteObjects([cab.id]).catch(() => undefined); }}>
                  <Icon name="trash" size={15} />
                </button>
              </span>
            </div>
          ))}
        </div>
      ))}
      {c && c.cabinets.length > 0 && (
        <div className="manage-total">
          Tổng {c.cabinets.length} tủ · {c.cut_list.length} tấm · <b>{fmt(c.totals.total, 0)} đ</b>
          <button className="btn" onClick={() => useUi.getState().set({ drawer: 'report' })}>
            <Icon name="report" size={14} /> Báo cáo
          </button>
        </div>
      )}
    </div>
  );
}

function SettingsTab() {
  const { snap, gridSize, showZones, isolate, set, revision, tree, active } = useUi();
  const [lib, setLib] = useState<TemplatesInfo | null>(null);
  const [preset, setPreset] = useState('AIC Wardrobe Standard');
  useEffect(() => {
    Queries.templates()
      .then(setLib)
      .catch(() => setLib(null));
  }, [revision]);
  const cab = cabinetOf(tree, active);
  return (
    <div className="designer-form">
      <Collapse title={`Template tủ (${lib?.templates.length ?? 0})`} defaultOpen>
        <div className="tpl-list">
          {lib?.templates.map((t) => (
            <div className="tpl-item" key={t.name}>
              <span>{t.name}</span>
              <small>{t.size.map((v) => Math.round(v)).join('×')}</small>
              <button className="icon-btn danger" title="Xóa template" onClick={() => void Commands.deleteTemplate(t.name).catch(() => undefined)}>
                <Icon name="x" size={12} />
              </button>
            </div>
          ))}
          {!lib?.templates.length && <p className="muted small">Chưa có. Chuột phải một tủ → Lưu làm template.</p>}
        </div>
        <button
          className="btn"
          disabled={cab === null}
          onClick={() => cab !== null && set({ prompt: { title: 'Lưu tủ làm template', label: 'Tên template', value: '', ok: (v) => void Commands.saveTemplate(cab, v).catch(() => undefined) } })}
        >
          Lưu tủ đang chọn làm template
        </button>
      </Collapse>
      <Collapse title="Rule preset" defaultOpen>
        <div className="form-row">
          <label>Preset</label>
          <select className="field" value={preset} onChange={(e) => setPreset(e.target.value)}>
            {lib?.presets.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
                {p.builtin ? ' (có sẵn)' : ''}
              </option>
            ))}
          </select>
        </div>
        <p className="muted small">
          {Object.entries(lib?.presets.find((p) => p.name === preset)?.values ?? {})
            .map(([k, v]) => `${FIELD_LABEL[k] ?? k} ${v}`)
            .join(' · ')}
        </p>
        <div className="row-btns">
          <button className="btn primary" disabled={cab === null} onClick={() => cab !== null && void Commands.applyRulePreset([cab], preset).catch(() => undefined)}>
            Áp cho tủ đang chọn
          </button>
          <button
            className="btn"
            disabled={cab === null}
            onClick={() => cab !== null && set({ prompt: { title: 'Lưu rule preset', label: 'Tên preset (lấy thông số kết cấu của tủ đang chọn)', value: '', ok: (v) => void Commands.saveRulePreset(cab, v).catch(() => undefined) } })}
          >
            Lưu từ tủ
          </button>
          {lib?.presets.find((p) => p.name === preset && !p.builtin) && (
            <button className="btn" onClick={() => void Commands.deleteRulePreset(preset).catch(() => undefined)}>
              Xóa
            </button>
          )}
        </div>
      </Collapse>
      <label className="check toggle-row">
        <input type="checkbox" checked={showZones} onChange={(e) => set({ showZones: e.target.checked })} /> Hiện vùng (zone) khi tạo tấm
      </label>
      <label className="check toggle-row">
        <input type="checkbox" checked={isolate} onChange={(e) => set({ isolate: e.target.checked })} /> Cô lập tủ đang chọn
      </label>
      <label className="check toggle-row">
        <input type="checkbox" checked={snap} onChange={(e) => set({ snap: e.target.checked })} /> Bắt dính khi di chuyển
      </label>
      <div className="form-row">
        <label>Lưới</label>
        <select className="field" value={gridSize} onChange={(e) => set({ gridSize: Number(e.target.value) })}>
          {[1, 5, 10, 50, 100].map((g) => (
            <option key={g} value={g}>
              {g} mm
            </option>
          ))}
        </select>
      </div>
      <p className="muted small">Mặc định: ván 17.2 mm, hậu 8.6 mm, chỉ dán cạnh Đơn 1 mm. Đơn giá sửa trong Báo cáo → Costing Report (lưu theo dự án).</p>
      <button className="btn" onClick={() => set({ drawer: 'shortcuts' })}>
        <Icon name="keyboard" size={14} /> Phím tắt
      </button>
    </div>
  );
}
