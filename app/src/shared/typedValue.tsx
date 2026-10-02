// Nhập số khi kéo: while a drag is active, digits typed on the keyboard build an
// exact value (like SketchUp's measurement box); after the drag an input box opens
// at the drop point, pre-filled, so the final value can be typed exactly.
import { useEffect, useRef } from 'react';

/** Listen for typed digits during a drag; returns the cleanup. */
export function listenTyped(onChange: (text: string) => void): () => void {
  let buf = '';
  const kd = (e: KeyboardEvent) => {
    if (/^[0-9.,-]$/.test(e.key)) buf += e.key === ',' ? '.' : e.key;
    else if (e.key === 'Backspace') buf = buf.slice(0, -1);
    else return;
    e.preventDefault();
    e.stopPropagation();
    onChange(buf);
  };
  window.addEventListener('keydown', kd, true);
  return () => window.removeEventListener('keydown', kd, true);
}

/** Parse a typed value ("", "12.5", "1,5") — null when not a number. */
export function typedNumber(t: string): number | null {
  if (!t.trim()) return null;
  const v = Number(t.replace(',', '.'));
  return Number.isFinite(v) ? v : null;
}

/**
 * Input box inside an SVG (foreignObject) at (x, y). Enter / blur → onDone(value);
 * Escape → onDone(null). `w`, `h` and `fontSize` are in the SVG's units.
 */
export function ValueBox({ x, y, value, onDone, w = 100, h = 28, fontSize = 13, suffix = 'mm' }: {
  x: number;
  y: number;
  value: number;
  onDone: (v: number | null) => void;
  w?: number;
  h?: number;
  fontSize?: number;
  suffix?: string;
}) {
  const closed = useRef(false);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    closed.current = false;
    // Focus after the pointer-up that opened the box has finished.
    const t = setTimeout(() => {
      ref.current?.focus();
      ref.current?.select();
    }, 0);
    return () => clearTimeout(t);
  }, [x, y, value]);
  const done = (v: number | null) => {
    if (closed.current) return;
    closed.current = true;
    onDone(v);
  };
  return (
    <foreignObject x={x - w / 2} y={y - h / 2} width={w} height={h} style={{ pointerEvents: 'all', overflow: 'visible' }}>
      <div className="value-box" style={{ fontSize }} title={`Nhập giá trị chính xác (${suffix}) rồi Enter · Esc để hủy`}>
        <input
          ref={ref}
          defaultValue={String(Math.round(value * 10) / 10)}
          onPointerDown={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === 'Enter') done(typedNumber(e.currentTarget.value));
            if (e.key === 'Escape') done(null);
          }}
          onBlur={(e) => done(typedNumber(e.currentTarget.value))}
        />
        <em>{suffix}</em>
      </div>
    </foreignObject>
  );
}
