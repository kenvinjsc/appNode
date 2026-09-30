import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// In the browser the UI talks to the Rust core through the dev server
// (crates/aic-dev-server, POST /api). Inside Tauri it uses IPC instead.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: { '/api': 'http://127.0.0.1:8787' },
  },
  build: { chunkSizeWarningLimit: 1500 },
  test: { environment: 'node', include: ['src/**/*.test.{ts,tsx}'] },
} as never);
