// TC-05.9 Kệ giày nghiêng thật: tấm xoay quanh trục X, có thanh chặn gót, không chốt kệ.
import { expect, test } from '@playwright/test';
import { countNamed, freshSample, idOf, pick } from './helpers';

test('TC-05.9 kệ giày nghiêng 15° × 4 từ menu Dựng nhanh, một Undo', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'TủQA01');
  const cab = await idOf(page, 'TủQA01');
  const before = await countNamed(page, 'ThanhChặnGót', cab);
  await page.locator('.d2e-zone').first().click({ button: 'right', force: true });
  await page.getByText('Kệ giày nghiêng 15° × 4').click();
  await expect.poll(() => countNamed(page, 'ThanhChặnGót', cab)).toBe(before + 4);
  await page.keyboard.press('Control+z');
  await expect.poll(() => countNamed(page, 'ThanhChặnGót', cab)).toBe(before);
});
