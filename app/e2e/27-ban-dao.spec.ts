// TC-02.11 Bàn đảo (D23): mẫu Bàn đảo 1800 → mở 2 mặt, không hậu, mặt đá nhô phía ghế, đứng trước dãy bếp.
import { expect, test } from '@playwright/test';
import { api, countNamed, flatten, freshSample, idOf, tree } from './helpers';

test('TC-02.11 bàn đảo 1800 mở 2 mặt', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Tủ ▾' }).click();
  await page.getByText('Mẫu dựng sẵn (tủ áo').click();
  await page.locator('.gallery-card', { hasText: 'Bàn đảo 1800' }).click();
  await page.getByRole('button', { name: 'Chèn mẫu' }).click();
  await expect.poll(() => idOf(page, 'BànĐảo').catch(() => 0)).toBeGreaterThan(0);
  const id = await idOf(page, 'BànĐảo');
  expect(await countNamed(page, 'HậuPhụ', id)).toBe(1); // vách giữa
  const parts = flatten(flatten(await tree(page)).find((n) => n.id === id)!.children);
  expect(parts.filter((n) => n.name === 'Hậu').length).toBe(0); // không có tấm hậu
  expect(await countNamed(page, 'MặtĐáBànĐảo', id)).toBe(1);
  expect(await countNamed(page, 'ỐpHông', id)).toBe(2);
  // Không chồng lên dãy bếp: nằm trước các tủ bếp dưới.
  const isl = await api(page, { cmd: 'get_bounds', ids: [id] });
  const b1 = await api(page, { cmd: 'get_bounds', ids: [await idOf(page, 'BếpDưới01')] });
  expect(isl.min[2]).toBeGreaterThan(b1.max[2]);
});
