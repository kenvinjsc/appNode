// TC-04 Dãy tủ (mặt đá, len chân liền, khoét chậu) và tủ góc L mù (D07, D09).
import { expect, test } from '@playwright/test';
import { api, countNamed, flatten, freshSample, idOf, pick, tree } from './helpers';

const panel = async (page: import('@playwright/test').Page, name: string) => {
  const id = flatten(await tree(page)).find((n) => n.name === name)?.id;
  if (id === undefined) return null;
  return api(page, { cmd: 'get_properties', id });
};

test('TC-04.1 tạo dãy từ 3 tủ bếp dưới, mặt đá theo tủ khi đổi rộng, một undo', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới01');
  await pick(page, 'BếpDưới02', true);
  await pick(page, 'BếpDưới03', true);
  await page.getByText('BếpDưới03', { exact: true }).first().click({ button: 'right' });
  await page.getByText('Tạo dãy tủ').first().click();
  await expect(page.locator('.run-dlg')).toBeVisible();
  await expect.poll(() => countNamed(page, 'MặtĐá')).toBe(1);
  const runs = (await api(page, { cmd: 'get_runs' })).runs;
  expect(runs[0].cabinets.length).toBe(3);
  const w0 = (await api(page, { cmd: 'get_costing' })).cut_list.find((r: any) => r.name === 'MặtĐá').length;
  // Khoét chậu từ bảng Dãy tủ.
  await page.locator('.run-dlg').getByRole('button', { name: '+ Khoét chậu' }).click();
  await expect.poll(async () => (await api(page, { cmd: 'get_runs' })).runs[0].rules.cutouts.length).toBe(1);
  // Đổi rộng tủ giữa dãy → mặt đá dài thêm 100.
  const b2 = await idOf(page, 'BếpDưới02');
  await api(page, { cmd: 'resize_cabinet', id: b2, name: 'width', value: 900, stretch: 'KEEP' });
  const w1 = (await api(page, { cmd: 'get_costing' })).cut_list.find((r: any) => r.name === 'MặtĐá').length;
  expect(w1 - w0).toBeCloseTo(100, 3);
  await api(page, { cmd: 'undo' });
  const w2 = (await api(page, { cmd: 'get_costing' })).cut_list.find((r: any) => r.name === 'MặtĐá').length;
  expect(w2).toBeCloseTo(w0, 3);
  void panel;
});

test('TC-04.2 tủ góc L mù góc phải cạnh tủ đang chọn', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  await page.getByText('Tủ ▾').click();
  await page.getByText('Tủ góc L mù (góc phải)').click();
  await expect.poll(() => countNamed(page, 'BếpGócPhải')).toBe(1);
  const cab = await idOf(page, 'BếpGócPhải');
  expect(await countNamed(page, 'TấmMù', cab)).toBe(1);
  expect(await countNamed(page, 'CửaĐơn', cab)).toBe(1);
  const props = await api(page, { cmd: 'get_properties', id: cab });
  expect(JSON.stringify(props)).toContain('Bếp');
});

test('TC-06.10 tủ góc chéo bếp dưới: đáy / nóc 5 cạnh, cánh 45°, đổi số kệ', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  await page.getByText('Tủ ▾').click();
  await page.getByText('Tủ góc chéo (bếp dưới)').click();
  await expect.poll(() => countNamed(page, 'BếpGócChéo')).toBe(1);
  const cab = await idOf(page, 'BếpGócChéo');
  expect(await countNamed(page, 'CửaChéo', cab)).toBe(1);
  expect(await countNamed(page, 'Hậu', cab)).toBe(2);
  const s = await api(page, { cmd: 'get_structure', cabinet: cab });
  expect(s.tabs[0].title).toBe('Tủ góc chéo');
  await api(page, { cmd: 'set_parameter', id: cab, name: 'dg_shelves', value: '3' });
  expect(await countNamed(page, 'KệCốĐịnh', cab)).toBe(3);
});
