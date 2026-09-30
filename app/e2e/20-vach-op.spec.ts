// TC-02.9 Vách ốp (D22): 5 tấm theo công thức /5, đổi rộng → tấm giãn đều; đổi công thức trên tab Vách ốp.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, idOf, menu } from './helpers';

test('TC-02.9 vách TV 3000 × 2700: /5 → 5 tấm; công thức /6 → 6 tấm', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Tủ ▾' }).click();
  await page.getByText('Vách TV 3000 × 2700').click();
  await expect.poll(() => idOf(page, 'VáchỐp').catch(() => 0)).toBeGreaterThan(0);
  const w = await idOf(page, 'VáchỐp');
  expect(await countNamed(page, 'TấmỐp', w)).toBe(5);
  await menu(page, 'VáchỐp', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Vách ốp', exact: true }).click();
  const cols = dlg.locator('.struct-row', { hasText: 'Chia cột' }).locator('input');
  await cols.fill('/6');
  await cols.press('Enter');
  await expect.poll(() => countNamed(page, 'TấmỐp', w)).toBe(6);
  const q = await api(page, { cmd: 'get_costing' });
  expect(JSON.stringify(q.quote.rows.find((r: any) => r.id === w))).toContain('m²');
});
