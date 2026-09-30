// TC-05 Báo cáo: báo giá mét dài / m², VAT, danh sách cắt gộp, CSV (D11, D12).
import { expect, test } from '@playwright/test';
import { api, freshSample } from './helpers';

test('TC-05.1 báo giá theo phòng, đổi VAT cập nhật tổng', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Báo cáo' }).last().click();
  await page.getByRole('button', { name: 'Báo giá' }).click();
  await expect(page.locator('.quote')).toContainText('Bếp');
  await expect(page.locator('.quote')).toContainText('PN1');
  const t0 = (await api(page, { cmd: 'get_costing' })).quote.total;
  const vat = page.locator('.quote-settings label', { hasText: 'VAT' }).locator('input');
  await vat.fill('10');
  await vat.press('Enter');
  await expect.poll(async () => (await api(page, { cmd: 'get_costing' })).quote.total).toBeGreaterThan(t0);
  await api(page, { cmd: 'undo' });
});

test('TC-05.2 danh sách cắt gộp ít dòng hơn, có mã tấm, xuất CSV', async ({ page }) => {
  await freshSample(page);
  const c = await api(page, { cmd: 'get_costing' });
  expect(c.cut_groups.length).toBeLessThan(c.cut_list.length);
  expect(c.cut_list.every((r: any) => r.code.length > 0)).toBe(true);
  await page.getByRole('button', { name: 'Báo cáo' }).last().click();
  await page.getByRole('button', { name: 'Danh sách cắt' }).click();
  await page.getByText('Gộp tấm giống nhau').click();
  await expect(page.getByText(/dòng · \d+ tấm/)).toBeVisible();
  const [dl] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: /Excel \/ CSV/ }).click()]);
  expect(dl.suggestedFilename()).toBe('danh-sach-cat-gop.csv');
});
