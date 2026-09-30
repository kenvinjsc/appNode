// TC-03.8 Khoét hậu (ống nước giữa tủ, giữ chỗ khi đổi rộng), hậu ốp bắt vít.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, idOf, menu, tree } from './helpers';

/** Tâm lỗ khoét trên hậu, tính từ giữa tấm (0 = chính giữa); null khi không có lỗ. */
async function holeOff(page: import('@playwright/test').Page, cab: number): Promise<number | null> {
  const node = flatten(await tree(page)).find((n) => n.id === cab)!;
  const back = flatten(node.children).find((n) => n.name === 'Hậu')!;
  const m = await api(page, { cmd: 'get_manufacturing', id: back.id });
  if (!m.inner.length) return null;
  const xs = m.inner[0].points.map((p: any) => p.x);
  return (Math.min(...xs) + Math.max(...xs)) / 2 - m.width / 2;
}

test('TC-03.8 khoét ống Ø60 giữa hậu, đổi rộng lỗ vẫn giữa; hậu ốp có vít', async ({ page }) => {
  await freshSample(page);
  await menu(page, 'BếpDưới03', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Hậu', exact: true }).click();
  await dlg.getByRole('button', { name: '+ Ống nước Ø60' }).click();
  await expect(dlg.locator('.cut-row:not(.cut-head)')).toHaveCount(1);
  const cab = await idOf(page, 'BếpDưới03');
  await expect.poll(() => holeOff(page, cab)).toBeCloseTo(0, 1);
  // Đổi rộng +200 → lỗ vẫn giữa tủ (hậu là tấm liền, giữa tấm = giữa tủ).
  await api(page, { cmd: 'set_parameter', id: cab, name: 'width', value: '600' });
  await expect.poll(() => holeOff(page, cab)).toBeCloseTo(0, 1);
  const m = await api(page, { cmd: 'get_manufacturing', id: flatten((flatten(await tree(page)).find((n) => n.id === cab))!.children).find((n) => n.name === 'Hậu')!.id });
  expect(m.width).toBeGreaterThan(500);
  // Hậu ốp bắt vít → báo giá có vít bắt hậu.
  await dlg.locator('.struct-row', { hasText: 'Hậu ốp bắt vít' }).locator('input').click();
  await expect
    .poll(async () => JSON.stringify(await api(page, { cmd: 'get_costing' })).includes('Vít bắt hậu'))
    .toBe(true);
  // Bỏ lỗ.
  await dlg.locator('.cut-row button').click();
  await expect.poll(() => holeOff(page, cab)).toBeNull();
});
