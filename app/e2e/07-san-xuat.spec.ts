// TC-07 Sản xuất: trải phẳng tấm (gia công), xếp tấm, G-code.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, tree } from './helpers';

test('TC-07.1 trải phẳng hồi tủ áo có lỗ khoan / rãnh', async ({ page }) => {
  await freshSample(page);
  const side = flatten(await tree(page)).find((n) => n.name === 'HồiTrái')!;
  const flat = await api(page, { cmd: 'get_manufacturing', id: side.id });
  expect(flat.width).toBeGreaterThan(0);
  expect(flat.features.length).toBeGreaterThan(0);
});

test('TC-07.2 xếp tấm không chồng, G-code có lệnh chạy dao', async ({ page }) => {
  await freshSample(page);
  const n = await api(page, { cmd: 'run_nesting' });
  expect(n.jobs.length).toBeGreaterThan(0);
  const job = n.jobs.find((j: any) => j.result.placements.length > 0);
  expect(job.result.unplaced.length).toBe(0);
  const cnc = await api(page, { cmd: 'generate_cnc', material: job.material_id, sheet_id: job.result.sheets[0].id });
  expect(cnc.program.gcode).toMatch(/G0|G1/);
  expect(cnc.placements.length).toBeGreaterThan(0);
});

test('TC-07.3 mở workspace Xếp tấm và CNC không lỗi trang', async ({ page }) => {
  const errors = await freshSample(page);
  await page.getByText('Xếp tấm').first().click();
  await page.waitForTimeout(1500);
  await page.getByText('CNC').first().click();
  await page.waitForTimeout(1500);
  expect(errors).toEqual([]);
});
