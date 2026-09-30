// TC-02.6 Mẫu bếp dựng sẵn (D30): chèn từ menu Tủ ▾, đúng kích thước / vật liệu, một undo.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, pick } from './helpers';

test('TC-02.6 chèn "Tủ lò 600 kịch trần" cạnh BếpDưới03', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'BếpDưới03');
  await page.getByText('Tủ ▾').click();
  await expect(page.getByText('Mẫu bếp dựng sẵn')).toBeVisible();
  await page.getByText('Tủ lò 600 kịch trần').click();
  await expect.poll(() => countNamed(page, 'TủLò')).toBe(1);
  const tree = await api(page, { cmd: 'get_scene_tree' });
  const flat = (n: any[]): any[] => n.flatMap((x) => [x, ...flat(x.children ?? [])]);
  const cab = flat(tree.roots).find((n) => n.name.startsWith('TủLò'));
  const z = await api(page, { cmd: 'get_zones', cabinet: cab.id });
  expect(z.size).toEqual([600, 2300, 580]);
  expect(z.room).toBe('Bếp');
  expect(z.floor).toBe('Tầng 1');
  await page.mouse.move(600, 700);
  await page.keyboard.press('Control+z');
  await expect.poll(() => countNamed(page, 'TủLò')).toBe(0);
});
