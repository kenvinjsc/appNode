// TC-06.11 Cánh lùa theo hệ ray (D16): 3 cánh chồng 35; khung nhôm + gương từ bảng Chỉnh tấm.
import { expect, test } from '@playwright/test';
import { api, countNamed, flatten, freshSample, pick, tree } from './helpers';

test('TC-06.11 cánh lùa 3 cánh, khung nhôm bản 45 + gương', async ({ page }) => {
  await freshSample(page);
  const c = await api(page, { cmd: 'create_cabinet', kind: 'BASE', name: 'TủLùa', overrides: { width: 2400, height: 2400, depth: 620, doors: 0, shelves: 2 } });
  const z = (await api(page, { cmd: 'get_zones', cabinet: c.id })).zones[0].id;
  await api(page, { cmd: 'zone_add_doors', cabinet: c.id, zones: [z], kind: 'SLIDING', cols: 3, mount: 'OVERLAY' });
  await page.reload();
  const cab = flatten(await tree(page)).find((n) => n.id === c.id)!;
  expect(await countNamed(page, 'CửaLùa', c.id)).toBe(3);
  const leaf = flatten(cab.children).find((n) => n.name.startsWith('CửaLùa'))!;
  const props = await api(page, { cmd: 'get_properties', id: leaf.id });
  expect(JSON.stringify(props)).toContain('door_slide_overlap');
  // Chọn cánh trên cây → Chỉnh tấm → Khung = Nhôm bản 45.
  await pick(page, leaf.name);
  await page.getByRole('button', { name: 'Chỉnh tấm' }).click();
  const row = page.locator('.prop-row, .struct-row, label', { hasText: 'Khung' }).filter({ has: page.locator('select') }).first();
  await row.locator('select').selectOption('ALU_WIDE');
  await expect.poll(() => countNamed(page, 'KhungNhôm', c.id)).toBe(12);
  const bar = flatten(flatten(await tree(page)).find((n) => n.id === c.id)!.children).find((n) => n.name.startsWith('KhungNhôm'))!;
  await api(page, { cmd: 'set_parameter', id: bar.id, name: 'door_slide_infill', value: 'MIRROR' });
  await expect.poll(() => countNamed(page, 'GươngCửa', c.id)).toBe(3);
  const q = JSON.stringify(await api(page, { cmd: 'get_costing' }));
  expect(q).toContain('Profile nhôm cánh');
});
