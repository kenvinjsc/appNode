import { describe, expect, it } from 'vitest';
import { touchesZones } from './useZones';

describe('touchesZones (2D chỉ tải lại khoang của tủ bị đổi)', () => {
  it('chỉ phản ứng với sự kiện của đúng tủ', () => {
    expect(touchesZones([{ type: 'ZonesChanged', cabinets: [2] }], 2)).toBe(true);
    expect(touchesZones([{ type: 'ZonesChanged', cabinets: [5] }], 2)).toBe(false);
    expect(touchesZones([{ type: 'TransformChanged', ids: [2, 3] }], 2)).toBe(true);
    expect(touchesZones([{ type: 'GeometryChanged', ids: [2] }], 2)).toBe(false);
    expect(touchesZones([{ type: 'SceneTreeChanged' }], 2)).toBe(false);
    expect(touchesZones([{ type: 'ProjectLoaded' }], 2)).toBe(true);
  });
});
