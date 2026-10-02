// TC-06 Sửa tham số: kích thước tủ + chế độ dãn, khoang KHÓA/AUTO trên 2D, di chuyển khối, template.
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, pick } from './helpers';

const bays = async (page: import('@playwright/test').Page, cab: number) => {
  const z = await api(page, { cmd: 'get_zones', cabinet: cab });
  return z.bays as { zone: number; index: number; size: number; mode: string | null; axis: number }[];
};

test('TC-06.1 nhập kích thước khoang trên 2D → khoang KHÓA đúng số', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'TủQA01');
  const cab = await idOf(page, 'TủQA01');
  const b0 = (await bays(page, cab)).filter((b) => b.axis === 0);
  expect(b0.length).toBeGreaterThanOrEqual(2);
  // Bấm số kích thước khoang đầu trên 2D rồi gõ 600.
  const label = page.locator('.d2e-bay .d2e-num').first();
  await label.click({ force: true });
  const input = page.locator('.d2e-input');
  await input.fill('600');
  await input.press('Enter');
  await expect
    .poll(async () => (await bays(page, cab)).some((b) => Math.abs(b.size - 600) < 0.01 && b.mode === 'LOCK'))
    .toBe(true);
});

test('TC-06.2 đổi rộng tủ "dãn đều" vs "chỉ khoang sát cạnh"', async ({ page }) => {
  await freshSample(page);
  const cab = await idOf(page, 'TủQA01');
  const w = (xs: { axis: number; size: number; zone: number }[]) => xs.filter((b) => b.axis === 0).map((b) => b.size);
  const before = w(await bays(page, cab));
  await api(page, { cmd: 'resize_cabinet', id: cab, name: 'width', value: 1800, stretch: 'EDGE', edge: 'END' });
  const edge = w(await bays(page, cab));
  expect(edge[0]).toBeCloseTo(before[0], 3);
  expect(edge[edge.length - 1] - before[before.length - 1]).toBeCloseTo(200, 3);
  await api(page, { cmd: 'undo' });
  await api(page, { cmd: 'resize_cabinet', id: cab, name: 'width', value: 1800, stretch: 'PROPORTIONAL' });
  const prop = w(await bays(page, cab));
  expect(prop[0] / prop[1]).toBeCloseTo(before[0] / before[1], 3);
});

test('TC-06.3 di chuyển khối theo vector: một bước undo', async ({ page }) => {
  await freshSample(page);
  const cab = await idOf(page, 'BếpDưới03');
  const t0 = await api(page, { cmd: 'get_transform', id: cab });
  await api(page, { cmd: 'move_objects', ids: [cab], delta: [300, 0, 0] });
  const t1 = await api(page, { cmd: 'get_transform', id: cab });
  expect(JSON.stringify(t1)).not.toBe(JSON.stringify(t0));
  await api(page, { cmd: 'undo' });
  expect(JSON.stringify(await api(page, { cmd: 'get_transform', id: cab }))).toBe(JSON.stringify(t0));
});

test('TC-06.4 lưu template rồi chèn tủ mới cùng cấu trúc', async ({ page }) => {
  await freshSample(page);
  const cab = await idOf(page, 'TủQA01');
  await api(page, { cmd: 'save_template', cabinet: cab, name: 'E2E tủ áo', to_library: false });
  const r = await api(page, { cmd: 'insert_template', name: 'E2E tủ áo', width: 1200, height: 2200, depth: 580 });
  const z = await api(page, { cmd: 'get_zones', cabinet: r.id });
  expect(z.size).toEqual([1200, 2200, 580]);
  expect(z.bays.length).toBeGreaterThan(0);
  await api(page, { cmd: 'delete_template', name: 'E2E tủ áo' });
});
