import { describe, expect, it } from 'vitest';
import { describeError } from './errors';

describe('describeError', () => {
  it('never exposes the technical message and localises constraints', () => {
    const m = describeError({ code: 'CONSTRAINT_VIOLATED', message: 'constraint WIDTH_LESS_THAN_SIDES violated', details: { constraint: 'WIDTH_LESS_THAN_SIDES' } });
    expect(m.title).toBe('Không thể cập nhật kích thước.');
    expect(m.detail).toBe('Chiều rộng không thể nhỏ hơn tổng chiều dày hai hồi.');
  });
  it('has a fallback for unknown codes', () => {
    const m = describeError({ code: 'SOMETHING_NEW', message: 'panic at src/lib.rs', details: null });
    expect(m.detail).not.toContain('src/lib.rs');
  });
});
