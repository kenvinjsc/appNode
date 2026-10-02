// TC-11.5 / 11.6 Vật liệu Việt Nam và bộ vật liệu (áp cho tủ / cả phòng, một undo).
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, idOf, menu, tree } from './helpers';

const sideMaterial = async (page: import('@playwright/test').Page, cabName: string) => {
  const cab = flatten(await tree(page)).find((n) => n.name === cabName)!;
  const side = flatten(cab.children).find((n) => n.name === 'HồiTrái')!;
  const p = await api(page, { cmd: 'get_properties', id: side.id });
  const g = p.groups.find((x: any) => x.key === 'material');
  return String(g.fields.find((f: any) => f.key === 'material').value);
};

test('TC-11.5 có vật liệu MFC lõi xanh, Acrylic trong thư viện vật liệu', async ({ page }) => {
  await freshSample(page);
  const mats = await api(page, { cmd: 'get_materials' });
  const list = (mats.materials ?? mats) as { id: string; kind: string; sheet_width_mm: number; sheet_height_mm: number }[];
  const mr = list.find((m) => m.id === 'MFCMR18-WHITE')!;
  expect(mr.kind).toBe('MFC_MR');
  expect([mr.sheet_width_mm, mr.sheet_height_mm]).toEqual([2440, 1830]);
  expect(list.some((m) => m.kind === 'ACRYLIC')).toBe(true);
});

test('TC-11.6 áp bộ "Bếp chống ẩm" cho cả phòng Bếp từ bảng kết cấu, một undo', async ({ page }) => {
  await freshSample(page);
  await menu(page, 'BếpDưới01', 'Thuộc tính kết cấu');
  const row = page.locator('.struct-std', { hasText: 'Bộ vật liệu' });
  await row.locator('select').selectOption({ label: 'Bếp chống ẩm (MFC lõi xanh + Acrylic)' });
  await row.getByRole('button', { name: 'Cả phòng' }).click();
  await expect(page.locator('.toast').first()).toContainText('cho 4 tủ');
  for (const n of ['BếpDưới01', 'BếpDưới03', 'BếpTrên01']) expect(await sideMaterial(page, n)).toContain('MFCMR18-WHITE');
  expect(await sideMaterial(page, 'TủQA01')).not.toContain('MFCMR18-WHITE');
  await page.locator('.struct-dlg .icon-btn[title="Đóng"]').click();
  await page.mouse.move(600, 700);
  await page.keyboard.press('Control+z');
  await expect.poll(() => sideMaterial(page, 'BếpDưới03')).not.toContain('MFCMR18-WHITE');
  void idOf;
});
