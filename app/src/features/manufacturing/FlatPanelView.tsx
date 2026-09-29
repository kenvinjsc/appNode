// Flattened panel in its local manufacturing frame (X right, Y up), side A or B.
import { useState } from 'react';
import type { FlatPanel, MachiningFeature } from '../../core-api/types';
import { EDGE_LABEL, PURPOSE_LABEL, fmt } from '../../shared/i18n';

const PURPOSE_COLOR: Record<string, string> = {
  DOWEL: '#1c7ed6',
  SHELF_PIN: '#2f9e44',
  HINGE_CUP: '#e8590c',
  CAM_LOCK: '#ae3ec9',
  GENERIC: '#495057',
  CONNECTOR: '#ae3ec9',
  HINGE_SCREW: '#e8590c',
  HANDLE: '#868e96',
};

export function FlatPanelView({ flat, side, highlight, onHover }: { flat: FlatPanel; side: 'A' | 'B'; highlight: number | null; onHover: (i: number | null) => void }) {
  const W = flat.width;
  const H = flat.height;
  const u = Math.max(W, H) / 70;
  const pad = u * 9;
  // Local → SVG (mirror X when looking at side B).
  const X = (x: number) => (side === 'A' ? x : W - x);
  const Y = (y: number) => H - y;
  const [tip, setTip] = useState<{ i: number; x: number; y: number } | null>(null);

  const drills = flat.features
    .map((f, i) => ({ f: f.feature, i }))
    .filter((d): d is { f: Extract<MachiningFeature, { type: 'DRILL' }>; i: number } => d.f.type === 'DRILL');
  // Ordinate ticks, thinned so labels do not overlap.
  const thin = (vals: number[]) => {
    const out: number[] = [];
    for (const v of Array.from(new Set(vals.map((x) => Math.round(x * 10) / 10))).sort((a, b) => a - b)) {
      if (!out.length || v - out[out.length - 1] > u * 3.2) out.push(v);
    }
    return out.slice(0, 16);
  };
  const xs = thin(drills.map((d) => d.f.x));
  const ys = thin(drills.map((d) => d.f.y));

  const featureEl = (f: MachiningFeature, i: number) => {
    const hl = highlight === i;
    const common = {
      onMouseEnter: (e: React.MouseEvent) => {
        onHover(i);
        setTip({ i, x: e.clientX, y: e.clientY });
      },
      onMouseLeave: () => {
        onHover(null);
        setTip(null);
      },
      className: `feat ${hl ? 'hl' : ''}`,
    };
    switch (f.type) {
      case 'DRILL': {
        const other = f.side !== side;
        const c = PURPOSE_COLOR[f.purpose] ?? '#495057';
        return (
          <g key={i} {...common}>
            <circle cx={X(f.x)} cy={Y(f.y)} r={Math.max(f.diameter / 2, u * 0.8)} fill={other ? 'none' : c} fillOpacity={0.25} stroke={c} strokeWidth={u * 0.18} strokeDasharray={other ? `${u * 0.6} ${u * 0.4}` : undefined} />
            <line x1={X(f.x) - u * 0.6} y1={Y(f.y)} x2={X(f.x) + u * 0.6} y2={Y(f.y)} stroke={c} strokeWidth={u * 0.1} />
            <line x1={X(f.x)} y1={Y(f.y) - u * 0.6} x2={X(f.x)} y2={Y(f.y) + u * 0.6} stroke={c} strokeWidth={u * 0.1} />
          </g>
        );
      }
      case 'EDGE_DRILL': {
        const c = PURPOSE_COLOR[f.purpose] ?? '#495057';
        const r = f.diameter / 2;
        let x = 0, y = 0, w = 0, h = 0;
        if (f.edge === 'LEFT') [x, y, w, h] = [0, f.offset - r, f.depth, f.diameter];
        if (f.edge === 'RIGHT') [x, y, w, h] = [W - f.depth, f.offset - r, f.depth, f.diameter];
        if (f.edge === 'BOTTOM') [x, y, w, h] = [f.offset - r, 0, f.diameter, f.depth];
        if (f.edge === 'TOP') [x, y, w, h] = [f.offset - r, H - f.depth, f.diameter, f.depth];
        const sx = Math.min(X(x), X(x + w));
        return (
          <g key={i} {...common}>
            <rect x={sx} y={Y(y + h)} width={w} height={h} fill={c} fillOpacity={0.12} stroke={c} strokeWidth={u * 0.14} strokeDasharray={`${u * 0.5} ${u * 0.35}`} />
          </g>
        );
      }
      case 'POCKET': {
        const sx = Math.min(X(f.x), X(f.x + f.width));
        return (
          <g key={i} {...common}>
            <rect x={sx} y={Y(f.y + f.height)} width={f.width} height={f.height} fill="url(#hatch)" stroke="#5f3dc4" strokeWidth={u * 0.16} strokeDasharray={f.side !== side ? `${u * 0.6} ${u * 0.4}` : undefined} />
          </g>
        );
      }
      case 'GROOVE': {
        const [w, h] = f.direction === 'X' ? [f.length, f.width] : [f.width, f.length];
        const sx = Math.min(X(f.x), X(f.x + w));
        return (
          <g key={i} {...common}>
            <rect x={sx} y={Y(f.y + h)} width={w} height={h} fill="#0c8599" fillOpacity={0.25} stroke="#0c8599" strokeWidth={u * 0.16} strokeDasharray={f.side !== side ? `${u * 0.6} ${u * 0.4}` : undefined} />
          </g>
        );
      }
      case 'CONTOUR':
        return (
          <g key={i} {...common}>
            <polygon points={f.polygon.points.map((p) => `${X(p.x)},${Y(p.y)}`).join(' ')} fill={f.inner ? '#fff' : 'none'} stroke="#343a40" strokeWidth={u * 0.2} />
          </g>
        );
    }
  };

  const band = (edge: string) => flat.edge_bands.some((b) => b.edge === edge);
  const edgeLine = (edge: 'LEFT' | 'RIGHT' | 'TOP' | 'BOTTOM') => {
    const pts = { LEFT: [0, 0, 0, H], RIGHT: [W, 0, W, H], BOTTOM: [0, 0, W, 0], TOP: [0, H, W, H] }[edge];
    return <line key={edge} x1={X(pts[0])} y1={Y(pts[1])} x2={X(pts[2])} y2={Y(pts[3])} stroke="#e8590c" strokeWidth={u * 0.8} strokeLinecap="square" />;
  };

  return (
    <div className="flat-view">
      <svg viewBox={`${-pad} ${-pad} ${W + 2 * pad} ${H + 2 * pad}`} preserveAspectRatio="xMidYMid meet">
        <defs>
          <pattern id="hatch" width={u * 1.2} height={u * 1.2} patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
            <line x1="0" y1="0" x2="0" y2={u * 1.2} stroke="#5f3dc4" strokeWidth={u * 0.14} strokeOpacity={0.6} />
          </pattern>
          <marker id="fa" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse">
            <path d="M0 1 L9 5 L0 9 z" fill="#343a40" />
          </marker>
        </defs>
        <polygon className="flat-outer" points={flat.outer.points.map((p) => `${X(p.x)},${Y(p.y)}`).join(' ')} strokeWidth={u * 0.2} />
        {/* grain */}
        {flat.grain !== 'NONE' && (
          <g opacity={0.35} stroke="#8d6e63" strokeWidth={u * 0.12}>
            {Array.from({ length: 7 }, (_, k) =>
              flat.grain === 'ALONG_HEIGHT' ? (
                <line key={k} x1={(W * (k + 1)) / 8} y1={u * 2} x2={(W * (k + 1)) / 8} y2={H - u * 2} />
              ) : (
                <line key={k} x1={u * 2} y1={(H * (k + 1)) / 8} x2={W - u * 2} y2={(H * (k + 1)) / 8} />
              ),
            )}
          </g>
        )}
        {(['LEFT', 'RIGHT', 'TOP', 'BOTTOM'] as const).filter(band).map(edgeLine)}
        {flat.features.map((f, i) => featureEl(f.feature, i))}
        {/* overall dimensions */}
        <g className="fdim" fontSize={u * 1.5}>
          <line x1={0} y1={H + u * 4} x2={W} y2={H + u * 4} markerStart="url(#fa)" markerEnd="url(#fa)" />
          <text x={W / 2} y={H + u * 6.4} textAnchor="middle">{fmt(W, 1)}</text>
          <line x1={-u * 4} y1={0} x2={-u * 4} y2={H} markerStart="url(#fa)" markerEnd="url(#fa)" />
          <text transform={`translate(${-u * 5} ${H / 2}) rotate(-90)`} textAnchor="middle">{fmt(H, 1)}</text>
          {/* ordinate ticks for drill positions */}
          {xs.map((x) => (
            <g key={`x${x}`}>
              <line x1={X(x)} y1={-u * 1} x2={X(x)} y2={-u * 2.2} />
              <text x={X(x)} y={-u * 2.8} textAnchor="middle" fontSize={u * 1.05}>{fmt(x, 1)}</text>
            </g>
          ))}
          {ys.map((y) => (
            <g key={`y${y}`}>
              <line x1={W + u} y1={Y(y)} x2={W + u * 2.2} y2={Y(y)} />
              <text x={W + u * 2.6} y={Y(y) + u * 0.4} fontSize={u * 1.05}>{fmt(y, 1)}</text>
            </g>
          ))}
        </g>
        <text x={0} y={-u * 5.5} fontSize={u * 1.6} className="side-tag">
          Mặt {side} {side === 'B' ? '(lật)' : ''} · gốc (0,0) {side === 'A' ? 'dưới trái' : 'dưới phải'}
        </text>
      </svg>
      {tip && flat.features[tip.i] && (
        <div className="feat-tip" style={{ left: tip.x + 12, top: tip.y + 12 }}>
          {describe(flat.features[tip.i].feature)}
        </div>
      )}
    </div>
  );
}

export function describe(f: MachiningFeature): string {
  switch (f.type) {
    case 'DRILL':
      return `${PURPOSE_LABEL[f.purpose]} Ø${f.diameter} sâu ${f.depth} · (${fmt(f.x)}, ${fmt(f.y)}) · mặt ${f.side}`;
    case 'EDGE_DRILL':
      return `Khoan cạnh ${EDGE_LABEL[f.edge]} Ø${f.diameter} sâu ${f.depth} · tại ${fmt(f.offset)}`;
    case 'POCKET':
      return `Hốc ${fmt(f.width)}×${fmt(f.height)} sâu ${f.depth} · mặt ${f.side}`;
    case 'GROOVE':
      return `Rãnh ${fmt(f.length)}×${fmt(f.width)} sâu ${f.depth} · mặt ${f.side}`;
    case 'CONTOUR':
      return `Đường bao ${f.inner ? 'trong' : 'ngoài'}`;
  }
}
