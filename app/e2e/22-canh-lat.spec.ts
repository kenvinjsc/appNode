// TC-06.12 Cánh lật tay nâng (D17): tủ trên → 1 cánh lật HK + 1 bộ tay nâng; khoang quá thấp → thông báo.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, pick } from './helpers';

test('TC-06.12 cánh lật HK trên tủ trên 800 × 400, báo giá có tay nâng', async ({ page }) => {
  await freshSample(page);
  const c = await api(page, { cmd: 'create_cabinet', kind: 'WALL', name: 'TủLật', overrides: { width: 800, height: 400, depth: 350, doors: 0, shelves: 0 } });
  await page.reload();
  const name = (await api(page, { cmd: 'get_properties', id: c.id })).name as string;
  await pick(page, name);
  const cab = c.id as number;
  await page.locator('.d2e-zone').first().click({ button: 'right', force: true });
  await page.getByText('Cánh lật tay nâng HK').click();
  await expect.poll(() => countNamed(page, 'CửaLật', cab)).toBeGreaterThan(0);
  await expect.poll(async () => JSON.stringify(await api(page, { cmd: 'get_costing' })).includes('Tay nâng cánh lật HK')).toBe(true);
  // Tủ trên thấp 300: báo lỗi tiếng Việt, không đổi dữ liệu.
  const low = await api(page, { cmd: 'create_cabinet', kind: 'WALL', overrides: { width: 800, height: 330, depth: 350, doors: 0, shelves: 0 } });
  const z = (await api(page, { cmd: 'get_zones', cabinet: low.id })).zones[0].id;
  const r = await page.evaluate(async (b) => (await (await fetch('/api', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(b) })).json()), { cmd: 'zone_add_doors', cabinet: low.id, zones: [z], kind: 'LIFT_UP' });
  expect(r.error.details.constraint).toBe('LIFT_HEIGHT');
});
