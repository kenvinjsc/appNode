// TC-02.10 Mẫu dựng sẵn (D30): lưới mẫu theo phòng, sửa W + tham số kịch trần, chèn một undo.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, idOf } from './helpers';

test('TC-02.10 tủ áo 4 cánh 1800 → W 2000, kịch trần 2700', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Tủ ▾' }).click();
  await page.getByText('Mẫu dựng sẵn (tủ áo').click();
  const g = page.locator('.gallery');
  await g.locator('.seg button', { hasText: 'Phòng ngủ' }).click();
  await g.locator('.gallery-card', { hasText: 'Tủ áo 4 cánh 1800' }).click();
  await g.locator('.struct-row', { hasText: 'Rộng' }).locator('input').fill('2000');
  await g.locator('.struct-row', { hasText: 'Kịch trần' }).locator('input').check();
  await g.getByRole('button', { name: 'Chèn mẫu' }).click();
  await expect(g).toHaveCount(0);
  await expect.poll(() => idOf(page, 'TủÁo').catch(() => 0)).toBeGreaterThan(0);
  const id = await idOf(page, 'TủÁo');
  const b = await api(page, { cmd: 'get_structure', cabinet: id });
  const general = b.tabs.find((t: any) => t.key === 'general').fields;
  expect(general.find((f: any) => f.key === 'width').value).toBe(2000);
  expect(general.find((f: any) => f.key === 'height').value).toBe(2650);
  expect(await countNamed(page, 'MặtNgănTrong', id)).toBe(6);
  await page.keyboard.press('Control+z');
  await expect.poll(() => idOf(page, 'TủÁo').catch(() => 0)).toBe(0);
});
