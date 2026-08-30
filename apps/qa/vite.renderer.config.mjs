import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const qaRoot = fileURLToPath(new URL('.', import.meta.url));
const webSource = process.env.AUTODRILL_QA_WEB_SOURCE
  ? resolve(process.env.AUTODRILL_QA_WEB_SOURCE)
  : fileURLToPath(new URL('../web/src', import.meta.url));

export default defineConfig({
  root: fileURLToPath(new URL('./renderer', import.meta.url)),
  base: '/renderer/',
  plugins: [react()],
  resolve: { alias: { '@': webSource } },
  build: {
    outDir: fileURLToPath(new URL('./public/renderer', import.meta.url)),
    emptyOutDir: true,
    rollupOptions: {
      output: {
        entryFileNames: 'renderer.js',
        chunkFileNames: 'chunk-[hash].js',
        assetFileNames: (asset) => asset.names?.some((name) => name.endsWith('.css')) ? 'renderer.css' : 'asset-[hash][extname]',
      },
    },
  },
  cacheDir: `${qaRoot}/.vite-renderer`,
});
