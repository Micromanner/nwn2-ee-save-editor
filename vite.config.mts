import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    tsconfigPaths: true,
  },
  server: {
    port: 24314,
    strictPort: true,
  },
  build: {
    outDir: 'out',
    emptyOutDir: true,
  },
  optimizeDeps: {
    include: [
      'react',
      'react-dom',
      'react-router-dom',
      'react-intl',
      'react-window',
      '@tauri-apps/api',
      '@tauri-apps/plugin-dialog',
    ],
  },
});
