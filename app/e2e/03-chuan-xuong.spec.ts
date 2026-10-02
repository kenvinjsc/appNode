// TC-03 Thuộc tính kết cấu, chuẩn xưởng (D01, D02, D05, D06, D08, D10).
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, menu } from './helpers';

async function pins(page: import('@playwright/test').Page, cab: number) {
  const z = await api(page, { cmd: 'get_structure', cabinet: cab });
  const shelves = z.tabs.find((t: any) => t.key === 'shelves');
  return shelves.fields.find((f: any) => f.key === 's_pin_row').value as string;
}

test('TC-03.1 bật hàng lỗ hệ 32 trên bảng, lưu chuẩn, áp cho tủ khác, undo', async ({ page }) => {
  await freshSample(page);
  await menu(page, 'TủQA01', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Kệ & chốt tầng' }).click();
  await dlg.locator('.struct-row', { hasText: 'Lỗ chốt tầng' }).locator('select').selectOption('ROW_32');
  const qa = await idOf(page, 'TủQA01');
  await expect.poll(() => pins(page, qa)).toBe('ROW_32');
  await expect(dlg.locator('.struct-row', { hasText: 'Lỗ chốt tầng' }).locator('select')).toHaveValue('ROW_32');
  // Lưu chuẩn xưởng.
  await dlg.getByRole('button', { name: 'Lưu chuẩn' }).click();
  await page.locator('.prompt input, .modal input').first().fill('E2E Xưởng');
  await page.keyboard.press('Enter');
  const b3 = await idOf(page, 'BếpDưới03');
  await api(page, { cmd: 'apply_group_preset', ids: [b3], group: 'all', name: 'E2E Xưởng' });
  expect(await pins(page, b3)).toBe('ROW_32');
  await api(page, { cmd: 'undo' });
  expect(await pins(page, b3)).toBe('AT_SHELF');
  await api(page, { cmd: 'delete_group_preset', group: 'all', name: 'E2E Xưởng' });
});

test('TC-03.2 tay nắm push-open, chân nhựa, liên kết cam → báo giá đếm đúng', async ({ page }) => {
  await freshSample(page);
  const b1 = await idOf(page, 'BếpDưới01');
  const set = (name: string, value: string) => api(page, { cmd: 'set_parameter', id: b1, name, value });
  await set('s_handle_type', 'PUSH_OPEN');
  await set('plinth_height', '100');
  await set('base_type', 'LEGS');
  await set('s_joint_type', 'CAM_DOWEL');
  const c = await api(page, { cmd: 'get_costing' });
  const fit = (n: string) => c.fittings.find((l: any) => l.name.startsWith(n))?.qty ?? 0;
  expect(fit('Nhấn mở')).toBeGreaterThan(0);
  expect(fit('Chân nhựa')).toBe(6);
  expect(fit('Cam')).toBeGreaterThanOrEqual(4);
});

test('TC-03.3 dán cạnh nhóm cánh ABS 2 mm → kích thước cắt trừ 4 mm', async ({ page }) => {
  await freshSample(page);
  const b1 = await idOf(page, 'BếpDưới01');
  await api(page, { cmd: 'set_parameter', id: b1, name: 'edge_g_front_mode', value: 'ALL' });
  await api(page, { cmd: 'set_parameter', id: b1, name: 'edge_g_front_code', value: 'ABS-2' });
  const c = await api(page, { cmd: 'get_costing' });
  const door = c.cut_list.find((r: any) => r.cabinet === 'BếpDưới01' && r.name.startsWith('Cửa'));
  expect(door.length - door.cut_length).toBeCloseTo(4, 3);
});
