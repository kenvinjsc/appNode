// Tiện ích chung cho kịch bản E2E: gọi thẳng core qua /api để chuẩn bị / kiểm tra dữ liệu,
// còn thao tác người dùng đi qua giao diện thật.
import { expect, type Page } from '@playwright/test';

export interface TreeNode {
  id: number;
  name: string;
  kind: string;
  children: TreeNode[];
}

/** Gửi một request tới core, trả `result` (ném lỗi nếu core báo lỗi). */
export async function api<T = any>(page: Page, body: Record<string, unknown>): Promise<T> {
  const r = await page.request.post('/api', { data: body });
  const j = await r.json();
  if (!j.ok) throw new Error(`${body.cmd}: ${JSON.stringify(j.error)}`);
  return j.result as T;
}

/** Dự án trống → trang nạp lại → app tự dựng Dự án mẫu (TủQA01, BếpDưới01–03, BếpTrên01). */
export async function freshSample(page: Page) {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto('/');
  await api(page, { cmd: 'create_project', name: 'e2e' });
  await page.reload();
  await expect(page.getByText('BếpTrên01', { exact: true }).first()).toBeVisible();
  await page.waitForTimeout(500);
  return errors;
}

export async function tree(page: Page): Promise<TreeNode[]> {
  const t = await api<{ roots: TreeNode[] }>(page, { cmd: 'get_scene_tree' });
  return t.roots;
}

export function flatten(nodes: TreeNode[]): TreeNode[] {
  return nodes.flatMap((n) => [n, ...flatten(n.children ?? [])]);
}

export async function idOf(page: Page, name: string): Promise<number> {
  const n = flatten(await tree(page)).find((x) => x.name === name);
  if (!n) throw new Error(`không thấy "${name}" trong cây`);
  return n.id;
}

export async function countNamed(page: Page, prefix: string, under?: number): Promise<number> {
  let nodes = flatten(await tree(page));
  if (under !== undefined) nodes = flatten(nodes.filter((n) => n.id === under));
  return nodes.filter((n) => n.name.startsWith(prefix)).length;
}

/** Bấm tên trong cây đối tượng (Ctrl để chọn thêm). */
export async function pick(page: Page, name: string, add = false) {
  await page.getByText(name, { exact: true }).first().click({ modifiers: add ? ['Control'] : [] });
  await page.waitForTimeout(250);
}

/** Chuột phải tên trong cây rồi chọn mục menu. */
export async function menu(page: Page, name: string, item: string | RegExp) {
  await page.getByText(name, { exact: true }).first().click({ button: 'right' });
  await page.getByText(item).first().click();
  await page.waitForTimeout(400);
}

/** Hoàn tác qua giao diện (Ctrl+Z) để UI nhận sự kiện và vẽ lại như người dùng thật. */
export async function undo(page: Page) {
  await page.mouse.move(600, 700);
  await page.keyboard.press('Control+z');
  await page.waitForTimeout(400);
}
