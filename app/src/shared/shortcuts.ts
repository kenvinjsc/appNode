// Shortcut layer: action ids → default key chords. Overrides persist per user
// (localStorage) so the map is configurable without code changes.

export type ActionId =
  | 'delete' | 'duplicate' | 'undo' | 'redo' | 'redoAlt' | 'move' | 'rotate' | 'select' | 'fitSelection' | 'fitAll'
  | 'viewFront' | 'viewBack' | 'viewLeft' | 'viewRight' | 'viewTop' | 'viewPerspective' | 'save' | 'open'
  | 'escape' | 'hide' | 'rename' | 'relations' | 'dimensions' | 'selectAll' | 'shortcuts' | 'splitZone';

export const DEFAULT_SHORTCUTS: Record<ActionId, string> = {
  delete: 'Delete',
  duplicate: 'Ctrl+D',
  undo: 'Ctrl+Z',
  redo: 'Ctrl+Y',
  redoAlt: 'Ctrl+Shift+Z',
  move: 'M',
  rotate: 'R',
  select: 'Q',
  fitSelection: 'F',
  fitAll: 'Shift+F',
  viewFront: '1',
  viewBack: '2',
  viewLeft: '3',
  viewRight: '4',
  viewTop: '5',
  viewPerspective: '0',
  save: 'Ctrl+S',
  open: 'Ctrl+O',
  escape: 'Escape',
  hide: 'H',
  rename: 'F2',
  relations: 'Shift+R',
  dimensions: 'Shift+D',
  selectAll: 'Ctrl+A',
  shortcuts: '?',
  splitZone: 'K',
};

export const ACTION_LABEL: Record<ActionId, string> = {
  delete: 'Xóa',
  duplicate: 'Sao chép',
  undo: 'Hoàn tác',
  redo: 'Làm lại',
  redoAlt: 'Làm lại (phụ)',
  move: 'Di chuyển',
  rotate: 'Xoay',
  select: 'Chọn',
  fitSelection: 'Phóng vừa vùng chọn',
  fitAll: 'Phóng vừa toàn bộ',
  viewFront: 'Nhìn trước',
  viewBack: 'Nhìn sau',
  viewLeft: 'Nhìn trái',
  viewRight: 'Nhìn phải',
  viewTop: 'Nhìn trên',
  viewPerspective: 'Phối cảnh',
  save: 'Lưu',
  open: 'Mở',
  escape: 'Hủy / bỏ chọn',
  hide: 'Ẩn / hiện',
  rename: 'Đổi tên',
  relations: 'Quan hệ lắp ghép',
  dimensions: 'Kích thước',
  selectAll: 'Chọn tất cả',
  shortcuts: 'Bảng phím tắt',
  splitZone: 'Chia khoang',
};

const KEY = 'aic.shortcuts';

export function loadShortcuts(): Record<ActionId, string> {
  try {
    const o = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    return { ...DEFAULT_SHORTCUTS, ...o };
  } catch {
    return { ...DEFAULT_SHORTCUTS };
  }
}

export function saveShortcut(action: ActionId, chord: string) {
  try {
    const o = JSON.parse(localStorage.getItem(KEY) ?? '{}');
    o[action] = chord;
    localStorage.setItem(KEY, JSON.stringify(o));
  } catch {
    /* storage unavailable: keep defaults */
  }
}

export function chordOf(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  let k = e.key;
  if (k === ' ') k = 'Space';
  if (k.length === 1) k = k.toUpperCase();
  if (e.shiftKey && !(k.length === 1 && !/[A-Z0-9]/.test(k))) parts.push('Shift');
  parts.push(k);
  return parts.join('+');
}

export function matchAction(e: KeyboardEvent, map: Record<ActionId, string>): ActionId | null {
  const c = chordOf(e);
  for (const [a, chord] of Object.entries(map)) {
    if (chord === c) return a as ActionId;
  }
  // Digits typed with Shift on some layouts, '?' etc.
  if (e.key === '?' && map.shortcuts === '?') return 'shortcuts';
  return null;
}
