// Keyboard shortcut layer + configurable shortcut sheet.
import { useEffect, useState } from 'react';
import { ACTION_LABEL, chordOf, loadShortcuts, matchAction, saveShortcut, type ActionId } from '../shared/shortcuts';
import { Icon } from '../shared/icons';
import { View } from '../viewport/viewportBus';
import { Actions } from './actions';
import { useUi } from './uiStore';

export function useShortcutLayer() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement;
      if (t && (t.tagName === 'INPUT' || t.tagName === 'SELECT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
      const a = matchAction(e, loadShortcuts());
      if (!a) return;
      const s = useUi.getState();
      e.preventDefault();
      const design = s.workspace === 'design';
      switch (a) {
        case 'delete':
          return void Actions.delete();
        case 'duplicate':
          return void Actions.duplicate();
        case 'undo':
          return void Actions.undo();
        case 'redo':
        case 'redoAlt':
          return void Actions.redo();
        case 'move':
          return s.setTool({ type: 'move' });
        case 'rotate':
          return s.setTool({ type: 'rotate' });
        case 'select':
          return s.setTool({ type: 'select' });
        case 'fitSelection':
          return design && View.fit(s.selection, s.tree);
        case 'fitAll':
          return design && View.fitAll();
        case 'viewFront':
          return design && View.set('front', s.selection, s.tree);
        case 'viewBack':
          return design && View.set('back', s.selection, s.tree);
        case 'viewLeft':
          return design && View.set('left', s.selection, s.tree);
        case 'viewRight':
          return design && View.set('right', s.selection, s.tree);
        case 'viewTop':
          return design && View.set('top', s.selection, s.tree);
        case 'viewPerspective':
          return design && View.set('iso', s.selection, s.tree);
        case 'save':
          return void Actions.save();
        case 'open':
          return Actions.open();
        case 'escape':
          if (s.tool.type !== 'select') return s.setTool({ type: 'select' });
          if (s.contextMenu) return s.set({ contextMenu: null });
          if (s.drawer) return s.set({ drawer: null });
          return s.clearSelection();
        case 'hide':
          return void Actions.toggleHidden();
        case 'rename':
          return s.active !== null && s.set({ renaming: s.active });
        case 'relations':
          return s.set({ showRelations: !s.showRelations });
        case 'dimensions':
          return s.set({ showDimensions: !s.showDimensions });
        case 'selectAll':
          return Actions.selectAll();
        case 'shortcuts':
          return s.set({ drawer: s.drawer === 'shortcuts' ? null : 'shortcuts' });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}

export function ShortcutSheet() {
  const [map, setMap] = useState(loadShortcuts());
  const [editing, setEditing] = useState<ActionId | null>(null);
  useEffect(() => {
    if (!editing) return;
    const on = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;
      const c = chordOf(e);
      saveShortcut(editing, c);
      setMap(loadShortcuts());
      setEditing(null);
    };
    window.addEventListener('keydown', on, { capture: true });
    return () => window.removeEventListener('keydown', on, { capture: true });
  }, [editing]);
  return (
    <div className="drawer-panel">
      <div className="panel-header">
        <Icon name="keyboard" size={16} />
        <span>Phím tắt</span>
        <div className="spacer" />
        <button className="icon-btn" onClick={() => useUi.getState().set({ drawer: null })}>
          <Icon name="x" size={16} />
        </button>
      </div>
      <div className="shortcut-list">
        {(Object.keys(ACTION_LABEL) as ActionId[]).map((a) => (
          <div key={a} className="shortcut-row">
            <span>{ACTION_LABEL[a]}</span>
            <button className={editing === a ? 'rec' : ''} onClick={() => setEditing(a)} title="Nhấp rồi bấm tổ hợp phím mới">
              {editing === a ? 'Bấm phím…' : map[a]}
            </button>
          </div>
        ))}
      </div>
      <p className="muted small">Nhấp vào phím để gán lại. Thiết lập được lưu cho máy này.</p>
    </div>
  );
}
