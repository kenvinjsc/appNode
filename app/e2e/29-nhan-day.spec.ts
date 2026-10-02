// TC-02.12 Nhân dãy tủ (D28): chuột phải tủ → Nhân dãy tủ… → công thức 400,600,800 → 3 tủ đúng thứ tự, một undo.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, idOf, menu, tree } from './helpers';

test('TC-02.12 nhân dãy BếpTrên01 theo 400,600,800', async ({ page }) => {
  await freshSample(page);
  const before = flatten(await tree(page)).filter((n) => n.kind === 'CABINET').length;
  await menu(page, 'BếpTrên01', 'Nhân dãy tủ…');
  const d = page.locator('.array-dlg');
  await d.locator('.struct-row', { hasText: 'Kích thước từng tủ' }).locator('input').fill('400,600,800');
  await d.getByRole('button', { name: 'Nhân dãy' }).click();
  await expect(d).toHaveCount(0);
  await expect.poll(async () => flatten(await tree(page)).filter((n) => n.kind === 'CABINET').length).toBe(before + 3);
  const cabs = flatten(await tree(page)).filter((n) => n.kind === 'CABINET' && n.name.startsWith('BếpTrên'));
  const src = await api(page, { cmd: 'get_bounds', ids: [await idOf(page, 'BếpTrên01')] });
  const rows = [];
  for (const c of cabs) {
    const b = await api(page, { cmd: 'get_bounds', ids: [c.id] });
    if (b.min[0] >= src.max[0] - 1) rows.push([b.min[0], Math.round(b.max[0] - b.min[0])]);
  }
  rows.sort((a, b) => a[0] - b[0]);
  expect(rows.map((r) => r[1]).slice(0, 3)).toEqual([400, 600, 800]);
  await page.keyboard.press('Control+z');
  await expect.poll(async () => flatten(await tree(page)).filter((n) => n.kind === 'CABINET').length).toBe(before);
});
