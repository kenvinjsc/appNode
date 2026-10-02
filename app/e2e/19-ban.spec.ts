// TC-02.8 Bàn học (D21): chèn từ menu, hộc 3 ngăn, kệ trên 2 tầng; đổi đỡ trái sang chân sắt.
import { expect, test } from '@playwright/test';
import { api, countNamed, freshSample, idOf } from './helpers';

test('TC-02.8 bàn học 1200: hộc phải 3 ngăn, kệ trên 2 tầng, khoét dây', async ({ page }) => {
  await freshSample(page);
  await page.getByRole('button', { name: 'Tủ ▾' }).click();
  await page.getByText('Bàn học 1200').click();
  await expect.poll(() => idOf(page, 'Bàn').catch(() => 0)).toBeGreaterThan(0);
  const desk = await idOf(page, 'Bàn');
  expect(await countNamed(page, 'KệTrên', desk)).toBe(2);
  expect(await countNamed(page, 'HồiHộcPhải', desk)).toBe(2);
  await api(page, { cmd: 'set_parameter', id: desk, name: 'desk_support_left', value: 'LEG' });
  await expect.poll(() => countNamed(page, 'ChânTấmTrái', desk)).toBe(0);
  const s = await api(page, { cmd: 'get_structure', cabinet: desk });
  expect(s.tabs[0].key).toBe('desk');
});
