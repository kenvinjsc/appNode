// TC-08.8 Nối vân (D32): chọn 2 cánh → chuột phải → Nối vân → xếp tấm: 2 cánh liền nhau, cùng tấm ván, cùng hướng.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, menu, pick, tree } from './helpers';

test('TC-08.8 nối vân 2 cánh TủQA01', async ({ page }) => {
  await freshSample(page);
  await pick(page, 'TủQA01');
  await page.locator('.tree-row', { hasText: 'TủQA01' }).locator('.twisty').click();
  await pick(page, 'CửaĐôi_01');
  await pick(page, 'CửaĐôi_02', true);
  await menu(page, 'CửaĐôi_02', /Nối vân 2 tấm/);
  await page.locator('.prompt input, .modal input').first().fill('Cánh QA');
  await page.keyboard.press('Enter');
  await expect(page.locator('.toast', { hasText: 'Đã nối vân 2 tấm' })).toBeVisible();
  const qa = flatten(await tree(page)).find((n) => n.name === 'TủQA01')!;
  const ids = flatten(qa.children).filter((n) => n.name === 'CửaĐôi_01' || n.name === 'CửaĐôi_02').map((n) => n.id);
  const flat = await api(page, { cmd: 'get_manufacturing', id: ids[0] });
  const n = await api(page, { cmd: 'run_nesting', material: flat.material_id });
  const pl = n.jobs[0].result.placements.filter((p: any) => ids.includes(p.part_id));
  expect(pl.length).toBe(2);
  expect(pl[0].sheet_id).toBe(pl[1].sheet_id);
  expect(pl[0].rotation_deg).toBe(pl[1].rotation_deg);
});
