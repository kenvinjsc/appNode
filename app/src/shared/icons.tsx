// Consistent 24px stroke icon set (no emoji, no icon fonts).
import type { CSSProperties } from 'react';

const P: Record<string, JSX.Element> = {
  logo: (<><path d="M12 2.5 20.5 7.3v9.4L12 21.5 3.5 16.7V7.3z" fill="currentColor" stroke="none" /><path d="M12 12 20.5 7.3M12 12 3.5 7.3M12 12v9.5" stroke="#fff" /></>),
  new: (<><path d="M6 3h8l4 4v14H6z" /><path d="M14 3v4h4" /><path d="M12 11v6M9 14h6" /></>),
  open: (<><path d="M3 7h6l2 2h10v10H3z" /><path d="M3 7V5h6l2 2" /></>),
  edit: (<><path d="M4 20h4L19 9l-4-4L4 16z" /><path d="M13 7l4 4" /></>),
  mirror: (<><path d="M12 3v18" strokeDasharray="2 2" /><path d="M9 6L3 18h6z" /><path d="M15 6l6 12h-6z" /></>),
  save: (<><path d="M5 4h11l3 3v13H5z" /><path d="M8 4v5h7V4" /><rect x="8" y="13" width="8" height="5" /></>),
  saveAs: (<><path d="M5 4h11l3 3v6" /><path d="M5 4v16h7" /><path d="M8 4v5h7V4" /><path d="M15 21l1-3 4-4 2 2-4 4z" /></>),
  undo: (<><path d="M9 7 5 11l4 4" /><path d="M5 11h9a5 5 0 0 1 0 10h-2" /></>),
  redo: (<><path d="m15 7 4 4-4 4" /><path d="M19 11h-9a5 5 0 0 0 0 10h2" /></>),
  select: (<path d="M6 3v16l4.5-4.5 3 6.5 2.5-1.2-3-6.3H19z" />),
  move: (<><path d="M12 3v18M3 12h18" /><path d="m9 6 3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3" /></>),
  rotate: (<><path d="M20 12a8 8 0 1 1-2.3-5.6" /><path d="M20 4v5h-5" /></>),
  duplicate: (<><rect x="8" y="8" width="12" height="12" rx="1" /><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3" /></>),
  trash: (<><path d="M4 7h16M10 7V4h4v3M6 7l1 13h10l1-13" /><path d="M10 11v6M14 11v6" /></>),
  align: (<><path d="M4 3v18" /><rect x="7" y="6" width="10" height="4" /><rect x="7" y="14" width="14" height="4" /></>),
  room: (<><path d="M3 20V8l9-5 9 5v12" /><path d="M3 20h18" /><path d="M9 20v-6h6v6" /></>),
  cabinet: (<><rect x="4" y="3" width="16" height="18" rx="1" /><path d="M12 3v18M10 11v2M14 11v2" /></>),
  panel: (<><path d="M5 5h12l2 2v12H7l-2-2z" /><path d="M5 5v12M17 5v12M7 19V7h12" /></>),
  shelf: (<><path d="M4 3v18M20 3v18" /><path d="M4 9h16M4 15h16" /></>),
  door: (<><rect x="5" y="3" width="14" height="18" rx="1" /><path d="M15 11v2" /></>),
  drawer: (<><rect x="4" y="3" width="16" height="18" rx="1" /><path d="M4 9h16M4 15h16M10 6h4M10 12h4M10 18h4" /></>),
  divider: (<><rect x="4" y="3" width="16" height="18" rx="1" /><path d="M12 3v18" /></>),
  hardware: (<><circle cx="8" cy="8" r="3" /><path d="m10 10 9 9M15 19l4-4" /></>),
  view: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M12 12 20 7.5M12 12 4 7.5M12 12v9" /></>),
  front: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M4 7.5 12 12v9l-8-4.5z" fill="currentColor" fillOpacity=".25" /></>),
  top: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M12 3 20 7.5 12 12 4 7.5z" fill="currentColor" fillOpacity=".25" /></>),
  side: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M20 7.5 12 12v9l8-4.5z" fill="currentColor" fillOpacity=".25" /></>),
  perspective: (<><path d="M4 6h16l-3 12H7z" /><path d="M12 6v12M5.5 12h13" /></>),
  fit: (<><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /><rect x="8" y="8" width="8" height="8" /></>),
  section: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M2 13h20" strokeDasharray="2 2" /></>),
  drill: (<><circle cx="12" cy="12" r="7" /><circle cx="12" cy="12" r="2.5" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3" /></>),
  pocket: (<><rect x="4" y="4" width="16" height="16" /><rect x="8" y="8" width="8" height="8" fill="currentColor" fillOpacity=".25" /></>),
  groove: (<><rect x="3" y="5" width="18" height="14" /><path d="M3 12h18" strokeWidth="3" strokeOpacity=".4" /></>),
  edgeband: (<><rect x="4" y="6" width="16" height="12" /><path d="M4 6h16" strokeWidth="3.2" /></>),
  inspect: (<><circle cx="10" cy="10" r="6" /><path d="m14.5 14.5 6 6" /><path d="M8 10h4M10 8v4" /></>),
  nesting: (<><rect x="3" y="4" width="18" height="16" /><path d="M3 11h9V4M12 14h9M8 11v9M16 14v6" /></>),
  cnc: (<><path d="M4 20h16" /><path d="M7 20V9h10v11" /><path d="M12 9V4M10 4h4" /><path d="M12 13v3" /></>),
  relations: (<><circle cx="6" cy="6" r="2.5" /><circle cx="18" cy="8" r="2.5" /><circle cx="10" cy="18" r="2.5" /><path d="M8.3 6.6 15.6 7.6M7.2 8.2 9.2 15.6M16.4 10 11.8 16.2" /></>),
  eye: (<><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" /></>),
  eyeOff: (<><path d="M3 3l18 18" /><path d="M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.1M6.6 6.6C3.8 8.4 2 12 2 12s3.5 7 10 7a9.6 9.6 0 0 0 5.3-1.6" /><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" /></>),
  lock: (<><rect x="5" y="11" width="14" height="10" rx="1" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></>),
  unlock: (<><rect x="5" y="11" width="14" height="10" rx="1" /><path d="M8 11V7a4 4 0 0 1 7.7-1.5" /></>),
  chevronRight: (<path d="m9 6 6 6-6 6" />),
  chevronDown: (<path d="m6 9 6 6 6-6" />),
  search: (<><circle cx="11" cy="11" r="6.5" /><path d="m16 16 5 5" /></>),
  play: (<path d="M7 4v16l13-8z" />),
  pause: (<><path d="M8 4v16M16 4v16" strokeWidth="3" /></>),
  step: (<><path d="M5 4v16l10-8z" /><path d="M19 4v16" /></>),
  stop: (<rect x="6" y="6" width="12" height="12" />),
  settings: (<><circle cx="12" cy="12" r="3" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9 7 7M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" /></>),
  grid: (<><path d="M3 3h18v18H3zM3 9h18M3 15h18M9 3v18M15 3v18" /></>),
  layers: (<><path d="m12 3 9 5-9 5-9-5z" /><path d="m3 13 9 5 9-5" /></>),
  library: (<><path d="M4 4h4v16H4zM10 4h4v16h-4z" /><path d="m16 5 3.5-1 3 15.5-3.6.9z" /></>),
  material: (<><path d="M4 5h16v14H4z" /><path d="M4 9c4 1 6-2 10-1s5 2 6 1M4 14c3 1 6-2 10-1s5 2 6 1" /></>),
  report: (<><path d="M6 3h9l4 4v14H6z" /><path d="M9 11h7M9 15h7M9 7h3" /></>),
  x: (<path d="M6 6l12 12M18 6 6 18" />),
  check: (<path d="m5 12 5 5 9-10" />),
  plus: (<path d="M12 5v14M5 12h14" />),
  minus: (<path d="M5 12h14" />),
  box3d: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M12 12 20 7.5M12 12 4 7.5M12 12v9" /></>),
  drawing: (<><rect x="3" y="3" width="18" height="18" /><path d="M7 17V7h10v10M12 7v10M7 12h5" /></>),
  download: (<><path d="M12 4v11M7 10l5 5 5-5" /><path d="M5 20h14" /></>),
  upload: (<><path d="M12 16V5M7 10l5-5 5 5" /><path d="M5 20h14" /></>),
  warning: (<><path d="M12 3 2 20h20z" /><path d="M12 10v4M12 17v.5" /></>),
  info: (<><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7.5v.5" /></>),
  dimension: (<><path d="M4 7v10M20 7v10M4 12h16" /><path d="m7 9-3 3 3 3M17 9l3 3-3 3" /></>),
  magnet: (<><path d="M6 4v8a6 6 0 0 0 12 0V4h-4v8a2 2 0 0 1-4 0V4z" /><path d="M6 8h4M14 8h4" /></>),
  hand: (<><path d="M8 13V5.5a1.5 1.5 0 0 1 3 0V11M11 10V4.5a1.5 1.5 0 0 1 3 0V11M14 10.5V6a1.5 1.5 0 0 1 3 0v8a7 7 0 0 1-7 7h-.5A6.5 6.5 0 0 1 4 16l-1.5-3a1.5 1.5 0 0 1 2.6-1.5L8 15" /></>),
  face: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M12 3 20 7.5 12 12 4 7.5z" fill="currentColor" fillOpacity=".35" /></>),
  edge: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" /><path d="M12 12v9" strokeWidth="3" /></>),
  object: (<><path d="M12 3 20 7.5v9L12 21l-8-4.5v-9z" fill="currentColor" fillOpacity=".2" /></>),
  boxSelect: (<><path d="M4 4h16v16H4z" strokeDasharray="3 2" /><path d="M11 11v8l2.2-2.2 1.5 3.2 1.3-.6-1.5-3.1H18z" /></>),
  keyboard: (<><rect x="2" y="6" width="20" height="12" rx="1" /><path d="M6 10h1M10 10h1M14 10h1M18 10h1M7 14h10" /></>),
  menu: (<path d="M4 7h16M4 12h16M4 17h16" />),
  sample: (<><path d="M4 20V10l8-6 8 6v10z" /><rect x="8" y="12" width="8" height="8" /></>),
};

export type IconName = keyof typeof P;

export function Icon({ name, size = 18, style, className }: { name: string; size?: number; style?: CSSProperties; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      style={style}
      className={className}
      aria-hidden
    >
      {P[name] ?? P.info}
    </svg>
  );
}
