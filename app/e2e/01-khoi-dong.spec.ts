// TC-01 Khởi động, dự án mẫu, tầng / phòng, undo / redo, lưu / mở dự án.
import { expect, test } from '@playwright/test';
import { api, countNamed, flatten, freshSample, pick, tree } from './helpers';

test('TC-01.1 mở app: dự án mẫu đủ tủ, không lỗi trang', async ({ page }) => {
  const errors = await freshSample(page);
  const names = flatten(await tree(page)).map((n) => n.name);
  for (const n of ['TủQA01', 'BếpDưới01', 'BếpDưới02', 'BếpDưới03', 'BếpTrên01']) expect(names).toContain(n);
  await expect(page.locator('.statusbar')).toContainText('Sẵn sàng');
  expect(errors).toEqual([]);
});

test('TC-01.2 lọc theo tầng / phòng', async ({ page }) => {
  await freshSample(page);
  await page.getByText(/^Tầng 2/).first().click();
  await expect(page.getByText('TủQA01', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('BếpDưới01', { exact: true })).toHaveCount(0);
});

test('TC-01.3 xóa tủ → undo → redo', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  await page.keyboard.press('Delete');
  await expect.poll(() => countNamed(page, 'BếpDưới03')).toBe(0);
  await page.keyboard.press('Control+z');
  await expect.poll(() => countNamed(page, 'BếpDưới03')).toBe(1);
  await page.keyboard.press('Control+y');
  await expect.poll(() => countNamed(page, 'BếpDưới03')).toBe(0);
});

test('TC-01.4 lưu → mở lại dự án giữ nguyên số tấm', async ({ page }) => {
  await freshSample(page);
  const before = flatten(await tree(page)).length;
  const project = await api(page, { cmd: 'save_project' });
  await api(page, { cmd: 'create_project', name: 'trống' });
  await api(page, { cmd: 'load_project', project });
  expect(flatten(await tree(page)).length).toBe(before);
});
