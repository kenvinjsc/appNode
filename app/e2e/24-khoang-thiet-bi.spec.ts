// TC-05.11 Khoang thiết bị (D34): khoang lò 600 → thanh đỡ + khoét thoát nhiệt; khoang thấp → báo lỗi.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, pick } from './helpers';

test('TC-05.11 khoang lò 600 trên tủ 600 × 700; khoang 400 → lỗi', async ({ page }) => {
  await freshSample(page);
  const c = await api(page, { cmd: 'create_cabinet', kind: 'BASE', overrides: { width: 600, height: 700, depth: 580, doors: 0, shelves: 0 } });
  await page.reload();
  const name = (await api(page, { cmd: 'get_properties', id: c.id })).name as string;
  await pick(page, name);
  await page.locator('.d2e-zone').first().click({ button: 'right', force: true });
  await page.getByText('Khoang lò 600').click();
  await expect.poll(() => countNamed(page, 'ThanhĐỡThiếtBị', c.id)).toBe(1);
  // Tủ thấp: báo lỗi tiếng Việt.
  const low = await api(page, { cmd: 'create_cabinet', kind: 'BASE', overrides: { width: 600, height: 450, depth: 580, doors: 0, shelves: 0 } });
  await page.reload();
  await pick(page, (await api(page, { cmd: 'get_properties', id: low.id })).name as string);
  await page.locator('.d2e-zone').first().click({ button: 'right', force: true });
  await page.getByText('Khoang lò 600').click();
  await expect(page.locator('.toast', { hasText: 'Khoang nhỏ hơn kích thước lọt lòng' })).toBeVisible();
});
