// TC-02 Chia khoang theo công thức (hộp thoại Chia ngang / Chia dọc), trên 2D.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, idOf, pick, undo } from './helpers';

test('TC-02.1 chia ngang 300 từ trên xuống, có tấm; chia dọc /2 không tấm', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  const cab = await idOf(page, 'BếpDưới03');
  const shelves0 = await countNamed(page, 'KệCốĐịnh', cab);
  await page.mouse.move(1100, 500);
  await page.keyboard.press('k');
  const dlg = page.locator('.split-dlg');
  await expect(dlg).toBeVisible();
  await dlg.locator('input').fill('300');
  const zones = page.locator('.d2e-zone');
  const n0 = await zones.count();
  await zones.first().click({ force: true });
  await expect.poll(() => countNamed(page, 'KệCốĐịnh', cab)).toBe(shelves0 + 1);
  await expect.poll(() => zones.count()).toBe(n0 + 1);
  // Chia dọc không tạo tấm.
  await dlg.getByRole('button', { name: 'Chia dọc' }).click();
  await dlg.locator('select').nth(1).selectOption('n');
  await dlg.locator('input').fill('/2');
  await zones.first().click({ force: true });
  await expect.poll(() => zones.count()).toBe(n0 + 2);
  expect(await countNamed(page, 'KệCốĐịnh', cab)).toBe(shelves0 + 1);
  // Mỗi lần chia là một bước undo.
  await undo(page);
  await undo(page);
  await expect.poll(() => zones.count()).toBe(n0);
  await page.keyboard.press('Escape');
  await expect(dlg).toBeHidden();
});

test('TC-02.2 công thức vượt khoang bị từ chối, báo lỗi tiếng Việt', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  await page.mouse.move(1100, 500);
  await page.keyboard.press('k');
  await page.locator('.split-dlg input').fill('5000');
  const zones = page.locator('.d2e-zone');
  const n0 = await zones.count();
  await zones.first().click({ force: true });
  await expect(page.locator('.toast').first()).toContainText('Không thể');
  expect(await zones.count()).toBe(n0);
  const r = await page.request.post('/api', { data: { cmd: 'split_zone', cabinet: await idOf(page, 'BếpDưới03'), zone: 1, kind: 'SHELF_FIXED', formula: 'abc' } });
  expect((await r.json()).ok).toBe(false);
  void api;
});
