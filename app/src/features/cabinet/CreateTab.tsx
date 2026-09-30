// Tab "Tạo tấm": add horizontal / vertical / back panels, doors, drawers and
// accessories into pinned zones. The core splits zones and sizes every part.
import { useEffect, useState } from 'react';
import { useUi, type CreateType } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import type { DoorKind, HingeSide, Lock, Mount, SplitKind, StopRail, ZonesInfo, AccessoryInfo } from '../../core-api/types';
import { fmt } from '../../shared/i18n';
import { Icon } from '../../shared/icons';
import { Collapse, Fieldset, LockRow, Num, QuickNums, Radio, Seg, Stepper } from '../../shared/ui';
import { useCurrentCabinet, useTabAction, useZones } from './useZones';

const TYPES: { t: CreateType; title: string; glyph: JSX.Element }[] = [
  { t: 'horizontal', title: 'Tấm ngang (kệ)', glyph: <path d="M5 12h14" strokeWidth="2.6" /> },
  { t: 'vertical', title: 'Tấm đứng (hông giữa)', glyph: <path d="M12 5v14" strokeWidth="2.6" /> },
  { t: 'back', title: 'Tấm hậu phụ', glyph: <rect x="8" y="4" width="8" height="16" strokeWidth="2" /> },
  { t: 'door', title: 'Cánh', glyph: <><rect x="7" y="4" width="10" height="16" strokeWidth="2" /><path d="M14 11v2" strokeWidth="2" /></> },
  { t: 'drawer', title: 'Ngăn kéo', glyph: <><rect x="4" y="8" width="16" height="8" strokeWidth="2" /><path d="M10 12h4" strokeWidth="2" /></> },
  { t: 'link', title: 'Liên kết (phụ kiện)', glyph: <><circle cx="9" cy="10" r="5" strokeWidth="2" /><circle cx="15" cy="14" r="5" strokeWidth="2" /></> },
];

function Glyph({ children }: { children: JSX.Element }) {
  return (
    <svg width={26} height={26} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeLinecap="round">
      {children}
    </svg>
  );
}

export function CreateTab() {
  const { createType, set, pinned } = useUi();
  const cabinet = useCurrentCabinet();
  const info = useZones(cabinet);
  // Zones to act on: pinned ones, else the root zone of the current cabinet.
  const zones = pinned.cabinet === cabinet && pinned.zones.length ? pinned.zones : info ? [info.zones[0].id] : [];
  const pinnedCount = pinned.cabinet === cabinet ? pinned.zones.length : 0;
  return (
    <div className="designer-form">
      <div className="muted small">
        Tạo tấm — đã ghim {pinnedCount} vùng (click tủ để ghim)
        {cabinet !== null && pinnedCount === 0 && info && ' · dùng vùng lọt lòng toàn tủ'}
      </div>
      <div className="type-icons">
        {TYPES.map((x) => (
          <button key={x.t} className={createType === x.t ? 'on' : ''} title={x.title} onClick={() => set({ createType: x.t })}>
            <Glyph>{x.glyph}</Glyph>
          </button>
        ))}
      </div>
      {cabinet === null ? (
        <div className="empty">Chọn một tủ trong khung nhìn (hoặc tạo tủ ở tab Khung), rồi click vào vùng trống để ghim.</div>
      ) : (
        <>
          {createType === 'horizontal' && <SplitForm key="h" cabinet={cabinet} zones={zones} info={info} axis={1} />}
          {createType === 'vertical' && <SplitForm key="v" cabinet={cabinet} zones={zones} info={info} axis={0} />}
          {createType === 'back' && <SplitForm key="b" cabinet={cabinet} zones={zones} info={info} axis={2} />}
          {createType === 'door' && <DoorForm cabinet={cabinet} zones={zones} />}
          {createType === 'drawer' && <DrawerForm cabinet={cabinet} zones={zones} />}
          {createType === 'link' && <LinkForm cabinet={cabinet} zones={zones} info={info} />}
        </>
      )}
    </div>
  );
}

const AXIS_LABELS: Record<number, [string, string]> = { 0: ['Cách trái', 'Cách phải'], 1: ['Cách dưới', 'Cách trên'], 2: ['Cách sau', 'Cách trước'] };

function SplitForm({ cabinet, zones, info, axis }: { cabinet: number; zones: number[]; info: ZonesInfo | null; axis: 0 | 1 | 2 }) {
  const kinds: [SplitKind, string][] = axis === 1 ? [['SHELF_ADJUSTABLE', 'KệDiĐộng'], ['SHELF_FIXED', 'KệCốĐịnh']] : axis === 0 ? [['DIVIDER', 'HôngGiữa']] : [['BACK_SUB', 'HậuPhụ']];
  const [kind, setKind] = useState<SplitKind>(kinds[0][0]);
  const [count, setCount] = useState(1);
  const [thick, setThick] = useState(axis === 2 ? 8.6 : 17.2);
  const [lock, setLock] = useState<Lock>(axis === 2 ? 'FROM_START' : 'RATIO');
  const [value, setValue] = useState(axis === 2 ? 0 : 50);
  const [tilt, setTilt] = useState<[number, number]>([0, 0]);
  const [frame, setFrame] = useState(false);
  // Preview of the resulting positions for the first pinned zone (display only).
  const zb = info?.zones.find((z) => z.id === zones[0]);
  const L = zb ? zb.size[axis] : 0;
  const free = L - thick;
  const start = lock === 'RATIO' ? (value / 100) * free : lock === 'FROM_START' ? value : free - value;
  const shown = {
    ratio: free > 0 ? Math.round((start / free) * 1000) / 10 : 0,
    start: Math.round(start * 10) / 10,
    end: Math.round((free - start) * 10) / 10,
  };
  const cell = count > 1 ? (L - count * thick) / (count + 1) : null;
  const [la, lb] = AXIS_LABELS[axis];
  const add = () =>
    void Commands.zoneAddPanels({ cabinet, zones, kind, count, thickness: thick, lock: count > 1 ? 'EVEN' : lock, value, tilt_deg: tilt }).then(() =>
      useUi.getState().set({ pinned: { cabinet, zones: [] } }),
    ).catch(() => undefined);
  useTabAction(add, [cabinet, zones.join(','), kind, count, thick, lock, value, tilt]);
  return (
    <>
      <div className="form-row">
        <label>Số lượng</label>
        <Stepper value={count} onChange={setCount} min={1} max={30} />
      </div>
      <QuickNums value={count} onChange={setCount} />
      <div className="form-row">
        <label>Độ dày (mm)</label>
        <Num value={thick} onCommit={setThick} />
      </div>
      <Fieldset title="Loại">
        <Radio value={kind} onChange={setKind} options={kinds} />
      </Fieldset>
      <Fieldset title="Vị trí">
        {count > 1 ? (
          <div className="muted small">Chia đều {count} tấm trong vùng{cell !== null && zb ? ` — lọt lòng mỗi ô ${fmt(cell, 1)} mm` : ''}.</div>
        ) : (
          <>
            <LockRow label="Tỷ lệ (%)" unit="%" locked={lock === 'RATIO'} value={zb ? shown.ratio : lock === 'RATIO' ? value : ''} onLock={() => { setLock('RATIO'); setValue(zb ? shown.ratio : 50); }} onCommit={(v) => { setLock('RATIO'); setValue(v); }} />
            <LockRow label={la} unit="mm" locked={lock === 'FROM_START'} value={zb ? shown.start : lock === 'FROM_START' ? value : ''} onLock={() => { setLock('FROM_START'); setValue(zb ? shown.start : 0); }} onCommit={(v) => { setLock('FROM_START'); setValue(v); }} />
            <LockRow label={lb} unit="mm" locked={lock === 'FROM_END'} value={zb ? shown.end : lock === 'FROM_END' ? value : ''} onLock={() => { setLock('FROM_END'); setValue(zb ? shown.end : 0); }} onCommit={(v) => { setLock('FROM_END'); setValue(v); }} />
          </>
        )}
      </Fieldset>
      <p className="muted small">Bấm tên dòng để đổi cách khai (tấm giữ nguyên vị trí). Tham số có chấm đỏ được khóa khi vùng đổi kích thước.</p>
      {zb && count === 1 && (
        <div className="form-row">
          <label>Lọt lòng mỗi ô</label>
          <span className="ro-box">
            {fmt(shown.start, 1)} / {fmt(shown.end, 1)}
          </span>
        </div>
      )}
      {axis !== 0 && (
        <div className="form-row">
          <label>{axis === 1 ? 'Nghiêng trước-sau (°)' : 'Nghiêng trái-phải (°)'}</label>
          <Num value={tilt[0]} unit="°" onCommit={(v) => setTilt([v, tilt[1]])} />
        </div>
      )}
      <div className="form-row">
        <label>{axis === 0 ? 'Nghiêng trước-sau (°)' : axis === 1 ? 'Nghiêng trái-phải (°)' : 'Nghiêng trên-dưới (°)'}</label>
        <Num value={tilt[1]} unit="°" onCommit={(v) => setTilt([tilt[0], v])} />
      </div>
      {axis === 1 && (
        <Collapse title={`Khung kệ: ${frame ? 'Có' : 'Không'}`}>
          <Seg value={frame ? 'y' : 'n'} onChange={(v) => setFrame(v === 'y')} options={[['n', 'Không'], ['y', 'Có']]} />
        </Collapse>
      )}
      <button className="btn primary tab-btn" onClick={add} disabled={!zones.length}>
        <b>[TAB]</b> Thêm
      </button>
    </>
  );
}

const DOOR_TYPES: { v: string; kind: DoorKind; label: string }[] = [
  { v: 'S1', kind: 'SINGLE', label: 'Đơn - "CửaĐơn 01"' },
  { v: 'D1', kind: 'DOUBLE', label: 'Đôi - "CửaĐôi 01"' },
  { v: 'SL', kind: 'SLIDING', label: 'Lùa - "CửaLùaThường"' },
];

function DoorForm({ cabinet, zones }: { cabinet: number; zones: number[] }) {
  const [cols, setCols] = useState(1);
  const [rows, setRows] = useState(1);
  const [thick, setThick] = useState(17.2);
  const [type, setType] = useState('S1');
  const [mount, setMount] = useState<Mount>('OVERLAY');
  const [hinge, setHinge] = useState<HingeSide>('LEFT');
  const [stop, setStop] = useState<StopRail>('NONE');
  const [sp, setSp] = useState({ height: 75, cover_up: 25, leg_depth: 40, setback: 20.2 });
  const kind = DOOR_TYPES.find((d) => d.v === type)!.kind;
  const add = () =>
    void Commands.zoneAddDoors({ cabinet, zones, kind, cols: kind === 'DOUBLE' ? Math.max(2, cols) : cols, rows, mount, hinge, thickness: thick, stop: { kind: stop, ...sp } })
      .then(() => useUi.getState().set({ pinned: { cabinet, zones: [] } }))
      .catch(() => undefined);
  useTabAction(add, [cabinet, zones.join(','), cols, rows, thick, kind, mount, hinge, stop, sp]);
  return (
    <>
      <div className="form-row"><label>Số cánh ngang</label></div>
      <QuickNums value={cols} onChange={setCols} />
      <div className="form-row"><label>Số cánh dọc</label></div>
      <QuickNums value={rows} onChange={setRows} />
      <div className="form-row">
        <label>Độ dày (mm)</label>
        <Num value={thick} onCommit={setThick} />
      </div>
      <div className="form-row"><label>Mặt gắn</label><span className="ro-box">Trước</span></div>
      <Collapse title={`Kiểu và Tên cửa: ${DOOR_TYPES.find((d) => d.v === type)!.label}`} defaultOpen>
        <Radio value={type} onChange={setType} options={DOOR_TYPES.map((d) => [d.v, d.label] as [string, string])} />
      </Collapse>
      <Collapse title={`Kiểu kết cấu: ${mount === 'OVERLAY' ? 'Phủ bì' : 'Lọt lòng'}`}>
        <Radio value={mount} onChange={setMount} options={[['OVERLAY', 'Phủ bì'], ['INSET', 'Lọt lòng']]} />
      </Collapse>
      {kind !== 'SLIDING' && (
        <Collapse title={`Lắp lề: ${kind === 'DOUBLE' ? (hinge === 'TOP' || hinge === 'BOTTOM' ? 'Trên / Dưới' : 'Trái / Phải') : { LEFT: 'Trái', RIGHT: 'Phải', TOP: 'Trên', BOTTOM: 'Dưới' }[hinge]}`}>
          {kind === 'DOUBLE' ? (
            <Radio value={hinge === 'TOP' || hinge === 'BOTTOM' ? 'TOP' : 'LEFT'} onChange={(v) => setHinge(v as HingeSide)} options={[['LEFT', 'Trái / Phải'], ['TOP', 'Trên / Dưới']]} />
          ) : (
            <Radio value={hinge} onChange={setHinge} options={[['LEFT', 'Trái'], ['RIGHT', 'Phải'], ['TOP', 'Trên'], ['BOTTOM', 'Dưới']]} />
          )}
        </Collapse>
      )}
      <Collapse title={`Thanh chặn: ${stop === 'NONE' ? 'Không' : 'Nóc'}`}>
        <div className="muted small">Vị trí thanh chặn: Nóc</div>
        <Seg value={stop} onChange={setStop} options={[['NONE', 'Không'], ['L_SHAPE', 'Chữ L'], ['STRAIGHT', 'Thẳng']]} />
        {stop !== 'NONE' && (
          <>
            {([['height', 'Cao vùng'], ['cover_up', 'Cửa phủ lên'], ['leg_depth', 'Sâu chân'], ['setback', 'Lùi thanh']] as const).map(([k, l]) => (
              <div className="form-row" key={k}>
                <label>{l} (mm)</label>
                <Num value={sp[k]} onCommit={(v) => setSp({ ...sp, [k]: v })} />
              </div>
            ))}
          </>
        )}
      </Collapse>
      <button className="btn primary tab-btn" onClick={add} disabled={!zones.length}>
        <b>[TAB]</b> Thêm cửa
      </button>
    </>
  );
}

function DrawerForm({ cabinet, zones }: { cabinet: number; zones: number[] }) {
  const [cols, setCols] = useState(1);
  const [count, setCount] = useState(2);
  const [thick, setThick] = useState(17.2);
  const [mount, setMount] = useState<Mount>('OVERLAY');
  const [box, setBox] = useState(true);
  const add = () =>
    void Commands.zoneAddDrawers({ cabinet, zones, count, cols, mount, thickness: thick, with_box: box })
      .then(() => useUi.getState().set({ pinned: { cabinet, zones: [] } }))
      .catch(() => undefined);
  useTabAction(add, [cabinet, zones.join(','), cols, count, thick, mount, box]);
  return (
    <>
      <div className="form-row"><label>Số cột</label></div>
      <QuickNums value={cols} onChange={setCols} />
      <div className="form-row"><label>Số tầng</label></div>
      <QuickNums value={count} onChange={setCount} />
      <div className="form-row">
        <label>Độ dày (mm)</label>
        <Num value={thick} onCommit={setThick} />
      </div>
      <div className="form-row">
        <label>Loại</label>
        <select className="field" value="RAYBI">
          <option value="RAYBI">01. RayBi-Ván17mm</option>
        </select>
      </div>
      <Collapse title={`Kiểu mặt: ${mount === 'OVERLAY' ? 'Phủ bì' : 'Lọt lòng'}`} defaultOpen>
        <Radio value={mount} onChange={setMount} options={[['OVERLAY', 'Phủ bì'], ['INSET', 'Lọt lòng']]} />
      </Collapse>
      <label className="check toggle-row">
        <input type="checkbox" checked={box} onChange={(e) => setBox(e.target.checked)} /> Tạo hộc kéo (thành, hậu, đáy + ray bi)
      </label>
      <button className="btn primary tab-btn" onClick={add} disabled={!zones.length}>
        <b>[TAB]</b> Thêm ngăn kéo
      </button>
    </>
  );
}

function LinkForm({ cabinet, zones, info }: { cabinet: number; zones: number[]; info: ZonesInfo | null }) {
  const [dir, setDir] = useState<'V' | 'H'>('V');
  const [kind, setKind] = useState<'OVAL_RAIL' | 'RAYBI' | 'ACCESSORY'>('OVAL_RAIL');
  const [offset, setOffset] = useState(60);
  const [accs, setAccs] = useState<AccessoryInfo[]>([]);
  const [code, setCode] = useState('');
  useEffect(() => {
    Commands.getAccessories(cabinet, zones)
      .then((r) => setAccs(r.accessories))
      .catch(() => setAccs([]));
  }, [cabinet, zones.join(',')]); // eslint-disable-line react-hooks/exhaustive-deps
  const add = () => {
    if (kind === 'RAYBI') return;
    if (kind === 'ACCESSORY' && !code) return;
    void Commands.zoneAddLink(cabinet, zones, kind, offset, kind === 'ACCESSORY' ? code : undefined)
      .then((r) => {
        if (r.misfit_zones.length) useUi.getState().toast({ kind: 'error', title: 'Phụ kiện không vừa khoang', detail: 'Đã thêm, khoang được tô đỏ trên 2D. Đổi kích thước tủ / khoang cho vừa.' });
        useUi.getState().set({ pinned: { cabinet, zones: [] } });
      })
      .catch(() => undefined);
  };
  useTabAction(add, [cabinet, zones.join(','), kind, offset, code]);
  const atts = info?.attachments ?? [];
  const label = (a: (typeof atts)[number]) => {
    if (a.link?.kind === 'ACCESSORY') return `${accs.find((x) => x.code === a.link?.code)?.name ?? a.link.code} · vùng #${a.zone}`;
    if (a.link) return `Thanh Oval · vùng #${a.zone}`;
    if (a.front?.type === 'DOORS') return `Cánh ${a.front.kind === 'DOUBLE' ? 'Đôi' : a.front.kind === 'SLIDING' ? 'Lùa' : 'Đơn'} ${a.front.cols}×${a.front.rows} · vùng #${a.zone}`;
    if (a.front?.type === 'DRAWERS') return `RayBi · Ngăn kéo ${a.front.count * a.front.cols} bộ · vùng #${a.zone}`;
    return `#${a.uid}`;
  };
  return (
    <>
      <div className="muted small">Chọn 1 vùng trong tủ + 1 liên kết, rồi [TAB] để thêm.</div>
      <Fieldset title="Hướng áp">
        <Radio value={dir} onChange={setDir} options={[['V', 'Đứng'], ['H', 'Ngang']]} />
      </Fieldset>
      <Fieldset title="Liên kết">
        <Radio
          value={kind}
          onChange={setKind}
          options={[
            ['RAYBI', 'RayBi (tự gắn theo ngăn kéo)', true],
            ['OVAL_RAIL', 'Thanh Oval'],
            ['ACCESSORY', 'Phụ kiện khoang'],
          ]}
        />
        {kind === 'ACCESSORY' && (
          <div className="acc-list">
            {accs.map((a) => (
              <label key={a.code} className={`acc-row ${a.fits ? 'fit' : zones.length ? 'nofit' : ''}`} title={`Lọt lòng ${a.max_w ? `${a.min_w}–${a.max_w}` : `≥ ${a.min_w}`} · sâu ≥ ${a.min_d} · cao ≥ ${a.min_h}`}>
                <input type="radio" name="acc" checked={code === a.code} onChange={() => setCode(a.code)} />
                <span>{a.name}</span>
                {zones.length > 0 && <em>{a.fits ? 'vừa' : 'không vừa'}</em>}
              </label>
            ))}
          </div>
        )}
        <div className="muted small">Ray Âm Hafele, CửaLùa Hafele 50IF, Ghép34mm: sắp có.</div>
      </Fieldset>
      <div className="form-row">
        <label>Cách nóc vùng</label>
        <Num value={offset} onCommit={setOffset} />
      </div>
      <button className="btn primary tab-btn" onClick={add} disabled={!zones.length}>
        <b>[TAB]</b> Thêm liên kết
      </button>
      <Fieldset title={`Liên kết đã áp (${atts.length})`}>
        {atts.length === 0 && <div className="muted small">Chưa có.</div>}
        {atts.map((a) => (
          <div className="att-row" key={a.uid}>
            <span>{label(a)}</span>
            <button className="icon-btn danger" title="Gỡ" onClick={() => void Commands.zoneRemove(cabinet, a.uid).catch(() => undefined)}>
              <Icon name="x" size={14} />
            </button>
          </div>
        ))}
      </Fieldset>
    </>
  );
}
