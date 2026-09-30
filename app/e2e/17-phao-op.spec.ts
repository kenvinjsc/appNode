// TC-03.9 Phào nóc 3 mặt + ốp hông trên tab "Phào & ốp", phào trước dài theo rộng tủ.
import { expect, test } from '@playwright/test';
import { api, countNamed, flatten, freshSample, idOf, menu, tree } from './helpers';

test('TC-03.9 phào nóc 3 mặt, ốp hông trái; đổi rộng → phào trước dài theo', async ({ page }) => {
  await freshSample(page);
  await menu(page, 'TủQA01', 'Thuộc tính kết cấu');
  const dlg = page.locator('.struct-dlg');
  await dlg.getByRole('button', { name: 'Phào & ốp' }).click();
  await dlg.locator('.struct-row', { hasText: 'Mặt có phào' }).locator('select').selectOption('3_SIDES');
  const cab = await idOf(page, 'TủQA01');
  await expect.poll(() => countNamed(page, 'PhàoNóc', cab)).toBe(3);
  await dlg.locator('.struct-row', { hasText: 'Ốp hông trái' }).locator('input').click();
  await expect.poll(() => countNamed(page, 'ỐpHôngTrái', cab)).toBe(1);
  const frontLen = async () => {
    const node = flatten(await tree(page)).find((n) => n.id === cab)!;
    const f = flatten(node.children).find((n) => n.name.startsWith('PhàoNócTrước'))!;
    const m = await api(page, { cmd: 'get_manufacturing', id: f.id });
    return Math.max(m.width, m.height);
  };
  await api(page, { cmd: 'set_parameter', id: cab, name: 'width', value: '1500' });
  const a = await frontLen();
  await api(page, { cmd: 'set_parameter', id: cab, name: 'width', value: '1600' });
  await expect.poll(frontLen).toBeCloseTo(a + 100, 0);
  await api(page, { cmd: 'undo' });
  await api(page, { cmd: 'undo' });
  await api(page, { cmd: 'undo' });
  await expect.poll(() => countNamed(page, 'ỐpHôngTrái', cab)).toBe(0);
});
