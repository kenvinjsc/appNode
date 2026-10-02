// TC-02.13 Luật theo phòng (D33): tủ dưới tạo trong phòng WC1 → MFC lõi xanh, chân nhựa, hậu khoét ống.
import { expect, test } from '@playwright/test';
import { api, flatten, freshSample, tree } from './helpers';

test('TC-02.13 tủ trong WC1: chống ẩm, chân nhựa, khoét ống', async ({ page }) => {
  await freshSample(page);
  // Khung → chọn phòng WC1: hiện Loại phòng tự đoán WC.
  await page.getByRole('button', { name: 'Khung', exact: true }).click();
  await page.locator('.room-chips .chip', { hasText: 'WC1' }).click();
  const row = page.locator('.form-row', { hasText: 'Loại phòng' });
  await expect(row.locator('select option').first()).toContainText('WC');
  const r = await api(page, { cmd: 'create_cabinet', kind: 'BASE', room: 'WC1', overrides: { width: 800 } });
  expect(r.room_rules.length).toBeGreaterThanOrEqual(3);
  const s = await api(page, { cmd: 'get_structure', cabinet: r.id });
  expect(s.back_cutouts.length).toBe(1);
  const base = s.tabs.find((t: any) => t.key === 'plinth').fields.find((f: any) => f.key === 'base_type').value;
  expect(base).toBe('LEGS');
  await page.reload();
  const cab = flatten(await tree(page)).find((n) => n.id === r.id)!;
  const side = flatten(cab.children).find((n) => n.name === 'HồiTrái')!;
  const m = await api(page, { cmd: 'get_manufacturing', id: side.id });
  expect(m.material_id).toBe('MFCMR18-WHITE');
});
