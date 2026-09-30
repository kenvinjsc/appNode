// Kịch bản test E2E (docs/KICH-BAN-TEST.md). Chạy: `npm run e2e` (tự bật core + Vite nếu chưa chạy).
// Core giữ một dự án chung nên test chạy tuần tự (workers: 1); mỗi test tự nạp lại dự án mẫu.
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: { timeout: 10_000 },
  workers: 1,
  fullyParallel: false,
  reporter: [['list'], ['html', { open: 'never', outputFolder: 'e2e-report' }]],
  use: {
    baseURL: 'http://127.0.0.1:5173',
    viewport: { width: 1680, height: 945 },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    launchOptions: {
      executablePath: process.env.PW_CHROMIUM || undefined,
      args: ['--use-gl=swiftshader', '--enable-webgl', '--ignore-gpu-blocklist', '--enable-unsafe-swiftshader'],
    },
  },
  webServer: [
    {
      command: 'cargo run -p aic-dev-server --release --manifest-path ../Cargo.toml',
      url: 'http://127.0.0.1:8787/api',
      reuseExistingServer: true,
      timeout: 900_000,
    },
    {
      command: 'npx vite --host 127.0.0.1 --port 5173 --strictPort',
      url: 'http://127.0.0.1:5173',
      reuseExistingServer: true,
      timeout: 120_000,
    },
  ],
});
