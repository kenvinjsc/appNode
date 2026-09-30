// TC-06.8 / 06.9 Ngăn kéo trong (sau cánh) và mặt giả tủ chậu, dựng từ menu chuột phải trên 2D.
import { expect, test } from '@playwright/test';
import { countNamed, freshSample, idOf, pick } from './helpers';

test('TC-06.8 ngăn kéo trong × 2 từ menu Dựng nhanh', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'TủQA01');
  const cab = await idOf(page, 'TủQA01');
  const zone = page.locator('.d2e-zone').first();
  await zone.click({ button: 'right', force: true });
  await page.getByText('Ngăn kéo trong × 2 (sau cánh)').click();
  await expect.poll(() => countNamed(page, 'MặtNgănTrong', cab)).toBe(2);
  await page.keyboard.press('Control+z');
  await expect.poll(() => countNamed(page, 'MặtNgănTrong', cab)).toBe(0);
});

test('TC-06.9 mặt giả tủ chậu: không hộc, không ray', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  const cab = await idOf(page, 'BếpDưới03');
  await page.locator('.d2e-zone').last().click({ button: 'right', force: true });
  await page.getByText('Mặt giả (tủ chậu, không hộc)').click();
  await expect.poll(() => countNamed(page, 'MặtGiả', cab)).toBeGreaterThan(0);
  expect(await countNamed(page, 'RayBi', cab)).toBe(0);
});
