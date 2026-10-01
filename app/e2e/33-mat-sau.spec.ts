// TC-04.x 2D Mặt sau: ghim khoang sau của bàn đảo và dựng cánh quay ra sau.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, pick, tree } from './helpers';

test('TC-04.9 bàn đảo: 2D Mặt sau → chuột phải khoang sau → Cánh đôi → cánh ở mặt sau', async ({ page }) => {
  await freshSample(page);
  const r = await api(page, { cmd: 'create_furniture', kind: 'ISLAND', width: 1600, height: 900, depth: 900, room: 'Bếp' });
  await page.reload();
  const name = (await api(page, { cmd: 'get_properties', id: r.id })).name as string;
  await pick(page, name);
  await page.locator('.panel.drawing select').first().selectOption('back');
  const zones = page.locator('.d2e-zone');
  await expect(zones).toHaveCount(2);
  // Khoang gần mặt sau nằm trên cùng (vẽ sau cùng).
  await zones.last().click({ button: 'right', force: true });
  await page.getByText('Cánh đôi (phủ bì)').click();
  const doors = async () => flatten(flatten(await tree(page)).find((n) => n.id === r.id)!.children).filter((n) => n.name.startsWith('CửaĐôi'));
  await expect.poll(async () => (await doors()).length).toBe(2);
  for (const d of await doors()) {
    const b = await api(page, { cmd: 'get_bounds', ids: [d.id] });
    expect(b.max[2] - (await api(page, { cmd: 'get_bounds', ids: [r.id] })).min[2]).toBeLessThan(400);
  }
});
