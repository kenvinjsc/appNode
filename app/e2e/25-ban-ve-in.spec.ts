// TC-13.8 Bản vẽ in (D29): phòng Bếp → 1 trang A3 có mặt đứng, mặt bằng, khung tên; tổng kích thước = tổng rộng tủ dưới.
import { expect, test } from '@playwright/test';
import { api, freshSample, pick } from './helpers';

test('TC-13.8 bản vẽ A3 phòng Bếp', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới01');
  await page.locator('.print-sheet .icon-btn').click();
  await page.locator('.print-pop .struct-row', { hasText: 'Người vẽ' }).locator('input').fill('E2E');
  const [pop] = await Promise.all([page.context().waitForEvent('page'), page.getByRole('button', { name: 'Xem & in' }).click()]);
  await pop.waitForLoadState();
  await expect(pop.locator('svg.page')).toHaveCount(1);
  await expect(pop.locator('text', { hasText: 'Mặt đứng' })).toHaveCount(1);
  await expect(pop.locator('text', { hasText: 'E2E' })).toHaveCount(1);
  const d = await api(page, { cmd: 'get_drawing_sheet', room: 'Bếp', floor: 'Tầng 1' });
  const dims = d.sheets[0].items.filter((i: any) => i.cls === 'dimtext').map((i: any) => i.value);
  expect(dims).toContain(2000);
  await pop.close();
});
