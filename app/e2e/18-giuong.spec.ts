// TC-02.7 Giường (D19): chèn từ menu Tủ ▾, đổi kiểu hộc kéo trên tab Giường, một undo.
import { expect, test } from '@playwright/test';
import { countNamed, freshSample, idOf, menu } from './helpers';

test('TC-02.7 giường 1600 × 2000: dát nan 14, đà giữa; bật hộc kéo 2 bên → 4 hộc', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Tủ ▾' }).click();
  await page.getByText('Giường 1600 × 2000').click();
  await expect.poll(() => idOf(page, 'Giường').catch(() => 0)).toBeGreaterThan(0);
  const bed = await idOf(page, 'Giường');
  expect(await countNamed(page, 'NanDát', bed)).toBe(14);
  expect(await countNamed(page, 'ĐàGiữa', bed)).toBe(1);
  await menu(page, 'Giường', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Giường', exact: true }).click();
  await dlg.locator('.struct-row', { hasText: 'Kiểu' }).last().locator('select').selectOption('DRAWERS_2_SIDES');
  await expect.poll(() => countNamed(page, 'MặtHộcGiường', bed)).toBe(4);
});
