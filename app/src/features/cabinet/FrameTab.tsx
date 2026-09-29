// Tab "Khung": create a cabinet (Thông tin tủ) with joint and edge-band rules.
import { useEffect, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { CabinetKind, TemplatesInfo } from '../../core-api/types';
import { Collapse, Fieldset, Num, Radio } from '../../shared/ui';
import { View } from '../../viewport/viewportBus';
import { useCurrentCabinet, useTabAction } from './useZones';

const FRAMES: { kind: CabinetKind; label: string; size: [number, number, number] }[] = [
  { kind: 'BASE', label: '01. BếpDưới', size: [800, 850, 600] },
  { kind: 'WALL', label: '02. BếpTrên', size: [800, 720, 320] },
  { kind: 'WARDROBE', label: '03. TủQA', size: [1600, 2400, 600] },
  { kind: 'OPEN_SHELF', label: '04. Kệ', size: [800, 1600, 350] },
  { kind: 'DRAWER', label: '05. TủNgănKéo', size: [800, 720, 560] },
  { kind: 'DOOR', label: '06. Tủ1Cánh', size: [450, 720, 560] },
];

const ROOMS: [string, string][] = [
  ['', 'Trống'],
  ['Bếp', 'Bếp'],
  ['Khách', 'Khách'],
  ['PN1', 'PN1'],
  ['PN2', 'PN2'],
  ['PN3', 'PN3'],
  ['PN4', 'PN4'],
  ['WC1', 'WC1'],
  ['WC2', 'WC2'],
  ['WC3', 'WC3'],
  ['WC4', 'WC4'],
];

type Style = 'INSET' | 'OVERLAY' | 'RAILS';

export function FrameTab() {
  const { select, activeRoom, activeFloor, tree, set } = useUi();
  const floor = activeFloor ?? '';
  const current = useCurrentCabinet();
  const [room, setRoomRaw] = useState(activeRoom ?? '');
  // The room tab decides where new cabinets go; picking a room here switches the tab.
  useEffect(() => {
    if (activeRoom !== null) setRoomRaw(activeRoom);
  }, [activeRoom]);
  const setRoom = (r: string) => {
    setRoomRaw(r);
    if (activeRoom !== null || r) set({ activeRoom: r });
  };
  const cur = tree?.roots.find((n) => n.id === current);
  const sameRoom = !!cur && (cur.room ?? '') === room && (activeFloor === null || (cur.floor ?? '') === floor);
  const [name, setName] = useState('');
  const [frame, setFrame] = useState<CabinetKind>('BASE');
  const [size, setSize] = useState<[number, number, number]>([800, 850, 600]);
  const [top, setTop] = useState<Style>('RAILS');
  const [bottom, setBottom] = useState<Style>('INSET');
  const [groove, setGroove] = useState(0);
  const [mode, setMode] = useState<'EXPOSED_ONLY' | 'ALL' | 'NONE'>('EXPOSED_ONLY');
  const [band, setBand] = useState('DON-1');
  const [skip, setSkip] = useState<Record<string, boolean>>({ '17.2': false, '8.6': true });
  const [threshold, setThreshold] = useState(0.5);
  const [minLen, setMinLen] = useState(20);
  const [tpl, setTpl] = useState('');
  const [preset, setPreset] = useState('');
  const [lib, setLib] = useState<TemplatesInfo | null>(null);
  const revision = useUi((s) => s.revision);
  useEffect(() => {
    Queries.templates()
      .then(setLib)
      .catch(() => setLib(null));
  }, [revision]);

  const pickFrame = (k: CabinetKind) => {
    const f = FRAMES.find((x) => x.kind === k)!;
    setFrame(k);
    setSize(f.size);
    setTop(k === 'BASE' ? 'RAILS' : k === 'WARDROBE' ? 'OVERLAY' : 'INSET');
  };

  const create = async () => {
    const bandT = band === 'DON-0.5' ? 0.5 : band === 'DON-2' ? 2 : 1;
    const edge_rule = {
      mode,
      band_code: band,
      band_thickness: bandT,
      skip_thicknesses: Object.entries(skip).filter(([, v]) => v).map(([k]) => Number(k)),
      threshold,
      min_length: minLen,
    };
    try {
      const after = sameRoom ? current ?? undefined : undefined;
      const fl = activeFloor === null ? cur?.floor : floor;
      const r = tpl
        ? await Commands.insertTemplate({ name: tpl, width: size[0], height: size[1], depth: size[2], room, floor: fl, after })
        : await Commands.createCabinet(
        frame,
        null,
        { width: size[0], height: size[1], depth: size[2], top_style: top, bottom_style: bottom, edge_rule, back_groove: groove },
        // Next to the current cabinet only when it is in the same room; otherwise the core
        // continues the room's row (or opens a new area for a new room).
        { room, floor: activeFloor === null ? cur?.floor : floor, name: name.trim() || undefined, after: sameRoom ? current ?? undefined : undefined },
      );
      if (preset) await Commands.applyRulePreset([r.id], preset).catch(() => undefined);
      if (tpl && name.trim()) await Commands.setName(r.id, name.trim()).catch(() => undefined);
      select([r.id]);
      setName('');
      // Frame the new cabinet once the tree read model includes it.
      const fit = (n: number) => {
        const t = useUi.getState().tree;
        if (t?.roots.some((x) => x.id === r.id) || n <= 0) View.fit([r.id], t);
        else setTimeout(() => fit(n - 1), 60);
      };
      setTimeout(() => fit(15), 60);
    } catch {
      /* toast shown */
    }
  };
  useTabAction(() => void create(), [room, name, frame, size, top, bottom, groove, mode, band, skip, threshold, minLen, current, sameRoom, floor, activeFloor, cur, tpl, preset]);

  return (
    <div className="designer-form">
      <div className="form-row">
        <label>Tên phòng</label>
        <input className="field" value={room} onChange={(e) => setRoom(e.target.value)} />
      </div>
      <div className="room-chips">
        {ROOMS.map(([v, l]) => (
          <button key={l} type="button" className={`chip ${room === v ? 'on' : ''} ${l.startsWith('PN') ? 'pn' : l.startsWith('WC') ? 'wc' : ''}`} onClick={() => setRoom(v)}>
            {l}
          </button>
        ))}
      </div>
      <div className="form-row">
        <label>Tên tủ</label>
        <input className="field" value={name} placeholder="Tự đánh số theo kiểu khung" onChange={(e) => setName(e.target.value)} />
      </div>
      <Fieldset title="Kích thước (mm)">
        {(['Rộng', 'Cao', 'Sâu'] as const).map((l, i) => (
          <div className="form-row" key={l}>
            <label>{l}</label>
            <Num value={size[i]} onCommit={(v) => setSize(size.map((x, j) => (j === i ? v : x)) as [number, number, number])} />
          </div>
        ))}
        <div className="form-row">
          <label>Dạng tủ</label>
          <select className="field" value="BOX" disabled>
            <option value="BOX">Tủ hộp</option>
          </select>
        </div>
        <div className="form-row">
          <label>Kiểu khung</label>
          <select className="field" value={frame} onChange={(e) => pickFrame(e.target.value as CabinetKind)}>
            {FRAMES.map((f) => (
              <option key={f.kind} value={f.kind}>
                {f.label}
              </option>
            ))}
          </select>
        </div>
      </Fieldset>
      <Collapse title="Template & Rule preset" defaultOpen={!!lib?.templates.length}>
        <div className="form-row">
          <label>Template</label>
          <select
            className="field"
            value={tpl}
            onChange={(e) => {
              setTpl(e.target.value);
              const t = lib?.templates.find((x) => x.name === e.target.value);
              if (t) setSize(t.size);
            }}
          >
            <option value="">— Không (khung trống) —</option>
            {lib?.templates.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name} · {t.size.map((v) => Math.round(v)).join('×')}
              </option>
            ))}
          </select>
        </div>
        <div className="form-row">
          <label>Rule preset</label>
          <select className="field" value={preset} onChange={(e) => setPreset(e.target.value)}>
            <option value="">— Mặc định —</option>
            {lib?.presets.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
          </select>
        </div>
        <p className="muted small">
          Template giữ cấu trúc khoang (KHÓA / % / AUTO), cánh, ngăn kéo, luật và vật liệu; nhập W/H/D ở trên, phần mềm tính lại. Lưu template: chuột phải một tủ → Lưu làm template.
        </p>
      </Collapse>
      <Collapse title="Luật liên kết">
        <div className="form-row">
          <label>Nóc</label>
          <select className="field" value={top} onChange={(e) => setTop(e.target.value as Style)}>
            <option value="OVERLAY">Nóc phủ hồi</option>
            <option value="INSET">Nóc lọt giữa hồi</option>
            <option value="RAILS">2 thanh giằng</option>
          </select>
        </div>
        <div className="form-row">
          <label>Đáy</label>
          <select className="field" value={bottom} onChange={(e) => setBottom(e.target.value as Style)}>
            <option value="INSET">Đáy lọt giữa hồi</option>
            <option value="OVERLAY">Đáy phủ hồi</option>
          </select>
        </div>
        <div className="form-row">
          <label>Rãnh hậu</label>
          <Num value={groove} onCommit={setGroove} />
        </div>
        <p className="muted small">Rãnh hậu = 0: hậu lọt giữa hồi. &gt; 0: hậu chui vào rãnh ở hồi, nóc, đáy.</p>
      </Collapse>
      <Collapse title="Luật dán cạnh" defaultOpen>
        <Fieldset title="Kiểu dán">
          <Radio
            value={mode}
            onChange={setMode}
            options={[
              ['EXPOSED_ONLY', 'Dán hở bỏ khuất'],
              ['ALL', 'Dán toàn bộ'],
              ['NONE', 'Không dán (xóa tất cả)'],
            ]}
          />
        </Fieldset>
        <div className="form-row">
          <label>Loại chỉ dán</label>
          <select className="field" value={band} onChange={(e) => setBand(e.target.value)}>
            <option value="DON-0.5">Đơn 0.5mm</option>
            <option value="DON-1">Đơn 1mm</option>
            <option value="DON-2">Đơn 2mm</option>
            <option value="KEP-1">Kép 1mm</option>
          </select>
        </div>
        <Fieldset title="Độ dày không dán">
          {Object.keys(skip).map((t) => (
            <label key={t} className="check">
              <input type="checkbox" checked={skip[t]} onChange={(e) => setSkip({ ...skip, [t]: e.target.checked })} /> {t} mm
            </label>
          ))}
        </Fieldset>
        <div className="form-row">
          <label>Ngưỡng dán cạnh</label>
          <Num value={threshold} onCommit={setThreshold} />
        </div>
        <div className="form-row">
          <label>Bỏ cạnh ngắn ≤</label>
          <Num value={minLen} onCommit={setMinLen} />
        </div>
      </Collapse>
      <button className="btn primary tab-btn" onClick={() => void create()}>
        <b>[TAB]</b> Tạo tủ
      </button>
      {current !== null && <p className="muted small">Tủ mới được đặt nối tiếp bên phải tủ đang chọn.</p>}
    </div>
  );
}
