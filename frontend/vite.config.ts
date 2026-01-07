import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tsconfigPaths from 'vite-tsconfig-paths';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tsconfigPaths(), tailwindcss()],
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
      '@tauri-apps/plugin-http',
      '@radix-ui/react-tabs',
      '@radix-ui/react-select',
      '@radix-ui/react-checkbox',
      '@radix-ui/react-slider',
      '@radix-ui/react-scroll-area',
      '@radix-ui/react-label',
      'lucide-react',
      '@heroicons/react/24/outline',
      '@heroicons/react/24/solid',
      'clsx',
      'class-variance-authority',
      'fuse.js',
    ],
  },
});
