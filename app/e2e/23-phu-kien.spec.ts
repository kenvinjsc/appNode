// TC-05.10 Phụ kiện khoang (D18): giá bát 800 vào khoang hẹp → cảnh báo, khoang đỏ; đổi tủ rộng → hết cảnh báo.
import { expect, test } from '@playwright/test';
import { api, freshSample, pick } from './helpers';

test('TC-05.10 giá bát 800: không vừa → tô đỏ, tủ rộng 800 → vừa', async ({ page }) => {
  await freshSample(page);
  const c = await api(page, { cmd: 'create_cabinet', kind: 'BASE', overrides: { width: 600, doors: 0, shelves: 0 } });
  await page.reload();
  const name = (await api(page, { cmd: 'get_properties', id: c.id })).name as string;
  await pick(page, name);
  await page.locator('.d2e-zone').first().click({ button: 'right', force: true });
  await page.getByText('Giá bát 800').click();
  await expect(page.locator('.toast', { hasText: 'không vừa khoang' })).toBeVisible();
  await expect(page.locator('.d2e-zone.misfit')).toHaveCount(1);
  await api(page, { cmd: 'set_parameter', id: c.id, name: 'width', value: '800' });
  await expect.poll(async () => (await api(page, { cmd: 'get_zones', cabinet: c.id })).misfits.length).toBe(0);
  await page.reload();
  await pick(page, name);
  await expect(page.locator('.d2e-zone').first()).toBeVisible();
  await expect(page.locator('.d2e-zone.misfit')).toHaveCount(0);
  expect(JSON.stringify(await api(page, { cmd: 'get_costing' }))).toContain('Giá bát đĩa 800');
});
