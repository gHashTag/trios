import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        'service-worker': resolve(__dirname, 'src/background/service-worker.ts'),
        'content/claude-injector': resolve(__dirname, 'src/content/claude-injector.ts'),
        'content/github-injector': resolve(__dirname, 'src/content/github-injector.ts'),
        'content/cursor-injector': resolve(__dirname, 'src/content/cursor-injector.ts'),
        popup: resolve(__dirname, 'src/popup/index.html'),
      },
      output: {
        entryFileNames: (chunkInfo) => {
          if (chunkInfo.name === 'popup') {
            return 'assets/[name].js';
          }
          return '[name].js';
        },
      },
    },
  },
});
