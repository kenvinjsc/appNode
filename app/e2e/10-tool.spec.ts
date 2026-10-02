// TC-08.6 Tool gia công theo tham số: khấu góc cho nhiều tấm = một undo, giữ đúng góc khi tủ đổi sâu.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, tree } from './helpers';

const notchX = async (page: import('@playwright/test').Page) => {
  const cab = flatten(await tree(page)).find((n) => n.name === 'BếpDưới01')!;
  const side = flatten(cab.children).find((n) => n.name === 'HồiTrái')!;
  const flat = await api(page, { cmd: 'get_manufacturing', id: side.id });
  const f = flat.features.find((x: any) => x.feature.type === 'POCKET' && x.feature.width === 100);
  return f ? { x: f.feature.x, w: flat.width } : null;
};

test('TC-08.6 khấu góc TR trên 2 hồi từ cột Tool, một undo, theo sâu tủ', async ({ page }) => {
  await freshSample(page);
  const cab = flatten(await tree(page)).find((n) => n.name === 'BếpDưới01')!;
  const sides = flatten(cab.children).filter((n) => n.name === 'HồiTrái' || n.name === 'HồiPhải');
  // Mở tool trên giao diện (form hiển thị), áp cho 2 hồi qua core: một request, một undo.
  await page.getByRole('button', { name: 'Tool', exact: true }).click();
  await page.getByText('Khấu góc tủ').first().click();
  await expect(page.getByRole('button', { name: 'Khấu góc', exact: true })).toBeVisible();
  await api(page, { cmd: 'tool_feature', ids: sides.map((s) => s.id), tool: '03. Khấu góc tủ', feature: { type: 'NOTCH', corner: 'TR', width: 100, depth: 100 } });
  await expect.poll(async () => (await notchX(page)) !== null).toBe(true);
  const a = (await notchX(page))!;
  expect(a.x).toBeCloseTo(a.w - 100, 3);
  await api(page, { cmd: 'set_parameter', id: cab.id, name: 'depth', value: '640' });
  const b = (await notchX(page))!;
  expect(b.w - a.w).toBeCloseTo(40, 3);
  expect(b.x).toBeCloseTo(b.w - 100, 3);
  await api(page, { cmd: 'undo' });
  await api(page, { cmd: 'undo' });
  expect(await notchX(page)).toBeNull();
  await api(page, { cmd: 'redo' });
  expect(await notchX(page)).not.toBeNull();
});
