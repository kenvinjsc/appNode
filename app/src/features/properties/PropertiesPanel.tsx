// Property sheet rendered generically from the core's read model. Edits are
// sent as set_parameter; the core validates, recomputes and emits events.
import { useEffect, useRef, useState } from 'react';
import { useUi } from '../../app/uiStore';
import { Commands } from '../../core-api/commands';
import { Queries } from '../../core-api/queries';
import type { FlatPanel, PropertyField, PropertySheet } from '../../core-api/types';
import { Icon } from '../../shared/icons';
import { FIELD_LABEL, GROUP_LABEL, KIND_LABEL, OPTION_LABEL, PURPOSE_LABEL, fmt, roleLabel } from '../../shared/i18n';

const TAB_GROUPS: Record<string, string[]> = {
  params: ['general', 'size', 'zone_position', 'door', 'drawer', 'link', 'stretch', 'constraints', 'offset', 'construction', 'content', 'position', 'rotation', 'derived', 'relations'],
  material: ['material', 'edges', 'edge_rule'],
  machining: ['manufacturing'],
};

export function PropertiesPanel({ embedded = false }: { embedded?: boolean }) {
  const { active, selection, revision, propertiesTab, set } = useUi();
  const [sheet, setSheet] = useState<PropertySheet | null>(null);
  const [flat, setFlat] = useState<FlatPanel | null>(null);

  const multi = selection.length > 1;
  const selKey = selection.join(',');
  useEffect(() => {
    let alive = true;
    if (active === null) {
      setSheet(null);
      return;
    }
    (multi ? Queries.propertiesMulti(selection) : Queries.properties(active))
      .then((s) => alive && setSheet(s))
      .catch(() => alive && setSheet(null));
    return () => {
      alive = false;
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, revision, selKey]);

  useEffect(() => {
    let alive = true;
    if (active === null || sheet?.kind !== 'PANEL' || propertiesTab !== 'machining') return setFlat(null);
    Queries.manufacturing(active)
      .then((f) => alive && setFlat(f))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [active, sheet, propertiesTab, revision]);

  if (!sheet) {
    if (embedded) return <div className="empty">{selection.length > 1 ? `${selection.length} đối tượng được chọn` : 'Click một tấm (click lần 2 vào tủ) để chỉnh.'}</div>;
    return (
      <div className="panel properties">
        <div className="panel-header">
          <Icon name="settings" size={16} />
          <span>Thuộc tính</span>
        </div>
        <div className="empty">{selection.length > 1 ? `${selection.length} đối tượng được chọn` : 'Chọn một đối tượng để xem thuộc tính.'}</div>
      </div>
    );
  }

  const matField = sheet.groups.flatMap((g) => g.fields).find((f) => f.key === 'material' || f.key === 'carcass_material');
  const matOpt = matField?.options.find((o) => o.value === matField.value);
  const groups = sheet.groups.filter((g) => TAB_GROUPS[propertiesTab].includes(g.key));

  return (
    <div className={embedded ? 'embedded-props' : 'panel properties'}>
      {!embedded && (
        <div className="panel-header">
          <Icon name="settings" size={16} />
          <span>Thuộc tính</span>
          {selection.length > 1 && <span className="badge">{selection.length}</span>}
        </div>
      )}
      <div className="prop-head">
        <div className="swatch" style={{ background: matOpt?.color ?? '#dee2e6' }} />
        <div>
          <div className="prop-name">{sheet.name}</div>
          <div className="prop-sub">
            {KIND_LABEL[sheet.kind]}
            {matOpt ? ` · ${matOpt.label}` : ''}
            {sheet.locked && (
              <>
                {' · '}
                <Icon name="lock" size={12} /> Đã khóa
              </>
            )}
          </div>
        </div>
      </div>
      <div className="tabs">
        {(
          [
            ['params', 'Tham số'],
            ['material', 'Vật liệu'],
            ['machining', 'Gia công'],
          ] as const
        ).map(([k, l]) => (
          <button key={k} className={propertiesTab === k ? 'active' : ''} onClick={() => set({ propertiesTab: k })}>
            {l}
          </button>
        ))}
      </div>
      <div className="prop-body">
        {groups.map((g) => (
          <section key={g.key} className="prop-group">
            <h4>{GROUP_LABEL[g.key] ?? g.title}</h4>
            {g.fields.map((f) => (
              <FieldRow key={f.key} id={sheet.id} ids={sheet.ids} f={f} locked={sheet.locked} />
            ))}
          </section>
        ))}
        {propertiesTab === 'machining' && flat && <FeatureList flat={flat} />}
        {propertiesTab === 'machining' && sheet.kind === 'PANEL' && (
          <button className="btn primary wide" onClick={() => set({ workspace: 'manufacturing', mfgPanel: sheet.id })}>
            <Icon name="drill" size={16} /> Mở trong Gia công
          </button>
        )}
        {propertiesTab === 'machining' && sheet.kind !== 'PANEL' && <div className="empty">Gia công được định nghĩa trên từng tấm.</div>}
        {propertiesTab === 'params' && sheet.bounds && (
          <div className="prop-foot">
            Bao: {fmt(sheet.bounds.max[0] - sheet.bounds.min[0], 0)} × {fmt(sheet.bounds.max[1] - sheet.bounds.min[1], 0)} × {fmt(sheet.bounds.max[2] - sheet.bounds.min[2], 0)} mm
          </div>
        )}
      </div>
    </div>
  );
}

function FeatureList({ flat }: { flat: FlatPanel }) {
  const counts = new Map<string, number>();
  for (const f of flat.features) {
    const k = f.feature.type === 'DRILL' || f.feature.type === 'EDGE_DRILL' ? `${f.feature.type === 'EDGE_DRILL' ? 'Cạnh · ' : ''}${PURPOSE_LABEL[f.feature.purpose]} Ø${f.feature.diameter}` : f.feature.type;
    counts.set(k, (counts.get(k) ?? 0) + 1);
  }
  return (
    <section className="prop-group">
      <h4>Danh sách gia công</h4>
      {Array.from(counts).map(([k, n]) => (
        <div className="prop-row" key={k}>
          <label>{k}</label>
          <span className="ro">× {n}</span>
        </div>
      ))}
      {counts.size === 0 && <div className="empty small">Chưa có gia công.</div>}
    </section>
  );
}

function FieldRow({ id, ids, f, locked }: { id: number; ids?: number[]; f: PropertyField; locked: boolean }) {
  const label = FIELD_LABEL[f.key] ?? f.label;
  const editable = f.editable && !locked;
  const commit = (v: string) => (ids ? Commands.setParameterMulti(ids, f.key, v) : Commands.setParameter(id, f.key, v)).catch(() => undefined);
  if (f.kind === 'anchors') return <AnchorsField id={id} f={f} editable={editable && !ids} />;
  if (f.kind === 'list') {
    const items = (Array.isArray(f.value) ? f.value : []) as (string | { id: number; name: string })[];
    return (
      <div className="prop-list">
        <div className="prop-list-head">
          <span>
            {label} ({items.length})
          </span>
          {f.key === 'tools' && items.length > 0 && editable && (
            <button className="icon-btn danger" title="Gỡ tất cả tool" onClick={() => void Commands.setPartMod(id, { clear_tools: true }).catch(() => undefined)}>
              <Icon name="x" size={14} />
            </button>
          )}
        </div>
        {items.map((it, i) => (
          <div key={i} className="prop-list-item" onClick={() => typeof it === 'object' && useUi.getState().select([it.id])}>
            {typeof it === 'object' ? it.name : it}
          </div>
        ))}
      </div>
    );
  }
  if (f.kind === 'readonly' || (!editable && f.kind !== 'number')) {
    let v = String(f.value ?? '–');
    if (f.key === 'role' || f.key === 'kind') v = roleLabel(v);
    return (
      <div className="prop-row">
        <label>{label}</label>
        <span className="ro">{typeof f.value === 'number' ? fmt(f.value) : v}</span>
      </div>
    );
  }
  if (f.kind === 'bool') {
    return (
      <div className="prop-row">
        <label>{label}</label>
        <label className="switch">
          <input type="checkbox" checked={Boolean(f.value)} ref={(el) => el && (el.indeterminate = !!f.mixed)} disabled={!editable} onChange={(e) => void commit(e.target.checked ? 'on' : 'off')} />
          <span />
        </label>
      </div>
    );
  }
  if (f.kind === 'select') {
    return (
      <div className="prop-row">
        <label>{label}</label>
        <div className="select-wrap">
          {f.options.find((o) => o.value === f.value)?.color && <i className="dot" style={{ background: f.options.find((o) => o.value === f.value)!.color }} />}
          <select value={f.mixed ? '' : String(f.value)} disabled={!editable} onChange={(e) => e.target.value !== '' && void commit(e.target.value)}>
            {f.mixed && <option value="">— Nhiều giá trị —</option>}
            {f.options.map((o) => (
              <option key={o.value} value={o.value}>
                {OPTION_LABEL[o.value] ?? OPTION_LABEL[o.label] ?? o.label}
              </option>
            ))}
          </select>
        </div>
      </div>
    );
  }
  if (f.kind === 'text') return <TextRow label={label} value={f.mixed ? '' : String(f.value ?? '')} placeholder={f.mixed ? 'Nhiều giá trị' : undefined} onCommit={commit} disabled={!editable} />;
  return <NumberRow label={label} f={f} onCommit={commit} disabled={!editable} />;
}

const EDGE_VI: Record<string, string> = { LEFT: 'Trái', RIGHT: 'Phải', BOTTOM: 'Dưới', TOP: 'Trên' };

/** Ràng buộc động: edge → face of another part (+ offset), one per edge. */
function AnchorsField({ id, f, editable }: { id: number; f: PropertyField; editable: boolean }) {
  type A = { edge: string; target: number | null; target_name: string; face: 'INNER' | 'OUTER'; offset: number };
  const items = (Array.isArray(f.value) ? f.value : []) as A[];
  const [edge, setEdge] = useState<'LEFT' | 'RIGHT' | 'BOTTOM' | 'TOP'>('RIGHT');
  const [target, setTarget] = useState('');
  const [face, setFace] = useState<'INNER' | 'OUTER'>('INNER');
  const [offset, setOffset] = useState('0');
  return (
    <div className="prop-list anchors">
      {items.map((a, i) => (
        <div key={i} className="prop-list-item anchor-row">
          <span>
            {EDGE_VI[a.edge] ?? a.edge} → <b>{a.target_name}</b> · {a.face === 'INNER' ? 'mặt trong' : 'mặt ngoài'} · {fmt(a.offset)} mm
          </span>
          {editable && (
            <button className="icon-btn danger" title="Bỏ ràng buộc" onClick={() => void Commands.setPartMod(id, { remove_anchor: i }).catch(() => undefined)}>
              <Icon name="x" size={12} />
            </button>
          )}
        </div>
      ))}
      {items.length === 0 && <div className="muted small">Chưa có. Kệ/vách mặc định bám theo khoang.</div>}
      {editable && (
        <div className="anchor-add">
          <select className="field" value={edge} onChange={(e) => setEdge(e.target.value as typeof edge)} title="Cạnh của tấm này">
            {Object.entries(EDGE_VI).map(([k, v]) => (
              <option key={k} value={k}>
                Cạnh {v.toLowerCase()}
              </option>
            ))}
          </select>
          <select className="field" value={target} onChange={(e) => setTarget(e.target.value)} title="Tấm đích">
            <option value="">→ Tấm đích…</option>
            {f.options.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <select className="field" value={face} onChange={(e) => setFace(e.target.value as typeof face)}>
            <option value="INNER">Mặt trong</option>
            <option value="OUTER">Mặt ngoài</option>
          </select>
          <input className="field" value={offset} onChange={(e) => setOffset(e.target.value)} title="Offset (mm)" />
          <button
            className="btn primary"
            disabled={!target}
            onClick={() => void Commands.setPartMod(id, { add_anchor: { edge, target: Number(target), face, offset: Number(offset.replace(',', '.')) || 0 } }).catch(() => undefined)}
          >
            Thêm
          </button>
        </div>
      )}
    </div>
  );
}

export function lockDot(f: PropertyField) {
  return f.locked ? <i className="lock-dot" title="Tham số đang khóa" /> : null;
}

function TextRow({ label, value, onCommit, disabled, placeholder }: { label: string; value: string; onCommit: (v: string) => void; disabled: boolean; placeholder?: string }) {
  const [v, setV] = useState(value);
  useEffect(() => setV(value), [value]);
  return (
    <div className="prop-row">
      <label>{label}</label>
      <input
        className="field"
        value={v}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(e) => setV(e.target.value)}
        onBlur={() => v !== value && onCommit(v)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
          if (e.key === 'Escape') setV(value);
          e.stopPropagation();
        }}
      />
    </div>
  );
}

/** Numeric field accepting plain numbers or expressions (`= cabinet.inner_width - 2`). */
export function NumberRow({ label, f, onCommit, disabled }: { label: string; f: PropertyField; onCommit: (v: string) => void; disabled: boolean }) {
  const shown = typeof f.value === 'number' ? String(Math.round(f.value * 1000) / 1000) : '';
  const [text, setText] = useState(shown);
  const [focus, setFocus] = useState(false);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (!focus) setText(shown);
  }, [shown, focus]);
  const isExpr = f.expression && f.source;
  const step = (dir: number, big: boolean) => {
    if (f.mixed) return;
    const v = (typeof f.value === 'number' ? f.value : 0) + dir * (big ? 10 : 1);
    onCommit(String(Math.round(v * 1000) / 1000));
  };
  return (
    <div className={`prop-row ${f.error ? 'error' : ''}`} title={isExpr ? `Công thức: ${f.source}` : undefined}>
      <label className={f.locked ? 'locked' : ''}>
        {f.locked && <i className="lock-dot" />}
        {label}
        {isExpr && <span className="fx">ƒx</span>}
      </label>
      <div className={`num ${isExpr ? 'expr' : ''}`}>
        <input
          ref={ref}
          placeholder={f.mixed ? 'Nhiều giá trị' : undefined}
          value={focus ? text : shown}
          disabled={disabled}
          onFocus={() => {
            setFocus(true);
            setText(isExpr ? `= ${String(f.source).replace(/^=\s*/, '')}` : shown);
            setTimeout(() => ref.current?.select(), 0);
          }}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => {
            setFocus(false);
            const orig = isExpr ? `= ${String(f.source).replace(/^=\s*/, '')}` : shown;
            if (text.trim() !== orig.trim() && text.trim() !== '') onCommit(text.replace(',', '.'));
          }}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') ref.current?.blur();
            if (e.key === 'Escape') {
              setText(shown);
              setFocus(false);
              ref.current?.blur();
            }
            if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
              e.preventDefault();
              step(e.key === 'ArrowUp' ? 1 : -1, e.shiftKey);
            }
          }}
        />
        {f.unit && <em>{f.unit === 'deg' ? '°' : 'mm'}</em>}
        {!disabled && !f.mixed && (
          <span className="spin">
            <button tabIndex={-1} onClick={(e) => step(1, e.shiftKey)}>
              <Icon name="chevronDown" size={10} style={{ transform: 'rotate(180deg)' }} />
            </button>
            <button tabIndex={-1} onClick={(e) => step(-1, e.shiftKey)}>
              <Icon name="chevronDown" size={10} />
            </button>
          </span>
        )}
      </div>
    </div>
  );
}
