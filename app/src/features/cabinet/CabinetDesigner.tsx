// Right-hand "Thiết kế tủ" panel: Khung · Tạo tấm · Chỉnh tấm · Quản lý · Thư viện · Cài đặt.
import { useEffect, useState } from 'react';
import { useUi, type DesignerTab } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { Costing } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { fmt } from '../../shared/i18n';
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
  const { designerTab, set } = useUi();
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
  const { snap, gridSize, showZones, isolate, set } = useUi();
  return (
    <div className="designer-form">
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
