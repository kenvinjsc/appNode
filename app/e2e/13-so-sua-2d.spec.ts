// TC-04.9 Số đo sửa trực tiếp trên 2D do core tính (D03): view Bên → bấm sâu 600 → nhập 580.
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, pick } from './helpers';

test('TC-04.9 view Bên: sửa sâu tủ và lùi kệ ngay trên bản vẽ', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới01');
  const cab = await idOf(page, 'BếpDưới01');
  await page.locator('.panel.drawing select').first().selectOption('side');
  const depth = page.locator('.d2-editdim text', { hasText: /^Sâu: bấm để nhập600$/ });
  await expect(depth).toHaveCount(1);
  await depth.click({ force: true });
  const input = page.locator('.d2-editdims input');
  await input.fill('580');
  await input.press('Enter');
  await expect.poll(async () => (await api(page, { cmd: 'get_zones', cabinet: cab })).size[2]).toBe(580);
  // Số trên bản vẽ cập nhật theo core.
  await expect(page.locator('.d2-editdim text', { hasText: /580$/ })).toHaveCount(1);
  // View Trước: tay nắm cách đầu cánh 60 → 80.
  await page.locator('.panel.drawing select').first().selectOption('front');
  const handle = page.locator('.d2-editdim text', { hasText: /^Tay nắm cách đầu cánh: bấm để nhập60$/ });
  await handle.click({ force: true });
  await page.locator('.d2-editdims input').fill('80');
  await page.locator('.d2-editdims input').press('Enter');
  await expect.poll(async () => (await api(page, { cmd: 'get_zones', cabinet: cab })).dims.find((d: any) => d.name === 's_handle_from_end')?.value).toBe(80);
  await page.mouse.move(600, 700);
  await page.keyboard.press('Control+z');
  await page.keyboard.press('Control+z');
  await expect.poll(async () => (await api(page, { cmd: 'get_zones', cabinet: cab })).size[2]).toBe(600);
});
