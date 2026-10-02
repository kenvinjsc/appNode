// TC-04.8 Xem trước do core tính khi kéo kệ (D25): số hiển thị = vị trí đã bắt lỗ hệ 32; chưa thả thì chưa có undo.
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, menu, pick } from './helpers';

test('TC-04.8 kéo kệ với hàng lỗ 32: số xem trước là bội 32 (+69), thả mới lưu', async ({ page }) => {
  await freshSample(page);
  await menu(page, 'TủQA01', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Kệ & chốt tầng' }).click();
  await dlg.locator('.struct-row', { hasText: 'Lỗ chốt tầng' }).locator('select').selectOption('ROW_32');
  await dlg.locator('.icon-btn[title="Đóng"]').click();
  await pick(page, 'TủQA01');
  const cab = await idOf(page, 'TủQA01');
  const handle = page.locator('.d2e-handle.y').first();
  await expect(handle).toBeVisible();
  const box = (await handle.boundingBox())!;
  const status0 = await api(page, { cmd: 'get_status' });
  const read = async () => (await page.locator('.d2e-bay .d2e-num').allTextContents()).map((t) => Number((t.match(/([\d.,]+)$/)?.[1] ?? '').replace(/\./g, '').replace(',', '.')));
  const before = await read();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2 - 23, { steps: 6 });
  await page.waitForTimeout(400);
  // Đang kéo: chưa đổi dự án.
  const mid = await api(page, { cmd: 'get_status' });
  expect(mid.revision ?? mid.rev).toEqual(status0.revision ?? status0.rev);
  const during = await read();
  const changed = during.filter((v, i) => Number.isFinite(v) && Math.abs(v - before[i]) > 0.01);
  expect(changed.length, `trước: ${before.join(' | ')} · đang kéo: ${during.join(' | ')}`).toBeGreaterThan(0);
  expect(changed.some((v) => Math.abs(((v - 69) % 32 + 32) % 32) < 0.01), `đổi: ${changed.join(' | ')}`).toBe(true);
  await page.mouse.up();
  const input = page.locator('.value-box input');
  if (await input.count()) await input.press('Enter');
  await expect.poll(async () => {
    const z = await api(page, { cmd: 'get_zones', cabinet: cab });
    return z.positions.filter((p: any) => p.axis === 1).every((p: any) => Math.abs(((p.from_start - 69) % 32 + 32) % 32) < 0.01);
  }).toBe(true);
});
