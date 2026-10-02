import { describe, expect, it } from 'vitest';
import { chordOf, DEFAULT_SHORTCUTS, matchAction } from './shortcuts';

const ev = (key: string, mods: Partial<KeyboardEvent> = {}) => ({ key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, ...mods }) as KeyboardEvent;

describe('shortcuts', () => {
  it('builds chords', () => {
    expect(chordOf(ev('z', { ctrlKey: true }))).toBe('Ctrl+Z');
    expect(chordOf(ev('Z', { ctrlKey: true, shiftKey: true }))).toBe('Ctrl+Shift+Z');
    expect(chordOf(ev('Delete'))).toBe('Delete');
  });
  it('matches default actions', () => {
    expect(matchAction(ev('d', { ctrlKey: true }), DEFAULT_SHORTCUTS)).toBe('duplicate');
    expect(matchAction(ev('1'), DEFAULT_SHORTCUTS)).toBe('viewFront');
    expect(matchAction(ev('m'), DEFAULT_SHORTCUTS)).toBe('move');
    expect(matchAction(ev('F', { shiftKey: true }), DEFAULT_SHORTCUTS)).toBe('fitAll');
  });
});
