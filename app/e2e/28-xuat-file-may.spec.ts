// TC-08.7 Xuất file máy (D26): chọn hồi → Chỉnh tấm → Gia công → Xuất MPR / DXF → tải file.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, pick, tree } from './helpers';

test('TC-08.7 xuất DXF hồi có cam + chốt: layer DRILL_15_12.5 và HDRILL_8_*', async ({ page }) => {
  await freshSample(page);
  const cab = flatten(await tree(page)).find((n) => n.name === 'BếpDưới01')!;
  await api(page, { cmd: 'set_parameter', id: cab.id, name: 's_joint_type', value: 'CAM_DOWEL' });
  await page.reload();
  const node = flatten(await tree(page)).find((n) => n.id === cab.id)!;
  // Chọn hồi trên cây.
  await pick(page, 'BếpDưới01');
  await pick(page, 'HồiTrái');
  await page.getByRole('button', { name: 'Chỉnh tấm' }).click();
  await page.locator('.tabs button', { hasText: 'Gia công' }).click();
  await expect(page.locator('.machine-export')).toBeVisible();
  const [dl] = await Promise.all([page.waitForEvent('download'), page.locator('.machine-export button').click()]);
  expect(dl.suggestedFilename()).toMatch(/\.dxf$/);
  // Nội dung: cả tủ (cam ở hồi, chốt khoan cạnh ở đáy / giằng).
  const ids = flatten(node.children).filter((n) => n.kind === 'PANEL').map((n) => n.id);
  const r = await api(page, { cmd: 'export_machine', ids, format: 'DXF' });
  const all = r.files.map((f: any) => f.content).join('');
  expect(all).toContain('DRILL_15_12.5');
  expect(all).toMatch(/HDRILL_8_\d+/);
});
