// TC-03.8 Căn chỉnh (D27): căn mặt trước 3 tủ từ ribbon, một undo; xoay 90°.
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, pick } from './helpers';

const front = async (page: import('@playwright/test').Page, id: number) => (await api(page, { cmd: 'get_bounds', ids: [id] })).max[2] as number;

test('TC-03.8 căn mặt trước 3 tủ bếp dưới, một undo', async ({ page }) => {
  await freshSample(page);
  const ids = [await idOf(page, 'BếpDưới01'), await idOf(page, 'BếpDưới02'), await idOf(page, 'BếpDưới03')];
  await api(page, { cmd: 'move_objects', ids: [ids[1]], delta: [0, 0, -120] });
  expect(Math.abs((await front(page, ids[1])) - (await front(page, ids[0])))).toBeGreaterThan(100);
  await pick(page, 'BếpDưới01');
  await pick(page, 'BếpDưới02', true);
  await pick(page, 'BếpDưới03', true);
  await page.getByRole('button', { name: 'Chỉnh sửa', exact: true }).click();
  await page.getByRole('button', { name: 'Căn trước', exact: true }).click();
  await expect.poll(async () => Math.abs((await front(page, ids[1])) - (await front(page, ids[0])))).toBeLessThan(0.01);
  expect(Math.abs((await front(page, ids[2])) - (await front(page, ids[0])))).toBeLessThan(0.01);
  await page.mouse.move(600, 700);
  await page.keyboard.press('Control+z');
  await expect.poll(async () => Math.abs((await front(page, ids[1])) - (await front(page, ids[0])))).toBeGreaterThan(100);
  // Xoay 90°: hộp bao đổi rộng ↔ sâu.
  const b0 = await api(page, { cmd: 'get_bounds', ids: [ids[2]] });
  await api(page, { cmd: 'rotate_objects', ids: [ids[2]], deg: 90, pivot: 'CENTER' });
  const b1 = await api(page, { cmd: 'get_bounds', ids: [ids[2]] });
  expect(b1.max[0] - b1.min[0]).toBeCloseTo(b0.max[2] - b0.min[2], 1);
});
