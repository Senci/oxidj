import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';
import { basename } from 'node:path';
import { renderRoadmap } from './src/logic/roadmapMarkup';
import { renderSite } from './src/logic/siteMarkup';
export default defineConfig({
  base: './',
  plugins: [
    {
      name: 'static-public-pages',
      transformIndexHtml(html, context) {
        return renderSite(renderRoadmap(html), basename(context.filename));
      },
    },
  ],
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      input: {
        home: fileURLToPath(new URL('./index.html', import.meta.url)),
        roadmap: fileURLToPath(new URL('./roadmap.html', import.meta.url)),
        dossier: fileURLToPath(new URL('./dossier.html', import.meta.url)),
        missing: fileURLToPath(new URL('./404.html', import.meta.url)),
      },
    },
  },
});
