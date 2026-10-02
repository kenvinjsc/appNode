// Đọc số tiền bằng chữ (tiếng Việt) cho báo giá in: 29039040 → "Hai mươi chín triệu không trăm ba mươi chín nghìn không trăm bốn mươi đồng".
const DIGITS = ['không', 'một', 'hai', 'ba', 'bốn', 'năm', 'sáu', 'bảy', 'tám', 'chín'];
const UNITS = ['', 'nghìn', 'triệu', 'tỷ', 'nghìn tỷ', 'triệu tỷ'];

/** Đọc một nhóm 3 chữ số. `full` = đã có nhóm lớn hơn phía trước (đọc cả "không trăm", "lẻ"). */
function readTriple(n: number, full: boolean): string {
  const h = Math.floor(n / 100);
  const t = Math.floor((n % 100) / 10);
  const u = n % 10;
  const out: string[] = [];
  if (h > 0 || full) out.push(`${DIGITS[h]} trăm`);
  if (t === 0) {
    if (u > 0 && (h > 0 || full)) out.push('lẻ');
  } else if (t === 1) {
    out.push('mười');
  } else {
    out.push(`${DIGITS[t]} mươi`);
  }
  if (u > 0) {
    if (u === 1 && t > 1) out.push('mốt');
    else if (u === 5 && t > 0) out.push('lăm');
    else if (u === 4 && t > 1) out.push('tư');
    else out.push(DIGITS[u]);
  }
  return out.join(' ');
}

export function numberToWords(value: number): string {
  let n = Math.round(Math.abs(value));
  if (n === 0) return 'Không';
  const groups: number[] = [];
  while (n > 0) {
    groups.push(n % 1000);
    n = Math.floor(n / 1000);
  }
  const parts: string[] = [];
  for (let i = groups.length - 1; i >= 0; i--) {
    const g = groups[i];
    if (g === 0) continue;
    const full = i < groups.length - 1;
    parts.push([readTriple(g, full), UNITS[i]].filter(Boolean).join(' '));
  }
  const s = (value < 0 ? 'âm ' : '') + parts.join(' ');
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export const moneyWords = (value: number) => `${numberToWords(value)} đồng`;
