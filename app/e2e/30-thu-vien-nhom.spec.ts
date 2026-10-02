// TC-12.6 Thư viện nhóm (D31): thêm nguồn thư mục chung ở Cài đặt, đẩy chuẩn xưởng lên nhóm, bỏ nguồn.
import { expect, test } from '@playwright/test';
import { api, freshSample, idOf, menu } from './helpers';

const DIR = '/tmp/aic-e2e-team';

test('TC-12.6 thư viện nhóm: thêm nguồn, đẩy chuẩn "E2E Nhóm" lên, nguồn đếm 1 mục', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Cài đặt' }).click();
  await page.getByText('Thư viện nhóm (thư mục chung)').click();
  const t = page.locator('.team-lib');
  await t.locator('.struct-row', { hasText: 'Tên nguồn' }).locator('input').fill('E2E Nhóm');
  await t.locator('.struct-row', { hasText: 'Thư mục chung' }).locator('input').fill(DIR);
  await t.getByRole('button', { name: 'Thêm nguồn' }).click();
  await expect(t.locator('.tpl-item', { hasText: 'E2E Nhóm' })).toBeVisible();
  // Lưu chuẩn trong Thuộc tính kết cấu rồi đẩy lên nhóm.
  const cab = await idOf(page, 'TủQA01');
  await api(page, { cmd: 'save_group_preset', cabinet: cab, group: 'all', name: 'E2E chuẩn nhóm' });
  await menu(page, 'TủQA01', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.locator('.struct-std', { hasText: 'Chuẩn xưởng' }).locator('select').selectOption('E2E chuẩn nhóm');
  await dlg.getByRole('button', { name: 'Đẩy lên nhóm' }).click();
  await expect(page.locator('.toast', { hasText: 'Đã đẩy' })).toBeVisible();
  const info = await api(page, { cmd: 'get_library_sources' });
  expect(info.sources.find((s: any) => s.name === 'E2E Nhóm').items).toBeGreaterThanOrEqual(1);
  // Dọn: bỏ nguồn, xóa chuẩn.
  await api(page, { cmd: 'set_library_sources', sources: info.sources.filter((s: any) => s.name !== 'E2E Nhóm').map(({ name, path, readonly }: any) => ({ name, path, readonly })) });
  await api(page, { cmd: 'delete_group_preset', group: 'all', name: 'E2E chuẩn nhóm' });
});
