import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
import { renderRoadmap } from './src/logic/roadmapMarkup';

export default defineConfig({
  base: './',
  plugins: [
    {
      name: 'static-roadmap',
      transformIndexHtml(html, context) {
        return context.filename.endsWith('/roadmap.html') ? renderRoadmap(html) : html;
      },
    },
  ],
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      input: {
        teaser: fileURLToPath(new URL('./index.html', import.meta.url)),
        roadmap: fileURLToPath(new URL('./roadmap.html', import.meta.url)),
        dossier: fileURLToPath(new URL('./dossier.html', import.meta.url)),
      },
    },
  },
});
