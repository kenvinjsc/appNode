// Small form building blocks shared by the cabinet designer and tool panels.
import { useEffect, useState, type ReactNode } from 'react';
import { Icon } from './icons';

export function QuickNums({ value, onChange, min = 1, max = 9 }: { value: number; onChange: (n: number) => void; min?: number; max?: number }) {
  return (
    <div className="quick-nums">
      {Array.from({ length: max - min + 1 }, (_, i) => i + min).map((n) => (
        <button key={n} className={value === n ? 'on' : ''} onClick={() => onChange(n)} type="button">
          {n}
        </button>
      ))}
    </div>
  );
}

export function Stepper({ value, onChange, min = 1, max = 50 }: { value: number; onChange: (n: number) => void; min?: number; max?: number }) {
  return (
    <div className="stepper">
      <button type="button" onClick={() => onChange(Math.max(min, value - 1))}>
        <Icon name="minus" size={14} />
      </button>
      <input value={value} onChange={(e) => onChange(Math.max(min, Math.min(max, Number(e.target.value) || min)))} inputMode="numeric" />
      <button type="button" onClick={() => onChange(Math.min(max, value + 1))}>
        <Icon name="plus" size={14} />
      </button>
    </div>
  );
}

export function Radio<T extends string>({ options, value, onChange, disabled }: { options: [T, string, boolean?][]; value: T; onChange: (v: T) => void; disabled?: boolean }) {
  return (
    <div className="radio-list">
      {options.map(([v, l, off]) => (
        <button key={v} type="button" disabled={disabled || off} className={value === v ? 'on' : ''} onClick={() => onChange(v)}>
          <i />
          {l}
        </button>
      ))}
    </div>
  );
}

export function Seg<T extends string>({ options, value, onChange }: { options: [T, string][]; value: T; onChange: (v: T) => void }) {
  return (
    <div className="seg wide">
      {options.map(([v, l]) => (
        <button key={v} type="button" className={value === v ? 'active' : ''} onClick={() => onChange(v)}>
          {l}
        </button>
      ))}
    </div>
  );
}

export function Fieldset({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="fs">
      <legend>{title}</legend>
      {children}
    </fieldset>
  );
}

export function Collapse({ title, children, defaultOpen = false, extra }: { title: ReactNode; children: ReactNode; defaultOpen?: boolean; extra?: ReactNode }) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className={`collapse ${open ? 'open' : ''}`}>
      <div className="collapse-head">
        <button type="button" onClick={() => setOpen(!open)}>
          <Icon name={open ? 'chevronDown' : 'chevronRight'} size={12} />
          <span>{title}</span>
        </button>
        {extra}
      </div>
      {open && <div className="collapse-body">{children}</div>}
    </div>
  );
}

/** Numeric input that commits on Enter/blur and accepts "," decimals. */
export function Num({ value, onCommit, disabled, unit = 'mm', placeholder }: { value: number | ''; onCommit: (v: number) => void; disabled?: boolean; unit?: string; placeholder?: string }) {
  const [t, setT] = useState(value === '' ? '' : String(value));
  useEffect(() => setT(value === '' ? '' : String(value)), [value]);
  const commit = () => {
    const v = Number(t.replace(',', '.'));
    if (t.trim() !== '' && Number.isFinite(v) && v !== value) onCommit(v);
  };
  return (
    <div className="num small">
      <input
        value={t}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(e) => setT(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
        }}
        inputMode="decimal"
      />
      {unit && <em>{unit}</em>}
    </div>
  );
}

/** Row of a 3-way position (Tỷ lệ / Cách A / Cách B) with a red "locked" dot. */
export function LockRow({ label, locked, value, onLock, onCommit, unit }: { label: string; locked: boolean; value: number | ''; onLock: () => void; onCommit: (v: number) => void; unit: string }) {
  return (
    <div className={`lock-row ${locked ? 'locked' : ''}`}>
      <button type="button" className="lock-label" onClick={onLock} title="Bấm để khóa tham số này">
        <i />
        {label}
      </button>
      <Num value={value} onCommit={onCommit} unit={unit} />
    </div>
  );
}

export function Steps({ steps }: { steps: { label: string; done: boolean; detail?: string }[] }) {
  return (
    <div className="steps">
      {steps.map((s, i) => (
        <div key={i} className={s.done ? 'done' : 'todo'}>
          Bước {i + 1}: {s.label} — {s.done ? s.detail ?? 'Đã chọn' : 'Chưa chọn'}
        </div>
      ))}
    </div>
  );
}
