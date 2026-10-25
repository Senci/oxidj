import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  base: './',
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      input: {
        roadmap: fileURLToPath(new URL('./index.html', import.meta.url)),
        dossier: fileURLToPath(new URL('./dossier.html', import.meta.url)),
      },
    },
  },
});
